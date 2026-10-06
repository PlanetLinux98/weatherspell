// The user guide is built into the exe (one file, nothing beside it; see
// build/guide.rs) and written out to a temporary file when asked for, so
// it opens in whatever the user has chosen for web pages, works offline
// and always matches this version (UserGuide.cs in 0.1).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use wxdragon::prelude::*;

pub const CREDITS: &str = "credits-and-licences";

const FILE_NAME: &str = "user-guide.html";
// The guide and the third-party notices it links to, side by side.
const PAGES: [(&str, &str); 2] = [
    (
        FILE_NAME,
        include_str!(concat!(env!("OUT_DIR"), "/user-guide.html")),
    ),
    (
        "third-party-notices.html",
        include_str!(concat!(env!("OUT_DIR"), "/third-party-notices.html")),
    ),
];

fn folder() -> PathBuf {
    std::env::temp_dir().join("Weatherspell")
}

// The page to open. Opening a file drops any #fragment, so a section is
// reached through a small page that forwards to it.
fn prepare(folder: &Path, section: Option<&str>) -> io::Result<PathBuf> {
    fs::create_dir_all(folder)?;
    for (name, page) in PAGES {
        fs::write(folder.join(name), page)?;
    }
    let guide = folder.join(FILE_NAME);
    let Some(section) = section else {
        return Ok(guide);
    };
    let target = format!("{FILE_NAME}#{section}");
    let forward = folder.join(format!("user-guide-{section}.html"));
    fs::write(
        &forward,
        format!(
            "<!DOCTYPE html>\n<html lang=\"en-CA\">\n<head>\n<meta charset=\"utf-8\">\n\
             <meta http-equiv=\"refresh\" content=\"0; url={target}\">\n<title>Weatherspell User Guide</title>\n</head>\n\
             <body>\n<p><a href=\"{target}\">Weatherspell User Guide</a></p>\n</body>\n</html>\n"
        ),
    )?;
    Ok(forward)
}

// A failure says so, with the reason, so the guide can be found on GitHub
// instead.
pub fn open(owner: &dyn WxWidget, section: Option<&str>) {
    let problem = match prepare(&folder(), section) {
        Ok(page) => {
            if launch_default_application(&page.display().to_string()) {
                return;
            }
            format!("Couldn't open the user guide.\n\n{}", page.display())
        }
        Err(e) => format!("Couldn't open the user guide: {e}"),
    };
    MessageDialog::builder(owner, &problem, "Weatherspell User Guide")
        .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconWarning)
        .build()
        .show_modal();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_guide_opens_as_written_and_a_section_through_a_page_that_forwards_to_it() {
        let folder =
            std::env::temp_dir().join(format!("weatherspell-guide-test-{}", std::process::id()));
        let guide = prepare(&folder, None).unwrap();
        assert_eq!(guide, folder.join(FILE_NAME));
        for (name, page) in PAGES {
            assert_eq!(fs::read_to_string(folder.join(name)).unwrap(), page);
        }
        assert!(PAGES[0].1.contains(&format!("id=\"{CREDITS}\"")));
        assert!(PAGES[0].1.contains("href=\"third-party-notices.html\""));

        let forward = prepare(&folder, Some(CREDITS)).unwrap();
        assert_eq!(forward, folder.join("user-guide-credits-and-licences.html"));
        assert!(fs::read_to_string(&forward).unwrap().contains(
            "<meta http-equiv=\"refresh\" content=\"0; url=user-guide.html#credits-and-licences\">"
        ));
        let _ = fs::remove_dir_all(&folder);
    }
}
