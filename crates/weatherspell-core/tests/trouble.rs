// The codes that end the app's failure messages, and the user guide's
// Troubleshooting section: each code has its entry there, under the words
// its message opens with, and the guide lists no code the app lacks.

use weatherspell_core::trouble::Code;

fn guide() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../USER_GUIDE.md");
    std::fs::read_to_string(path)
        .expect("USER_GUIDE.md")
        .replace("\r\n", "\n")
}

// From "## Troubleshooting" to the next section.
fn troubleshooting(guide: &str) -> &str {
    let start = guide
        .find("\n## Troubleshooting\n")
        .expect("the guide has a Troubleshooting section");
    let rest = &guide[start + 1..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |at| at + 3);
    &rest[..end]
}

#[test]
fn a_code_reads_as_the_messages_end_with_it() {
    assert_eq!(Code::U3.to_string(), "(Code U3)");
    assert_eq!(
        format!("Nothing was installed. {}", Code::U6),
        "Nothing was installed. (Code U6)"
    );
}

#[test]
fn the_cores_own_messages_open_with_their_codes_words_and_end_with_the_code() {
    use weatherspell_core::fetch::{FetchError, ServiceError};
    use weatherspell_core::updates::{DownloadProblem, wording};

    let slow = FetchError::TimedOut {
        host: "github.com".to_string(),
    };
    let cases = [
        (
            wording::could_not_check(&ServiceError::Fetch(slow.clone())),
            Code::U1,
        ),
        (DownloadProblem::NoChecksum.to_string(), Code::U2),
        (DownloadProblem::Mismatch.to_string(), Code::U3),
        (DownloadProblem::Fetch(slow).to_string(), Code::U4),
        (
            DownloadProblem::File(std::io::Error::other("the disk is full")).to_string(),
            Code::U5,
        ),
        (wording::not_installed(""), Code::U6),
        (wording::not_installed("Access is denied."), Code::U6),
    ];
    for (message, code) in cases {
        assert!(message.starts_with(code.opening()), "{message}");
        assert!(message.ends_with(&format!(" {code}")), "{message}");
    }
    assert_eq!(
        wording::could_not_check(&ServiceError::Unreadable("x".to_string())),
        "Couldn't check for updates: x. (Code U1)"
    );
}

#[test]
fn every_code_is_named_once() {
    let mut names: Vec<&str> = Code::ALL.iter().map(|code| code.name()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), Code::ALL.len());
}

#[test]
fn the_guide_explains_every_code_under_its_messages_opening_words() {
    let guide = guide();
    let section = troubleshooting(&guide);
    for code in Code::ALL {
        let entry = format!("- **{}: \"{}\"**", code.name(), code.opening());
        assert!(
            section.contains(&entry),
            "the guide's Troubleshooting section has no entry beginning {entry}"
        );
    }
}

#[test]
fn the_guide_lists_no_code_the_app_does_not_give() {
    let guide = guide();
    let listed: Vec<&str> = troubleshooting(&guide)
        .lines()
        .filter_map(|line| line.strip_prefix("- **")?.split_once(':'))
        .map(|(name, _)| name)
        .collect();
    let known: Vec<&str> = Code::ALL.iter().map(|code| code.name()).collect();
    assert_eq!(listed, known, "the guide's entries, in the codes' order");
}

#[test]
fn the_guide_shows_a_code_as_the_app_writes_it() {
    let guide = guide();
    assert!(troubleshooting(&guide).contains(&format!("\"{}\"", Code::U3)));
    assert!(guide.contains("- [Troubleshooting](#troubleshooting)"));
}
