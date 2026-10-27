// Anonymous FTP, the route the Bureau of Meteorology gives programs for its
// warnings and outlines (bom.rs): passive mode, binary, a file or a
// listing per request. "ftp://host/folder/file" fetches the file;
// "ftp://host/folder/" lists the folder, and "ftp://host/folder/*.cap.xml"
// only what matches. A finished session is kept for the next request to
// the same host, so a round of checks logs in once; one the server has
// since closed is replaced. FTP bypasses the system's web proxy, as every
// FTP client does.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Mutex;
use std::time::Duration;

use crate::fetch::FetchError;

// Sessions kept between requests, enough for the outline layers fetched
// side by side; the rest are closed.
const KEPT: usize = 6;

#[derive(Default)]
pub struct Ftp {
    idle: Mutex<Vec<Session>>,
}

struct Session {
    host: String,
    control: BufReader<TcpStream>,
}

enum Failure {
    Io(io::Error),
    Refused { code: u16, text: String },
}

impl From<io::Error> for Failure {
    fn from(e: io::Error) -> Failure {
        Failure::Io(e)
    }
}

impl Failure {
    fn into_fetch_error(self, host: &str) -> FetchError {
        let host = host.to_string();
        match self {
            Failure::Io(e)
                if matches!(
                    e.kind(),
                    io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                ) =>
            {
                FetchError::TimedOut { host }
            }
            Failure::Io(e) => FetchError::Unreachable {
                host,
                detail: e.to_string(),
            },
            Failure::Refused { code, text } => FetchError::Status {
                code,
                reason: text,
                host,
            },
        }
    }
}

impl Ftp {
    pub fn get(&self, url: &str, timeout: Duration) -> Result<Vec<u8>, FetchError> {
        let rest = url.strip_prefix("ftp://").unwrap_or(url);
        let (host, path) = rest.split_once('/').unwrap_or((rest, ""));
        let path = format!("/{path}");
        let (folder, last) = path.rsplit_once('/').unwrap_or(("", &path));
        let listing = last.is_empty() || last.contains('*');
        loop {
            let kept = {
                let mut idle = self.idle.lock().unwrap_or_else(|e| e.into_inner());
                idle.iter()
                    .position(|s| s.host == host)
                    .map(|i| idle.swap_remove(i))
            };
            let reused = kept.is_some();
            let mut session = match kept {
                Some(s) => s,
                None => Session::open(host, timeout).map_err(|f| f.into_fetch_error(host))?,
            };
            let answer = if listing {
                session
                    .command(&format!(
                        "CWD {}",
                        if folder.is_empty() { "/" } else { folder }
                    ))
                    .and_then(|_| {
                        session.transfer(&if last.is_empty() {
                            "LIST".to_string()
                        } else {
                            format!("LIST {last}")
                        })
                    })
            } else {
                session.transfer(&format!("RETR {path}"))
            };
            match answer {
                Ok(data) => {
                    let mut idle = self.idle.lock().unwrap_or_else(|e| e.into_inner());
                    if idle.len() < KEPT {
                        idle.push(session);
                    }
                    return Ok(data);
                }
                // A kept session the server closed while it sat idle.
                Err(Failure::Io(_)) | Err(Failure::Refused { code: 421, .. }) if reused => continue,
                Err(f) => return Err(f.into_fetch_error(host)),
            }
        }
    }
}

impl Session {
    fn open(host: &str, timeout: Duration) -> Result<Session, Failure> {
        let mut last = io::Error::new(io::ErrorKind::NotFound, "no address");
        let (name, port) = host
            .rsplit_once(':')
            .and_then(|(name, port)| Some((name, port.parse().ok()?)))
            .unwrap_or((host, 21));
        for address in (name, port).to_socket_addrs()? {
            match TcpStream::connect_timeout(&address, timeout) {
                Ok(stream) => {
                    stream.set_read_timeout(Some(timeout))?;
                    stream.set_write_timeout(Some(timeout))?;
                    let mut session = Session {
                        host: host.to_string(),
                        control: BufReader::new(stream),
                    };
                    session.expect_reply(2)?;
                    let (code, _) = session.send("USER anonymous")?;
                    if code == 331 {
                        session.command("PASS anonymous@")?;
                    } else if code / 100 != 2 {
                        return Err(Failure::Refused {
                            code,
                            text: "anonymous login refused".to_string(),
                        });
                    }
                    session.command("TYPE I")?;
                    return Ok(session);
                }
                Err(e) => last = e,
            }
        }
        Err(Failure::Io(last))
    }

    fn send(&mut self, command: &str) -> Result<(u16, String), Failure> {
        let stream = self.control.get_mut();
        stream.write_all(command.as_bytes())?;
        stream.write_all(b"\r\n")?;
        self.reply()
    }

    // A command that must succeed.
    fn command(&mut self, command: &str) -> Result<String, Failure> {
        let (code, text) = self.send(command)?;
        if code / 100 == 2 || code / 100 == 3 {
            Ok(text)
        } else {
            Err(Failure::Refused { code, text })
        }
    }

    fn expect_reply(&mut self, class: u16) -> Result<String, Failure> {
        let (code, text) = self.reply()?;
        if code / 100 == class {
            Ok(text)
        } else {
            Err(Failure::Refused { code, text })
        }
    }

