// Accessibility pieces for wxWidgets apps, made for Weatherspell and meant
// for other apps too (#24): what wxWidgets itself does not give screen
// readers. So far, announcements; the accessibility lint and the test
// tools are to follow.

mod announce;

pub use announce::Announcer;
