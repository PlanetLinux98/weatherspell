// Accessibility pieces for wxWidgets apps, made for Weatherspell and meant
// for other apps too (#24): what wxWidgets itself does not give screen
// readers: announcements, which screen readers are running, and a lint
// for the mechanical rules; the test tools are binaries (src/bin).

mod announce;
pub mod lint;
mod screen_readers;

pub use announce::Announcer;
pub use screen_readers::ScreenReaders;
