// The real Fetch: one HTTP client for the process, with reused
// connections, one User-Agent (the NWS refuses anonymous requests and the
// others ask for identification) and one timeout, as Http.cs had; and
// anonymous FTP for the Bureau of Meteorology (ftp.rs). TLS
// certificates are checked against the system's own store, and Windows'
// proxy setting is followed, as .NET's client did.

use std::time::Duration;

use ureq::Agent;
use ureq::tls::{RootCerts, TlsConfig};

use crate::fetch::{Fetch, FetchError};
use crate::ftp::Ftp;

pub const TIMEOUT: Duration = Duration::from_secs(20);

// ureq's own limit is 10 MB; MeteoAlarm's feed for Switzerland passed 11
// MB on a stormy day (2026-10-25).
const MAX_BODY: u64 = 64 * 1024 * 1024;

// An update's download: the exe is 14 MB, and TIMEOUT's 20 seconds for
// the whole of it would fail on a slow line. ureq has no timeout between
// reads, so this bounds a stalled one; the user's Cancel is the nearer
// way out. The size limit is a backstop against an endless answer.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30 * 60);
const MAX_DOWNLOAD: u64 = 512 * 1024 * 1024;

pub struct UreqFetch {
    agent: Agent,
    ftp: Ftp,
    // Developer-only, like WEATHERSPELL_FONT_POINTS was: every request
    // fails as if the network were down, so the cached view and its wording
    // can be rehearsed without disconnecting the PC.
    offline: bool,
}

impl UreqFetch {
    // "Weatherspell/0.2.0", say; the contact is added here.
    pub fn new(product: &str) -> UreqFetch {
        let config = Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .user_agent(format!(
                "{product} (+https://github.com/PlanetLinux98/weatherspell)"
            ))
            .tls_config(
                TlsConfig::builder()
                    .root_certs(RootCerts::PlatformVerifier)
                    .build(),
            )
            .build();
        UreqFetch {
            agent: config.into(),
            ftp: Ftp::default(),
            offline: std::env::var_os("WEATHERSPELL_OFFLINE").is_some_and(|v| !v.is_empty()),
        }
    }
}

fn host(url: &str) -> String {
    let rest = url.split_once("://").map_or(url, |(_, rest)| rest);
    rest.split(['/', '?', '#'])
        .next()
        .unwrap_or(rest)
        .to_string()
}

impl UreqFetch {
    fn call(
        &self,
        url: &str,
        timeout: Option<Duration>,
    ) -> Result<ureq::http::Response<ureq::Body>, FetchError> {
        let host = host(url);
        if self.offline {
            return Err(FetchError::Offline { host });
        }
        let mut request = self.agent.get(url).header("Accept", "application/json");
        if let Some(t) = timeout {
            request = request.config().timeout_global(Some(t)).build();
        }
        request.call().map_err(|e| failed(e, host))
    }
}

fn failed(e: ureq::Error, host: String) -> FetchError {
    match e {
        ureq::Error::StatusCode(code) => FetchError::Status {
            code,
            reason: ureq::http::StatusCode::from_u16(code)
                .ok()
                .and_then(|s| s.canonical_reason())
                .unwrap_or_default()
                .to_string(),
            host,
        },
        ureq::Error::Timeout(_) => FetchError::TimedOut { host },
        other => FetchError::Unreachable {
            host,
            detail: other.to_string(),
        },
    }
}

impl Fetch for UreqFetch {
    fn get(&self, url: &str, timeout: Option<Duration>) -> Result<String, FetchError> {
        if url.starts_with("ftp://") {
            return self
                .get_bytes(url, timeout)
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned());
        }
        self.call(url, timeout)?
            .body_mut()
            .with_config()
            .limit(MAX_BODY)
            .read_to_string()
            .map_err(|e| failed(e, host(url)))
    }

    fn get_bytes(&self, url: &str, timeout: Option<Duration>) -> Result<Vec<u8>, FetchError> {
        if url.starts_with("ftp://") {
            if self.offline {
                return Err(FetchError::Offline { host: host(url) });
            }
            return self.ftp.get(url, timeout.unwrap_or(TIMEOUT));
        }
        self.call(url, timeout)?
            .body_mut()
            .with_config()
            .limit(MAX_BODY)
            .read_to_vec()
            .map_err(|e| failed(e, host(url)))
    }

    fn download(
        &self,
        url: &str,
        piece: &mut dyn FnMut(&[u8]) -> bool,
    ) -> Result<bool, FetchError> {
        use std::io::Read;

        let host = host(url);
        if self.offline {
            return Err(FetchError::Offline { host });
        }
        let mut response = self
            .agent
            .get(url)
            .config()
            .timeout_global(Some(DOWNLOAD_TIMEOUT))
            .timeout_connect(Some(TIMEOUT))
            .timeout_recv_response(Some(TIMEOUT))
            .build()
            .call()
            .map_err(|e| failed(e, host.clone()))?;
        let mut body = response
            .body_mut()
            .with_config()
            .limit(MAX_DOWNLOAD)
            .reader();
        let mut block = vec![0; 64 * 1024];
        loop {
            match body.read(&mut block) {
                Ok(0) => return Ok(true),
                Ok(n) => {
                    if !piece(&block[..n]) {
                        return Ok(false);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    return Err(FetchError::TimedOut { host });
                }
                Err(e) => {
                    return Err(FetchError::Unreachable {
                        host,
                        detail: e.to_string(),
                    });
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hosts_are_named_from_the_address() {
        assert_eq!(
            super::host("https://dd.weather.gc.ca/today/citypage_weather/ON/03/"),
            "dd.weather.gc.ca"
        );
        assert_eq!(
            super::host("https://api.weather.gov/points/42.65,-73.75"),
            "api.weather.gov"
        );
        assert_eq!(
            super::host("https://forecast.weather.gov/MapClick.php?lat=1"),
            "forecast.weather.gov"
        );
    }
}
