// About (AboutDialog.cs and AboutText.cs in 0.1): the icon and text, and
// buttons, like a message box, so a screen reader reads the text as the
// dialog opens. A wx dialog is a real Windows dialog, so NVDA reads its
// static text on opening without the role 0.1 had to set. Short on
// purpose: each source asks for a credit, which this gives; the full
// wording, links and licences are in the guide's Credits and licences
// section.

use weatherspell_core::{environment_canada, nws, open_meteo};
use wxdragon::prelude::*;

use crate::{dialogs, guide};

const ID_CREDITS: Id = ID_HIGHEST + 1;
const ID_WEBSITE: Id = ID_HIGHEST + 2;

pub const WEBSITE: &str = "https://github.com/PlanetLinux98/weatherspell";
const LOGO: &[u8] = include_bytes!("../../../Assets/Weatherspell.svg");

fn paragraphs() -> Vec<String> {
    vec![
        // "Preview" until the switch from 0.1.
        format!(
            "Weatherspell Preview {}\nA text-based weather app for Windows.",
            crate::VERSION
        ),
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
    let dialog = build(parent);
    dialog.show_modal();
    dialog.destroy();
}

// Made and ready to show, focus in place (also for the lint's test).
pub fn build(parent: &dyn WxWidget) -> Dialog {
    let dialog = Dialog::builder(parent, "About Weatherspell").build();
    dialogs::developer_font(&dialog);
    let sizer = BoxSizer::builder(Orientation::Vertical).build();
    let top = BoxSizer::builder(Orientation::Horizontal).build();

    // Beside the text, as a message box has its icon: 48 pixels at 100
    // percent, drawn at the display's scale. Only a picture, which screen
    // readers pass over.
    if let Some(logo) = BitmapBundle::from_svg_data(LOGO, Size::new(48, 48)) {
        let picture = StaticBitmap::new_with_bitmap_bundle(&dialog, ID_ANY as Id, &logo);
        top.add(&picture, 0, SizerFlag::Left | SizerFlag::Top, 12);
    }
    let text = BoxSizer::builder(Orientation::Vertical).build();
    for paragraph in paragraphs() {
        let line = StaticText::builder(&dialog).with_label(&paragraph).build();
        dialogs::wrap(&line, 60);
        text.add(
            &line,
            0,
            SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
            12,
        );
    }
    top.add_sizer(&text, 1, SizerFlag::Expand, 0);
    sizer.add_sizer(&top, 0, SizerFlag::Expand, 0);

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

    credits.on_click(move |_| guide::open(&dialog, Some(guide::CREDITS)));
    website.on_click(move |_| open(&dialog, WEBSITE));

    // Focus on OK, as in a message box; Tab reaches the other buttons.
    ok.set_focus();
    dialog
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
