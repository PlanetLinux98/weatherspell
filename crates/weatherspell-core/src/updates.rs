// Check for Updates' logic (#11), with no window and no system calls:
// which published version this copy may be offered, its notes as text,
// the checksum its download must match, and every sentence the app says
// about it. The app passes in the running version, whether previews are
// wanted, and the name of this system's file; swapping the file in is the
// app's, per system.
//
// Checked on request only. The list comes from GitHub's API, anonymous,
// one request a check. SHA256SUMS guards a broken download, not a
// tampered release: signing was left for later (2026-11-10), and a later
// version can start asking for a signature on what it installs.

use std::cmp::Ordering;
use std::fmt;
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;

use crate::fetch::{Fetch, FetchError, ServiceError};
use crate::trouble::Code;

// Newest first by date; 30 is far more than anyone is behind by.
pub const RELEASES_URL: &str =
    "https://api.github.com/repos/PlanetLinux98/weatherspell/releases?per_page=30";
pub const RELEASES_PAGE: &str = "https://github.com/PlanetLinux98/weatherspell/releases";
pub const SUMS_NAME: &str = "SHA256SUMS";
pub const WINGET_COMMAND: &str = "winget upgrade PlanetLinux98.Weatherspell";

// How often the automatic check, when turned on, looks again while the
// app stays open.
pub const AUTOMATIC_CHECK_HOURS: u64 = 24;

// What release.yml attaches for each system.
pub const WINDOWS_FILE: &str = "Weatherspell.exe";
pub const MAC_FILE: &str = "Weatherspell.dmg";
pub const LINUX_FILE: &str = "Weatherspell-x86_64.AppImage";

// A version as the tags and build/version.rs write it: "0.3.0",
// "0.3.0-beta.1", "0.3.0-alpha.0.18" (a build between tags).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Version {
    numbers: [u64; 3],
    pre: Vec<Part>,
}

// In SemVer's order: a number sorts before a word, numbers by value,
// words by their ASCII, and a shorter list before a longer one it begins.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Part {
    Number(u64),
    Word(String),
}