    // One reply, all its lines: "220-Welcome" ... "220 ".
    fn reply(&mut self) -> Result<(u16, String), Failure> {
        let mut line = String::new();
        if self.control.read_line(&mut line)? == 0 {
            return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "closed").into());
        }
        let code: u16 = line
            .get(..3)
            .and_then(|c| c.parse().ok())
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "not an FTP reply"))?;
        if line.as_bytes().get(3) == Some(&b'-') {
            let end = format!("{code} ");
            loop {
                let mut more = String::new();
                if self.control.read_line(&mut more)? == 0 {
                    return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "closed").into());
                }
                if more.starts_with(&end) {
                    break;
                }
            }
        }
        Ok((code, line.get(4..).unwrap_or_default().trim().to_string()))
    }

    // Opens the data connection the server offers, extended passive mode
    // first, and always to the server's own address: a PASV answer's
    // address can be a private one behind the server's firewall.
    fn transfer(&mut self, command: &str) -> Result<Vec<u8>, Failure> {
        let (code, text) = self.send("EPSV")?;
        let port = if code == 229 {
            text.split('|').nth(3).and_then(|p| p.parse::<u16>().ok())
        } else {
            let (code, text) = self.send("PASV")?;
            if code != 227 {
                return Err(Failure::Refused { code, text });
            }
            let numbers: Vec<u16> = text
                .split(|c: char| !c.is_ascii_digit())
                .filter_map(|n| n.parse().ok())
                .collect();
            numbers
                .len()
                .checked_sub(2)
                .map(|i| numbers[i] * 256 + numbers[i + 1])
        };
        let port =
            port.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no passive port"))?;
        let control = self.control.get_ref();
        let timeout = control.read_timeout()?.unwrap_or(crate::net::TIMEOUT);
        let address = (control.peer_addr()?.ip(), port).into();
        let mut data = TcpStream::connect_timeout(&address, timeout)?;
        data.set_read_timeout(Some(timeout))?;
        let (code, text) = self.send(command)?;
        if code / 100 != 1 {
            return Err(Failure::Refused { code, text });
        }
        let mut body = Vec::new();
        data.read_to_end(&mut body)?;
        drop(data);
        self.expect_reply(2)?;
        Ok(body)
    }
}

#[cfg(test)]
mod tests {
    use std::net::TcpListener;

    use super::*;

    // Answers as the Bureau's server (vsftpd) does, on one connection: a
    // second would get no greeting, so the test fails unless the session
    // is kept.
    fn serve(listener: TcpListener, files: &[(&str, &str)]) {
        let (control, _) = listener.accept().unwrap();
        let mut commands = BufReader::new(control.try_clone().unwrap());
        let mut out = control;
        out.write_all(b"220-Welcome to a test server.\r\n220-\r\n220 \r\n")
            .unwrap();
        let mut passive: Option<TcpListener> = None;
        let mut line = String::new();
        loop {
            line.clear();
            if commands.read_line(&mut line).unwrap() == 0 {
                break;
            }
            let (command, argument) = line
                .trim_end()
                .split_once(' ')
                .unwrap_or((line.trim_end(), ""));
            let reply = match command {
                "USER" => "331 Please specify the password.".to_string(),
                "PASS" => "230 Login successful.".to_string(),
                "TYPE" => "200 Switching to Binary mode.".to_string(),
                "CWD" => "250 Directory successfully changed.".to_string(),
                "EPSV" => {
                    let data = TcpListener::bind("127.0.0.1:0").unwrap();
                    let port = data.local_addr().unwrap().port();
                    passive = Some(data);
                    format!("229 Entering Extended Passive Mode (|||{port}|)")
                }
                "RETR" | "LIST" => {
                    let (mut data, _) = passive.take().unwrap().accept().unwrap();
                    match files.iter().find(|(name, _)| *name == argument) {
                        Some((_, body)) => {
                            out.write_all(b"150 Here comes the directory listing.\r\n")
                                .unwrap();
                            data.write_all(body.as_bytes()).unwrap();
                            drop(data);
                            "226 Transfer complete.".to_string()
                        }
                        None => "550 Failed to open file.".to_string(),
                    }
                }
                _ => "500 Unknown command.".to_string(),
            };
            out.write_all(format!("{reply}\r\n").as_bytes()).unwrap();
        }
    }

    #[test]
    fn files_and_listings_come_over_one_kept_session() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let host = format!("127.0.0.1:{}", listener.local_addr().unwrap().port());
        let listing = "-rw-rw-r--    1 1050     1502         8 Oct 26 01:04 IDN1.cap.xml\r\n";
        let server = std::thread::spawn(move || {
            serve(
                listener,
                &[
                    ("/anon/gen/fwo/IDN1.cap.xml", "<alert/>"),
                    ("*.cap.xml", listing),
                ],
            )
        });
        let ftp = Ftp::default();
        let timeout = Duration::from_secs(5);
        let get = |path: &str| ftp.get(&format!("ftp://{host}{path}"), timeout);
        assert_eq!(get("/anon/gen/fwo/IDN1.cap.xml").unwrap(), b"<alert/>");
        assert_eq!(get("/anon/gen/fwo/*.cap.xml").unwrap(), listing.as_bytes());
        assert_eq!(
            get("/anon/gen/fwo/IDQ2.cap.xml"),
            Err(FetchError::Status {
                code: 550,
                reason: "Failed to open file.".to_string(),
                host: host.clone(),
            })
        );
        // The refused session is closed, which ends the server.
        server.join().unwrap();
    }
}
