// Check for Updates (#11): versions in order, which release a copy is
// offered, its notes as text, the checksum, and the wording. The list is
// GitHub's own answer for the 0.2.0 cycle, captured 2026-11-10 (trimmed
// to the members the app reads, plus a few beside them).

mod common;

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Duration;

use common::fixture;
use weatherspell_core::fetch::{Fetch, FetchError, ServiceError};
use weatherspell_core::updates::{
    self, Answer, File, LINUX_FILE, MAC_FILE, Release, Update, Version, WINDOWS_FILE, wording,
};

fn v(text: &str) -> Version {
    Version::parse(text).unwrap_or_else(|| panic!("{text} is a version"))
}

fn published() -> Vec<Release> {
    updates::releases(&fixture("github-releases.json")).unwrap()
}

fn file(name: &str) -> File {
    File {
        name: name.to_string(),
        url: format!("https://example.test/{name}"),
        size: 3,
    }
}

fn release(version: &str, notes: &str, files: &[&str]) -> Release {
    let version = v(version);
    Release {
        preview: version.is_preview(),
        version,
        notes: notes.to_string(),
        page: updates::RELEASES_PAGE.to_string(),
        files: files.iter().map(|name| file(name)).collect(),
    }
}

fn newer(answer: Answer) -> Update {
    match answer {
        Answer::Newer(update) => update,
        other => panic!("expected an update, got {other:?}"),
    }
}

#[test]
fn versions_read_as_the_tags_and_the_app_write_them() {
    for (text, shown) in [
        ("v0.3.0", "0.3.0"),
        ("0.3.0", "0.3.0"),
        ("v0.2.0-beta.1", "0.2.0-beta.1"),
        ("0.3.0-alpha.0.18", "0.3.0-alpha.0.18"),
        (" v1.20.3 ", "1.20.3"),
        ("1.0.0+build.5", "1.0.0"),
    ] {
        assert_eq!(v(text).to_string(), shown);
    }
    for junk in [
        "",
        "v",
        "1",
        "1.2",
        "1.2.3.4",
        "1.2.x",
        "1.2.3-",
        "1.2.3-a..b",
        "+1.2.3",
        "1.+2.3",
        "latest",
    ] {
        assert_eq!(Version::parse(junk), None, "{junk:?}");
    }
}

#[test]
fn versions_sort_as_semver_orders_them() {
    let in_order = [
        "0.1.0",
        "0.2.0-alpha.0.18",
        "0.2.0-alpha.1",
        "0.2.0-alpha.1.3",
        "0.2.0-alpha.2",
        "0.2.0-beta.1",
        "0.2.0-beta.2",
        "0.2.0-beta.11",
        "0.2.0-rc.1",
        "0.2.0",
        "0.2.1",
        "0.10.0",
        "1.0.0",
    ];
    for pair in in_order.windows(2) {
        assert!(v(pair[0]) < v(pair[1]), "{} before {}", pair[0], pair[1]);
        assert!(v(pair[1]) > v(pair[0]), "{} after {}", pair[1], pair[0]);
    }
    assert_eq!(v("1.0.0+a"), v("1.0.0+b"));
}

#[test]
fn a_build_between_tags_is_a_preview() {
    assert!(!v("0.3.0").is_preview());
    assert!(v("0.3.0-beta.1").is_preview());
    assert!(v("0.3.0-alpha.0.18").is_preview());
}

#[test]
fn githubs_list_is_read() {
    let all = published();
    let versions: Vec<String> = all.iter().map(|r| r.version.to_string()).collect();
    assert_eq!(
        versions,
        [
            "0.2.0",
            "0.2.0-beta.1",
            "0.2.0-alpha.2",
            "0.2.0-alpha.1",
            "0.1.0"
        ]
    );
    let previews: Vec<bool> = all.iter().map(|r| r.preview).collect();
    assert_eq!(previews, [false, true, true, true, false]);

    let exe = all[0]
        .files
        .iter()
        .find(|f| f.name == WINDOWS_FILE)
        .unwrap();
    assert_eq!(exe.size, 14_377_472);
    assert_eq!(
        exe.url,
        "https://github.com/PlanetLinux98/weatherspell/releases/download/v0.2.0/Weatherspell.exe"
    );
    assert_eq!(
        all[0].page,
        "https://github.com/PlanetLinux98/weatherspell/releases/tag/v0.2.0"
    );
}

