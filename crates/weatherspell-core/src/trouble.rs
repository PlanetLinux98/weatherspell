// The short codes that end the app's failure messages, "(Code U3)", and
// the words the user guide's Troubleshooting section lists each under
// (Elliott's idea, 2026-11-10: a code to look up and to quote in a
// report). A letter for the area and a number: short to say and to hear.
//
// A code goes at the end, so the sentence is heard first, and only on a
// message that stops something the user asked for: message boxes, the
// Problem section, the status bar's failures, Add Location's search.
// Not on announcements, and not on the forecast's own lines that say what
// was done instead ("Alerts couldn't be checked this time ..."), where it
// would be read on every pass through the text.
//
// tests/trouble.rs holds the guide to this list, both ways.

use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Code {
    // Check for Updates (updates.rs).
    U1,
    U2,
    U3,
    U4,
    U5,
    U6,
    // The forecast.
    F1,
    F2,
    // Add Location's search.
    L1,
    L2,
    // Settings and the app's own files.
    S1,
    S2,
    S3,
    // Opening a page.
    H1,
    H2,
}

impl Code {
    pub const ALL: [Code; 15] = [
        Code::U1,
        Code::U2,
        Code::U3,
        Code::U4,
        Code::U5,
        Code::U6,
        Code::F1,
        Code::F2,
        Code::L1,
        Code::L2,
        Code::S1,
        Code::S2,
        Code::S3,
        Code::H1,
        Code::H2,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Code::U1 => "U1",
            Code::U2 => "U2",
            Code::U3 => "U3",
            Code::U4 => "U4",
            Code::U5 => "U5",
            Code::U6 => "U6",
            Code::F1 => "F1",
            Code::F2 => "F2",
            Code::L1 => "L1",
            Code::L2 => "L2",
            Code::S1 => "S1",
            Code::S2 => "S2",
            Code::S3 => "S3",
            Code::H1 => "H1",
            Code::H2 => "H2",
        }
    }

    // How its message begins: what the guide's entry quotes, and what the
    // message itself is built from wherever it is one fixed sentence.
    pub fn opening(self) -> &'static str {
        match self {
            Code::U1 => "Couldn't check for updates",
            Code::U2 => "This version was published without a checksum",
            Code::U3 => "The download didn't match its checksum",
            Code::U4 => "Couldn't download the update",
            Code::U5 => "Couldn't save the download",
            Code::U6 => "Nothing was installed",
            Code::F1 => "Couldn't fetch the forecast",
            Code::F2 => "Something went wrong",
            Code::L1 => "Couldn't search",
            Code::L2 => "Couldn't look up those coordinates",
            Code::S1 => "Weatherspell couldn't read its settings",
            Code::S2 => "Couldn't save settings",
            Code::S3 => "Couldn't keep the forecast for offline use",
            Code::H1 => "Couldn't open the browser",
            Code::H2 => "Couldn't open the user guide",
        }
    }
}

// "(Code U3)": what follows the message, after a space.
impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "(Code {})", self.name())
    }
}