fn number(text: &str) -> Option<u64> {
    if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

impl Version {
    // With or without the tag's "v"; anything after a "+" is ignored, as
    // SemVer orders it.
    pub fn parse(text: &str) -> Option<Version> {
        let text = text.trim();
        let text = text.strip_prefix('v').unwrap_or(text);
        let text = text.split_once('+').map_or(text, |(version, _)| version);
        let (release, pre) = match text.split_once('-') {
            Some((release, pre)) => (release, Some(pre)),
            None => (text, None),
        };
        let mut parts = release.split('.');
        let mut numbers = [0; 3];
        for n in &mut numbers {
            *n = number(parts.next()?)?;
        }
        if parts.next().is_some() {
            return None;
        }
        let pre = match pre {
            None => Vec::new(),
            Some(pre) => pre
                .split('.')
                .map(|part| match number(part) {
                    Some(n) => Some(Part::Number(n)),
                    None if part.is_empty() => None,
                    None => Some(Part::Word(part.to_string())),
                })
                .collect::<Option<Vec<Part>>>()?,
        };
        Some(Version { numbers, pre })
    }

    // Anything with a pre-release part: an alpha, a beta, a build between
    // tags.
    pub fn is_preview(&self) -> bool {
        !self.pre.is_empty()
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Version) -> Ordering {
        self.numbers.cmp(&other.numbers).then_with(|| {
            match (self.pre.is_empty(), other.pre.is_empty()) {
                (true, true) => Ordering::Equal,
                // 0.3.0 comes after every 0.3.0 preview.
                (true, false) => Ordering::Greater,
                (false, true) => Ordering::Less,
                (false, false) => self.pre.cmp(&other.pre),
            }
        })
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Version) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let [major, minor, patch] = self.numbers;
        write!(f, "{major}.{minor}.{patch}")?;
        for (i, part) in self.pre.iter().enumerate() {
            f.write_str(if i == 0 { "-" } else { "." })?;
            match part {
                Part::Number(n) => write!(f, "{n}")?,
                Part::Word(w) => f.write_str(w)?,
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    pub preview: bool,
    // As published: Markdown.
    pub notes: String,
    pub page: String,
    pub files: Vec<File>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct File {
    pub name: String,
    pub url: String,
    pub size: u64,
}

impl Release {
    fn file(&self, name: &str) -> Option<&File> {
        self.files.iter().find(|f| f.name == name)
    }
}

#[derive(Deserialize)]
struct RawRelease {
    tag_name: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    body: Option<String>,
    #[serde(default)]
    html_url: Option<String>,
    #[serde(default)]
    assets: Vec<RawFile>,
}

#[derive(Deserialize)]
struct RawFile {
    name: String,
    browser_download_url: String,
    #[serde(default)]
    size: u64,
}

// GitHub's list as it answers. Drafts (never listed to an anonymous
// request, left out all the same) and tags that are not versions are
// skipped.
pub fn releases(json: &str) -> Result<Vec<Release>, ServiceError> {
    let raw: Vec<RawRelease> = serde_json::from_str(json).map_err(|_| {
        ServiceError::Unreadable("GitHub's list of versions could not be read".to_string())
    })?;
    Ok(raw
        .into_iter()
        .filter(|r| !r.draft)
        .filter_map(|r| {
            let version = Version::parse(&r.tag_name)?;
            Some(Release {
                preview: r.prerelease || version.is_preview(),
                version,
                notes: r.body.unwrap_or_default(),
                page: r
                    .html_url
                    .filter(|url| !url.is_empty())
                    .unwrap_or_else(|| RELEASES_PAGE.to_string()),
                files: r
                    .assets
                    .into_iter()
                    .map(|a| File {
                        name: a.name,
                        url: a.browser_download_url,
                        size: a.size,
                    })
                    .collect(),
            })
        })
        .collect())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Answer {
    // Nothing newer that this copy may be offered.
    Latest,
    // A preview, with previews turned off, that is ahead of the newest
    // official version. Nothing older is offered.
    Ahead { official: Version },
    Newer(Update),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Update {
    pub version: Version,
    // Ready to read: text, not Markdown.
    pub notes: String,
    pub file: File,
    // None when the release has no SHA256SUMS: shown, never installed.
    pub sums: Option<File>,
    pub page: String,
}

// The newest version this copy may be offered: official versions, and
// previews too when asked for. A release without this system's file is
// not one (0.2.0-alpha.1 had only the exe).
pub fn check(releases: &[Release], running: &Version, previews: bool, file_name: &str) -> Answer {
    let newest = releases
        .iter()
        .filter(|r| (previews || !r.preview) && r.file(file_name).is_some())
        .max_by(|a, b| a.version.cmp(&b.version));
    let Some(newest) = newest else {
        return Answer::Latest;
    };
    match newest.version.cmp(running) {
        Ordering::Greater => Answer::Newer(Update {
            version: newest.version.clone(),
            notes: notes(releases, running, newest),
            file: newest.file(file_name).expect("chosen for it").clone(),
            sums: newest.file(SUMS_NAME).cloned(),
            page: newest.page.clone(),
        }),
        Ordering::Less if running.is_preview() && !previews => Answer::Ahead {
            official: newest.version.clone(),
        },
        _ => Answer::Latest,
    }
}

// Official versions are each written against the one before, so someone
// several behind gets them all, newest first, each under its version. A
// preview's notes recap its cycle, so only the newest one's are shown.
fn notes(releases: &[Release], running: &Version, newest: &Release) -> String {
    let mut passed: Vec<&Release> = releases
        .iter()
        .filter(|r| !r.preview && r.version > *running && r.version <= newest.version)
        .collect();
    passed.sort_by(|a, b| b.version.cmp(&a.version));
    let text = if newest.preview || passed.len() < 2 {
        notes_text(&newest.notes)
    } else {
        passed
            .iter()
            .map(|r| format!("Version {}\n\n{}", r.version, notes_text(&r.notes)))
            .collect::<Vec<_>>()
            .join("\n\n")
    };
    if text.is_empty() {
        "No notes were published for this version.".to_string()
    } else {
        text
    }
}

// Release notes are Markdown, read here as text: a heading is a line of
// its own, bullets stay, a link is its words, and bold and code marks go.
pub fn notes_text(markdown: &str) -> String {
    static HEADING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s{0,3}#{1,6}\s+").unwrap());
    static STAR: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(\s*)[*+] ").unwrap());
    static LINK: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"!?\[([^\]]*)\]\([^)\s]*\)").unwrap());
    static POINTED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"<(https?://[^>\s]+)>").unwrap());
    static ESCAPED: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\\([\\`*_{}\[\]()#+\-.!>])").unwrap());

    let mut lines: Vec<String> = Vec::new();
    for line in markdown.replace("\r\n", "\n").lines() {
        let line = HEADING.replace(line.trim_end(), "");
        let line = STAR.replace(&line, "$1- ");
        let line = LINK.replace_all(&line, "$1");
        let line = POINTED.replace_all(&line, "$1");
        let line = line.replace("**", "").replace('`', "");
        let line = ESCAPED.replace_all(&line, "$1").into_owned();
        // One empty line between paragraphs, none at the start.
        if line.is_empty() && lines.last().is_none_or(|last| last.is_empty()) {
            continue;
        }
        lines.push(line);
    }
    while lines.last().is_some_and(|last| last.is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

// A file's line in SHA256SUMS, as sha256sum writes it ("<64 hex>  name",
// or " *name" for binary mode): its checksum in lower case.
pub fn listed_sum(sums: &str, file_name: &str) -> Option<String> {
    sums.lines().find_map(|line| {
        let (sum, name) = line.trim().split_once(char::is_whitespace)?;
        let name = name.trim_start();
        let name = name.strip_prefix('*').unwrap_or(name);
        (name == file_name && sum.len() == 64 && sum.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| sum.to_ascii_lowercase())
    })
}

// Whether the exe runs from where winget puts a portable package: its
// Packages folder (in the user's AppData, or Program Files for all users)
// or the Links folder beside it. winget owns those, so such a copy is
// pointed at `winget upgrade`.
pub fn is_winget_path(exe: &str) -> bool {
    let path = exe.replace('\\', "/").to_lowercase();
    path.contains("/winget/packages/") || path.contains("/winget/links/")
}

// One request: GitHub's list, and what it means for this copy.
pub fn look(
    http: &dyn Fetch,
    feed: &str,
    running: &Version,
    previews: bool,
    file_name: &str,
) -> Result<Answer, ServiceError> {
    let json = http.get(feed, None).map_err(ServiceError::Fetch)?;
    Ok(check(&releases(&json)?, running, previews, file_name))
}

// Why a download was not kept. Nothing is left on disk after any of them.
#[derive(Debug)]
pub enum DownloadProblem {
    // No SHA256SUMS on the release, or no line in it for this file.
    NoChecksum,
    Mismatch,
    Fetch(FetchError),
    File(std::io::Error),
    // The user's Cancel: nothing to say.
    Cancelled,
}

impl fmt::Display for DownloadProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DownloadProblem::NoChecksum => write!(
                f,
                "This version was published without a checksum to check the download against, so nothing was installed. {}",
                Code::U2
            ),
            DownloadProblem::Mismatch => write!(
                f,
                "The download didn't match its checksum, so nothing was installed. Try again, or download it from the releases page. {}",
                Code::U3
            ),
            DownloadProblem::Fetch(e) => write!(
                f,
                "Couldn't download the update: {e}. Nothing was installed. {}",
                Code::U4
            ),
            DownloadProblem::File(e) => write!(
                f,
                "Couldn't save the download: {e}. Nothing was installed. {}",
                Code::U5
            ),
            DownloadProblem::Cancelled => f.write_str("The download was cancelled."),
        }
    }
}

impl std::error::Error for DownloadProblem {}

// The update's file written to `to` and checked against the release's
// SHA256SUMS; `to` exists afterwards only if it matched. The checksum is
// fetched first, so a release without one costs no download, and is
// taken from the file as it lies on disk, not from the bytes as they
// came. `keep_going` hears how much has arrived and stops it with false.
#[cfg(feature = "net")]
pub fn download(
    http: &dyn Fetch,
    update: &Update,
    to: &std::path::Path,
    keep_going: &mut dyn FnMut(u64) -> bool,
) -> Result<(), DownloadProblem> {
    use std::io::Write;

    let sums = update.sums.as_ref().ok_or(DownloadProblem::NoChecksum)?;
    let sums = http.get(&sums.url, None).map_err(DownloadProblem::Fetch)?;
    let expected = listed_sum(&sums, &update.file.name).ok_or(DownloadProblem::NoChecksum)?;

    let fetched = (|| {
        let mut file = std::fs::File::create(to).map_err(DownloadProblem::File)?;
        let mut received = 0u64;
        let mut failed = None;
        let finished = http
            .download(&update.file.url, &mut |bytes| {
                if let Err(e) = file.write_all(bytes) {
                    failed = Some(e);
                    return false;
                }
                received += bytes.len() as u64;
                keep_going(received)
            })
            .map_err(DownloadProblem::Fetch)?;
        if let Some(e) = failed {
            return Err(DownloadProblem::File(e));
        }
        if !finished {
            return Err(DownloadProblem::Cancelled);
        }
        file.flush().map_err(DownloadProblem::File)?;
        drop(file);
        if sha256(to).map_err(DownloadProblem::File)? == expected {
            Ok(())
        } else {
            Err(DownloadProblem::Mismatch)
        }
    })();
    if fetched.is_err() {
        let _ = std::fs::remove_file(to);
    }
    fetched
}

// A file's SHA-256 in lower-case hex. ring is in the tree already, for
// TLS, so the checksum adds no crate.
#[cfg(feature = "net")]
pub fn sha256(path: &std::path::Path) -> std::io::Result<String> {
    use std::io::Read;

    let mut file = std::fs::File::open(path)?;
    let mut hash = ring::digest::Context::new(&ring::digest::SHA256);
    let mut block = vec![0; 64 * 1024];
    loop {
        let n = file.read(&mut block)?;
        if n == 0 {
            break;
        }
        hash.update(&block[..n]);
    }
    Ok(hash
        .finish()
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

// What the app says, kept here so the wording is tested with the rest.
pub mod wording {
    use super::{Update, Version, WINGET_COMMAND};
    use crate::fetch::ServiceError;
    use crate::trouble::Code;

    pub const CHECKING: &str = "Checking for updates...";

    pub fn latest(running: &Version) -> String {
        format!("You have the latest version, {running}.")
    }

    pub fn ahead(running: &Version, official: &Version) -> String {
        format!("You have {running}, a preview newer than the latest version, {official}.")
    }

    pub fn could_not_check(problem: &ServiceError) -> String {
        format!("Couldn't check for updates: {problem}. {}", Code::U1)
    }

    // Update Available's first line.
    pub fn available(update: &Update, running: &Version) -> String {
        format!(
            "Weatherspell {} is available. You have {running}.",
            update.version
        )
    }

    // Added to it where the app's folder takes an administrator to change.
    pub const NEEDS_PERMISSION: &str = "Installing will ask for an administrator's permission, because of the folder Weatherspell is in.";

    // In place of the line above for a copy winget installed: winget owns
    // that folder, and its version usually trails GitHub's by some days.
    pub fn winget(update: &Update, running: &Version) -> String {
        format!(
            "Weatherspell {} is available. You have {running}, installed with winget. To update, run: {WINGET_COMMAND}. A new version can take a few days to reach winget.",
            update.version
        )
    }

    // A copy with nothing to swap: built from source, a distro's package,
    // an AppImage in a folder the user can't write to.
    pub fn by_hand(update: &Update, running: &Version) -> String {
        format!(
            "Weatherspell {} is available. You have {running}. This copy can't update itself, so the new version is on its download page.",
            update.version
        )
    }

    pub const WHATS_NEW: &str = "What's &new";
    pub const COPIED: &str = "Copied.";

    // The automatic check's finding: said once, and kept at the end of
    // the status bar's line until the app is relaunched.
    pub fn found(version: &Version) -> String {
        format!("Weatherspell {version} is available. Check for Updates is in the Help menu.")
    }

    pub fn found_status(version: &Version) -> String {
        format!("Weatherspell {version} is available")
    }

    pub fn downloading(update: &Update) -> String {
        format!("Downloading Weatherspell {}...", update.version)
    }

    pub fn installed(version: &Version) -> String {
        format!("Weatherspell {version} is installed. Relaunch to start using it.")
    }

    pub const RELAUNCH_NOW: &str = "&Relaunch Now";
    pub const LATER: &str = "&Later";

    // The download was good, and putting it in place failed or was not
    // allowed: the reason after it, when there is one.
    pub fn not_installed(reason: &str) -> String {
        if reason.is_empty() {
            format!("Nothing was installed. {}", Code::U6)
        } else {
            format!("Nothing was installed: {reason} {}", Code::U6)
        }
    }
}