#[test]
fn drafts_and_tags_that_are_not_versions_are_left_out() {
    let json = r#"[
        {"tag_name": "v0.3.0", "draft": true, "prerelease": false, "body": "x", "assets": []},
        {"tag_name": "nightly", "draft": false, "prerelease": true, "body": "x", "assets": []},
        {"tag_name": "v0.2.1", "body": null}
    ]"#;
    let read = updates::releases(json).unwrap();
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].version, v("0.2.1"));
    assert_eq!(read[0].notes, "");
    assert_eq!(read[0].page, updates::RELEASES_PAGE);
    assert!(read[0].files.is_empty());
}

#[test]
fn an_answer_that_is_not_a_list_is_said_so() {
    let problem = updates::releases(r#"{"message": "API rate limit exceeded"}"#).unwrap_err();
    assert_eq!(
        problem,
        ServiceError::Unreadable("GitHub's list of versions could not be read".to_string())
    );
    assert_eq!(
        wording::could_not_check(&problem),
        "Couldn't check for updates: GitHub's list of versions could not be read."
    );
}

#[test]
fn the_newest_official_version_is_offered() {
    let update = newer(updates::check(
        &published(),
        &v("0.1.0"),
        false,
        WINDOWS_FILE,
    ));
    assert_eq!(update.version, v("0.2.0"));
    assert_eq!(update.file.name, WINDOWS_FILE);
    assert_eq!(update.sums.as_ref().unwrap().name, "SHA256SUMS");
    assert_eq!(
        update.page,
        "https://github.com/PlanetLinux98/weatherspell/releases/tag/v0.2.0"
    );
    // One version behind: its notes alone, with no version line.
    assert!(
        update
            .notes
            .starts_with("The first release for the Mac and Linux, alongside Windows.")
    );
    assert!(
        update
            .notes
            .contains("\n\nAdded\n- Weatherspell for the Mac,")
    );
    assert!(
        update
            .notes
            .contains("- Windows: download Weatherspell.exe and run it.")
    );
    // Issue numbers keep their "#"; headings lose theirs.
    assert!(update.notes.contains("(#26, #27)"));
    assert!(!update.notes.lines().any(|line| line.starts_with('#')));
    for mark in ["**", "`", "\r"] {
        assert!(!update.notes.contains(mark), "{mark:?} left in the notes");
    }
}

#[test]
fn the_running_version_and_older_ones_are_not_offered() {
    let all = published();
    assert_eq!(
        updates::check(&all, &v("0.2.0"), false, WINDOWS_FILE),
        Answer::Latest
    );
    assert_eq!(
        updates::check(&all, &v("0.2.0"), true, WINDOWS_FILE),
        Answer::Latest
    );
    assert_eq!(
        updates::check(&all, &v("0.2.1"), true, WINDOWS_FILE),
        Answer::Latest
    );
    assert_eq!(
        updates::check(&[], &v("0.2.0"), true, WINDOWS_FILE),
        Answer::Latest
    );
}

#[test]
fn a_preview_is_offered_the_official_version_that_follows_it() {
    for previews in [false, true] {
        let update = newer(updates::check(
            &published(),
            &v("0.2.0-beta.1"),
            previews,
            MAC_FILE,
        ));
        assert_eq!(update.version, v("0.2.0"));
        assert_eq!(update.file.name, MAC_FILE);
    }
}

#[test]
fn previews_are_offered_only_when_asked_for() {
    let all = [
        release("0.3.0-alpha.2", "Second.", &[WINDOWS_FILE, "SHA256SUMS"]),
        release("0.3.0-alpha.1", "First.", &[WINDOWS_FILE, "SHA256SUMS"]),
        release("0.2.0", "Official.", &[WINDOWS_FILE, "SHA256SUMS"]),
    ];
    assert_eq!(
        updates::check(&all, &v("0.2.0"), false, WINDOWS_FILE),
        Answer::Latest
    );
    let update = newer(updates::check(&all, &v("0.2.0"), true, WINDOWS_FILE));
    assert_eq!(update.version, v("0.3.0-alpha.2"));
    // A preview's notes recap the cycle: only the newest one's.
    assert_eq!(update.notes, "Second.");
    // A build between the two previews is offered the later one.
    let update = newer(updates::check(
        &all,
        &v("0.3.0-alpha.1.4"),
        true,
        WINDOWS_FILE,
    ));
    assert_eq!(update.version, v("0.3.0-alpha.2"));
}

#[test]
fn a_preview_ahead_of_the_official_version_is_told_so_and_offered_nothing_older() {
    let all = published();
    assert_eq!(
        updates::check(&all, &v("0.3.0-alpha.1"), false, WINDOWS_FILE),
        Answer::Ahead {
            official: v("0.2.0")
        }
    );
    assert_eq!(
        updates::check(&all, &v("0.3.0-alpha.1"), true, WINDOWS_FILE),
        Answer::Latest
    );
    assert_eq!(
        wording::ahead(&v("0.3.0-alpha.1"), &v("0.2.0")),
        "You have 0.3.0-alpha.1, a preview newer than the latest version, 0.2.0."
    );
}

#[test]
fn a_release_without_this_systems_file_is_not_offered() {
    // 0.2.0-alpha.1 had only the exe.
    let all = published();
    let old = v("0.1.0");
    let before_linux = [all[3].clone(), all[4].clone()];
    assert_eq!(
        newer(updates::check(&before_linux, &old, true, WINDOWS_FILE)).version,
        v("0.2.0-alpha.1")
    );
    assert_eq!(
        updates::check(&before_linux, &old, true, LINUX_FILE),
        Answer::Latest
    );
    assert_eq!(
        newer(updates::check(&all, &old, false, LINUX_FILE))
            .file
            .name,
        LINUX_FILE
    );
}

#[test]
fn several_versions_behind_reads_each_official_versions_notes_newest_first() {
    let all = [
        release("0.4.0-alpha.1", "Preview.", &[WINDOWS_FILE]),
        release("0.3.0", "### Added\n- Updates.", &[WINDOWS_FILE]),
        release("0.3.0-beta.1", "Beta.", &[WINDOWS_FILE]),
        release("0.2.1", "### Fixed\n- A fix.", &[WINDOWS_FILE]),
        release("0.2.0", "Old.", &[WINDOWS_FILE]),
    ];
    let update = newer(updates::check(&all, &v("0.2.0"), false, WINDOWS_FILE));
    assert_eq!(update.version, v("0.3.0"));
    assert_eq!(
        update.notes,
        "Version 0.3.0\n\nAdded\n- Updates.\n\nVersion 0.2.1\n\nFixed\n- A fix."
    );
    // With previews, the newest is a preview: its notes alone.
    let update = newer(updates::check(&all, &v("0.2.0"), true, WINDOWS_FILE));
    assert_eq!(update.version, v("0.4.0-alpha.1"));
    assert_eq!(update.notes, "Preview.");
}

#[test]
fn a_release_without_checksums_or_notes_is_still_shown() {
    let all = [release("0.3.0", " \r\n", &[WINDOWS_FILE])];
    let update = newer(updates::check(&all, &v("0.2.0"), false, WINDOWS_FILE));
    assert_eq!(update.sums, None);
    assert_eq!(update.notes, "No notes were published for this version.");
}

#[test]
fn notes_are_read_as_text() {
    let markdown = "\r\n## What's new\r\n\r\n\r\n- **Windows:** download `Weatherspell.exe`.\r\n* See [the guide](https://example.test/guide) or <https://example.test>.\r\n  + Nested, with 2 \\* 3 and snake_case kept.\r\n\r\n### Fixed\r\n- A fix (#23).\r\n\r\n";
    assert_eq!(
        updates::notes_text(markdown),
        "What's new\n\n- Windows: download Weatherspell.exe.\n- See the guide or https://example.test.\n  - Nested, with 2 * 3 and snake_case kept.\n\nFixed\n- A fix (#23)."
    );
}

#[test]
fn a_files_checksum_is_found_in_the_list() {
    let sums = "64777ae435eb3e3d0450611b086a9e6a0f1263713e336557de658bf718ddcb6f  Weatherspell.exe\n\
                5DB163F73C0294B6A32E428026CA4E2FFE11FBE3BB0FE0F38BB20D6EA608C1D2 *Weatherspell.dmg\r\n\
                abc123  Weatherspell-x86_64.AppImage\n";
    assert_eq!(
        updates::listed_sum(sums, "Weatherspell.exe").as_deref(),
        Some("64777ae435eb3e3d0450611b086a9e6a0f1263713e336557de658bf718ddcb6f")
    );
    assert_eq!(
        updates::listed_sum(sums, "Weatherspell.dmg").as_deref(),
        Some("5db163f73c0294b6a32e428026ca4e2ffe11fbe3bb0fe0f38bb20d6ea608c1d2")
    );
    // Too short to be one, a name that only begins the same, no line.
    assert_eq!(
        updates::listed_sum(sums, "Weatherspell-x86_64.AppImage"),
        None
    );
    assert_eq!(updates::listed_sum(sums, "Weatherspell"), None);
    assert_eq!(updates::listed_sum(sums, "SHA256SUMS"), None);
    assert_eq!(updates::listed_sum("", "Weatherspell.exe"), None);
}

#[test]
fn the_app_says_it_in_these_words() {
    let update = newer(updates::check(
        &published(),
        &v("0.1.0"),
        false,
        WINDOWS_FILE,
    ));
    let running = v("0.1.0");
    assert_eq!(wording::CHECKING, "Checking for updates...");
    assert_eq!(
        wording::latest(&v("0.3.0")),
        "You have the latest version, 0.3.0."
    );
    assert_eq!(
        wording::available(&update, &running),
        "Weatherspell 0.2.0 is available. You have 0.1.0."
    );
    assert_eq!(
        wording::winget(&update, &running),
        "Weatherspell 0.2.0 is available. You have 0.1.0, installed with winget. To update, run: winget upgrade PlanetLinux98.Weatherspell. A new version can take a few days to reach winget."
    );
    assert_eq!(
        wording::by_hand(&update, &running),
        "Weatherspell 0.2.0 is available. You have 0.1.0. This copy can't update itself, so the new version is on its download page."
    );
    assert_eq!(
        wording::downloading(&update),
        "Downloading Weatherspell 0.2.0..."
    );
    assert_eq!(
        wording::installed(&update),
        "Weatherspell 0.2.0 is installed. Relaunch to start using it."
    );
}

// Answers by address; anything else is a 404.
#[derive(Default)]
struct Fake {
    answers: HashMap<String, Result<String, FetchError>>,
    asked: Mutex<Vec<String>>,
}

impl Fake {
    fn answer(mut self, url: &str, body: Result<&str, FetchError>) -> Fake {
        self.answers
            .insert(url.to_string(), body.map(str::to_string));
        self
    }

    fn asked(&self) -> Vec<String> {
        self.asked.lock().unwrap().clone()
    }
}

impl Fetch for Fake {
    fn get(&self, url: &str, _timeout: Option<Duration>) -> Result<String, FetchError> {
        self.asked.lock().unwrap().push(url.to_string());
        self.answers.get(url).cloned().unwrap_or_else(|| {
            Err(FetchError::Status {
                code: 404,
                reason: "Not Found".to_string(),
                host: "example.test".to_string(),
            })
        })
    }
}

fn unreachable(host: &str) -> FetchError {
    FetchError::Unreachable {
        host: host.to_string(),
        detail: "no route".to_string(),
    }
}

#[test]
fn one_request_answers_a_check() {
    let json = fixture("github-releases.json");
    let http = Fake::default().answer(updates::RELEASES_URL, Ok(&json));
    let answer = updates::look(
        &http,
        updates::RELEASES_URL,
        &v("0.2.0-beta.1"),
        true,
        LINUX_FILE,
    )
    .unwrap();
    assert_eq!(newer(answer).version, v("0.2.0"));
    assert_eq!(http.asked(), [updates::RELEASES_URL]);
}

#[test]
fn a_check_that_gets_no_answer_says_why() {
    let http = Fake::default().answer(updates::RELEASES_URL, Err(unreachable("api.github.com")));
    let problem = updates::look(
        &http,
        updates::RELEASES_URL,
        &v("0.2.0"),
        false,
        WINDOWS_FILE,
    )
    .unwrap_err();
    assert_eq!(
        wording::could_not_check(&problem),
        "Couldn't check for updates: api.github.com could not be reached."
    );
}

#[cfg(feature = "net")]
mod downloads {
    use super::*;
    use common::TempDir;
    use weatherspell_core::updates::DownloadProblem;

    // SHA-256 of "abc", from the standard's own examples.
    const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    const FILE_URL: &str = "https://example.test/Weatherspell.exe";
    const SUMS_URL: &str = "https://example.test/SHA256SUMS";

    fn update(with_sums: bool) -> Update {
        Update {
            version: v("0.3.0"),
            notes: String::new(),
            file: file(WINDOWS_FILE),
            sums: with_sums.then(|| file("SHA256SUMS")),
            page: updates::RELEASES_PAGE.to_string(),
        }
    }

    fn target(dir: &TempDir) -> std::path::PathBuf {
        std::fs::create_dir_all(&dir.0).unwrap();
        dir.join("Weatherspell.exe.new")
    }

    fn sums(hash: &str) -> String {
        format!("{hash}  Weatherspell.exe\n")
    }

    #[test]
    fn a_file_is_hashed_as_sha256sum_does() {
        let dir = TempDir::new();
        let path = target(&dir);
        std::fs::write(&path, "abc").unwrap();
        assert_eq!(updates::sha256(&path).unwrap(), ABC);
        std::fs::write(&path, "").unwrap();
        assert_eq!(
            updates::sha256(&path).unwrap(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn a_download_that_matches_its_checksum_is_kept() {
        let dir = TempDir::new();
        let path = target(&dir);
        let http = Fake::default()
            .answer(SUMS_URL, Ok(&sums(ABC)))
            .answer(FILE_URL, Ok("abc"));
        let mut heard = Vec::new();
        updates::download(&http, &update(true), &path, &mut |received| {
            heard.push(received);
            true
        })
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"abc");
        assert_eq!(heard, [3]);
        // The checksum first, so a release without one costs no download.
        assert_eq!(http.asked(), [SUMS_URL, FILE_URL]);
    }

    #[test]
    fn a_download_that_differs_is_refused_and_removed() {
        let dir = TempDir::new();
        let path = target(&dir);
        let http = Fake::default()
            .answer(SUMS_URL, Ok(&sums(ABC)))
            .answer(FILE_URL, Ok("abd"));
        let problem = updates::download(&http, &update(true), &path, &mut |_| true).unwrap_err();
        assert!(matches!(problem, DownloadProblem::Mismatch));
        assert!(!path.exists());
        assert_eq!(
            problem.to_string(),
            "The download didn't match its checksum, so nothing was installed. Try again, or download it from the releases page."
        );
    }

    #[test]
    fn without_a_checksum_nothing_is_downloaded() {
        let dir = TempDir::new();
        let path = target(&dir);
        // No SHA256SUMS on the release at all.
        let http = Fake::default().answer(FILE_URL, Ok("abc"));
        let problem = updates::download(&http, &update(false), &path, &mut |_| true).unwrap_err();
        assert!(matches!(problem, DownloadProblem::NoChecksum));
        assert!(http.asked().is_empty());

        // One that does not list this file.
        let http = Fake::default()
            .answer(SUMS_URL, Ok(&format!("{ABC}  Weatherspell.dmg\n")))
            .answer(FILE_URL, Ok("abc"));
        let problem = updates::download(&http, &update(true), &path, &mut |_| true).unwrap_err();
        assert!(matches!(problem, DownloadProblem::NoChecksum));
        assert_eq!(http.asked(), [SUMS_URL]);
        assert!(!path.exists());
        assert_eq!(
            problem.to_string(),
            "This version was published without a checksum to check the download against, so nothing was installed."
        );
    }

    #[test]
    fn cancel_leaves_nothing_behind() {
        let dir = TempDir::new();
        let path = target(&dir);
        let http = Fake::default()
            .answer(SUMS_URL, Ok(&sums(ABC)))
            .answer(FILE_URL, Ok("abc"));
        let problem = updates::download(&http, &update(true), &path, &mut |_| false).unwrap_err();
        assert!(matches!(problem, DownloadProblem::Cancelled));
        assert!(!path.exists());
    }

    #[test]
    fn a_download_that_fails_says_why_and_leaves_nothing_behind() {
        let dir = TempDir::new();
        let path = target(&dir);
        let http = Fake::default()
            .answer(SUMS_URL, Ok(&sums(ABC)))
            .answer(FILE_URL, Err(unreachable("github.com")));
        let problem = updates::download(&http, &update(true), &path, &mut |_| true).unwrap_err();
        assert!(!path.exists());
        assert_eq!(
            problem.to_string(),
            "Couldn't download the update: github.com could not be reached. Nothing was installed."
        );

        // The checksum list itself out of reach.
        let http = Fake::default().answer(SUMS_URL, Err(unreachable("github.com")));
        let problem = updates::download(&http, &update(true), &path, &mut |_| true).unwrap_err();
        assert!(matches!(problem, DownloadProblem::Fetch(_)));
        assert!(!path.exists());
    }

    #[test]
    fn a_folder_that_cannot_be_written_to_is_said_so() {
        let dir = TempDir::new();
        // The folder was never made.
        let path = dir.join("missing").join("Weatherspell.exe.new");
        let http = Fake::default()
            .answer(SUMS_URL, Ok(&sums(ABC)))
            .answer(FILE_URL, Ok("abc"));
        let problem = updates::download(&http, &update(true), &path, &mut |_| true).unwrap_err();
        assert!(matches!(problem, DownloadProblem::File(_)));
        assert!(
            problem
                .to_string()
                .starts_with("Couldn't save the download: ")
        );
    }
}
