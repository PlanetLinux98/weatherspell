// Accessibility pieces for wxWidgets apps, made for Weatherspell and meant
// for other apps too (#24): what wxWidgets itself does not give screen
// readers. So far, announcements and which screen readers are running;
// the accessibility lint and the test tools are to follow.

mod announce;
mod screen_readers;

pub use announce::Announcer;
pub use screen_readers::ScreenReaders;
