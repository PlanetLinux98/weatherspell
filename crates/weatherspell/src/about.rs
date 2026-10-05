// About (AboutDialog.cs and AboutText.cs in 0.1): text and buttons, like a
// message box, so a screen reader reads the text as the dialog opens. A wx
// dialog is a real Windows dialog, so NVDA reads its static text on
// opening without the role 0.1 had to set. Short on purpose: each source
// asks for a credit, which this gives; the full wording, links and
// licences are in the guide's Credits and licences section.

use weatherspell_core::{environment_canada, nws, open_meteo};
use wxdragon::prelude::*;

use crate::dialogs;

const ID_CREDITS: Id = ID_HIGHEST + 1;
const ID_WEBSITE: Id = ID_HIGHEST + 2;

pub const WEBSITE: &str = "https://github.com/PlanetLinux98/weatherspell";
// Until the guide is built into the app (#24), its section on GitHub.
const CREDITS: &str =
    "https://github.com/PlanetLinux98/weatherspell/blob/main/USER_GUIDE.md#credits-and-licences";

fn paragraphs() -> Vec<String> {
    vec![
        // The version comes with the release build (#24); until then, the
        // preview says what it is.
        "Weatherspell Preview\nA text-based weather app for Windows.".to_string(),
        "Copyright 2026 PlanetLinux98. Released under the MIT licence.".to_string(),
        format!(
            "Weather data from {} (CC BY 4.0), {} and the {}.",
            open_meteo::SOURCE_NAME,
            environment_canada::FULL_NAME,
            nws::SOURCE_NAME
        ),
        "Place names from GeoNames (CC BY 4.0) and OpenStreetMap contributors (ODbL).".to_string(),
    ]
}

pub fn show(parent: &dyn WxWidget) {
    let dialog = Dialog::builder(parent, "About Weatherspell").build();
    let sizer = BoxSizer::builder(Orientation::Vertical).build();
    for paragraph in paragraphs() {
        let text = StaticText::builder(&dialog).with_label(&paragraph).build();
        dialogs::wrap(&text, 66);
        sizer.add(
            &text,
            0,
            SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
            12,
        );
    }
    let credits = Button::builder(&dialog)
        .with_id(ID_CREDITS)
        .with_label("&Credits and Licences")
        .build();
    let website = Button::builder(&dialog)
        .with_id(ID_WEBSITE)
        .with_label("&Website")
        .build();
    let ok = Button::builder(&dialog)
        .with_id(ID_OK)
        .with_label("OK")
        .build();
    ok.set_default();
    // Escape closes, as in a message box.
    dialog.set_escape_id(ID_OK);
    sizer.add_sizer(
        &dialogs::button_row(&[&credits, &website, &ok]),
        0,
        SizerFlag::Expand | SizerFlag::All,
        12,
    );
    dialog.set_sizer(sizer, true);
    dialogs::fit(&dialog, 71);

    credits.on_click(move |_| open(&dialog, CREDITS));
    website.on_click(move |_| open(&dialog, WEBSITE));

    // Focus on OK, as in a message box; Tab reaches the other buttons.
    ok.set_focus();
    dialog.show_modal();
    dialog.destroy();
}

// Pages open in the program the user has chosen for them. A failure says
// so and shows where it was going, so it can be reached by hand.
pub fn open(owner: &dyn WxWidget, url: &str) {
    if !launch_default_browser(url, BrowserLaunchFlags::Default) {
        MessageDialog::builder(
            owner,
            &format!("Couldn't open the browser.\n\n{url}"),
            "Weatherspell",
        )
        .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconWarning)
        .build()
        .show_modal();
    }
}
