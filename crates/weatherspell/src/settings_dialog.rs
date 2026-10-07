// Settings (SettingsDialog.cs in 0.1): two groups of choices. OK applies
// and closes, Apply applies and stays, Cancel closes and keeps whatever
// Apply already did, as Windows dialogs do. The lists and their wording
// are the core's (settings::choices); this only moves values between them
// and the controls.

use weatherspell_core::settings::{Announcements, AppSettings, choices};
use wxdragon::prelude::*;

use crate::dialogs;

const ID_APPLY: Id = ID_HIGHEST + 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chosen {
    pub forecast_minutes: u32,
    pub alert_minutes: u32,
    pub announcements: Announcements,
}

// apply is called for OK and for each Apply.
pub fn show(parent: &dyn WxWidget, settings: &AppSettings, apply: impl Fn(Chosen) + 'static) {
    let dialog = build(parent, settings, apply);
    dialog.show_modal();
    dialog.destroy();
}

// Made and ready to show, focus in place (also for the lint's test).
pub fn build(
    parent: &dyn WxWidget,
    settings: &AppSettings,
    apply: impl Fn(Chosen) + 'static,
) -> Dialog {
    let dialog = Dialog::builder(parent, "Settings").build();
    dialogs::app_font(&dialog);
    let sizer = BoxSizer::builder(Orientation::Vertical).build();

    let forecast_choices = choices::minutes(
        &choices::FORECAST_MINUTES,
        settings.forecast_refresh_minutes,
    );
    let alert_choices = choices::minutes(&choices::ALERT_MINUTES, settings.alert_check_minutes);

    // Each group box holds its controls, as wxWidgets asks and as 0.1's
    // WinForms group boxes did, so a screen reader names the group as focus
    // enters it.
    let forecast = group(&dialog, "Forecast");
    let forecast_minutes = row(
        &forecast.2,
        &forecast.1,
        "&Refresh the forecast every",
        forecast_choices.iter().map(|m| choices::minutes_label(*m)),
        forecast_choices
            .iter()
            .position(|m| *m == settings.forecast_refresh_minutes),
    );
    sizer.add_sizer(
        &forecast.0,
        0,
        SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
        10,
    );

    let alerts = group(&dialog, "Alerts and announcements");
    let alert_minutes = row(
        &alerts.2,
        &alerts.1,
        "&Check for alerts every",
        alert_choices.iter().map(|m| choices::minutes_label(*m)),
        alert_choices
            .iter()
            .position(|m| *m == settings.alert_check_minutes),
    );
    let announcements = row(
        &alerts.2,
        &alerts.1,
        "Announce &new alerts",
        choices::ANNOUNCEMENTS
            .iter()
            .map(|(_, label)| label.to_string()),
        choices::ANNOUNCEMENTS
            .iter()
            .position(|(value, _)| *value == settings.alert_announcements),
    );
    sizer.add_sizer(&alerts.0, 0, SizerFlag::Expand | SizerFlag::All, 10);

    let ok = Button::builder(&dialog)
        .with_id(ID_OK)
        .with_label("OK")
        .build();
    let cancel = Button::builder(&dialog)
        .with_id(ID_CANCEL)
        .with_label("Cancel")
        .build();
    let apply_button = Button::builder(&dialog)
        .with_id(ID_APPLY)
        .with_label("&Apply")
        .build();
    ok.set_default();
    sizer.add_sizer(
        &dialogs::button_row(&[&ok, &cancel, &apply_button]),
        0,
        SizerFlag::Expand | SizerFlag::Left | SizerFlag::Right | SizerFlag::Bottom,
        10,
    );
    dialog.set_sizer(sizer, true);
    wx_accessibility::name_inputs(&dialog);
    // Fixed: nothing here gains from more room. 0.1's 480 wide.
    dialogs::fit(&dialog, 68);

    let chosen = move || Chosen {
        forecast_minutes: forecast_choices[forecast_minutes.get_selection().unwrap_or(0) as usize],
        alert_minutes: alert_choices[alert_minutes.get_selection().unwrap_or(0) as usize],
        announcements: choices::ANNOUNCEMENTS[announcements.get_selection().unwrap_or(0) as usize]
            .0,
    };
    let apply = std::rc::Rc::new(apply);
    let (a, c) = (apply.clone(), chosen.clone());
    apply_button.on_click(move |_| a(c()));
    let (a, c) = (apply.clone(), chosen.clone());
    ok.on_click(move |_| {
        a(c());
        dialogs::end(&dialog, ID_OK);
    });

    forecast_minutes.set_focus();
    dialog
}

// A labelled group box and the two-column grid inside it: labels as wide
// as the widest, the choices taking the rest.
fn group(dialog: &Dialog, title: &str) -> (StaticBoxSizer, FlexGridSizer, StaticBox) {
    let boxed = StaticBoxSizerBuilder::new_with_label(Orientation::Vertical, dialog, title).build();
    let grid = FlexGridSizer::builder(0, 2)
        .with_vgap(6)
        .with_hgap(8)
        .build();
    grid.add_growable_col(1, 1);
    boxed.add_sizer(&grid, 1, SizerFlag::Expand | SizerFlag::All, 6);
    let static_box = boxed.get_static_box().expect("the sizer made its box");
    let room = crate::system::scale_box_title(&static_box, dialogs::chosen_percent());
    if room > 0 {
        boxed.add_spacer(room);
    }
    (boxed, grid, static_box)
}

// The label just before its choice, which is what names the choice.
fn row(
    parent: &StaticBox,
    grid: &FlexGridSizer,
    label: &str,
    items: impl Iterator<Item = String>,
    selected: Option<usize>,
) -> Choice {
    let text = StaticText::builder(parent).with_label(label).build();
    let choice = Choice::builder(parent).build();
    for item in items {
        choice.append(&item);
    }
    choice.set_selection(selected.unwrap_or(0) as u32);
    grid.add(&text, 0, SizerFlag::AlignCenterVertical, 0);
    grid.add(&choice, 1, SizerFlag::Expand, 0);
    choice
}
