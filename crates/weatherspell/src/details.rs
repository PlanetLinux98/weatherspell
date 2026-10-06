// One alert's full text (AlertDialog.cs in 0.1), read-only, so it can be
// read line by line, selected and copied like the forecast. Enter or
// Escape closes; the official page opens in the browser.

use wxdragon::prelude::*;

use crate::{dialogs, edit};

const ID_OFFICIAL: Id = ID_HIGHEST + 1;

pub fn show(parent: &Frame, title: &str, paragraphs: &[String], url: Option<&str>) {
    let dialog = build(parent, title, paragraphs, url);
    dialog.show_modal();
    dialog.destroy();
}

// Made and ready to show, focus in place (also for the lint's test).
pub fn build(parent: &Frame, title: &str, paragraphs: &[String], url: Option<&str>) -> Dialog {
    let dialog = Dialog::builder(parent, title)
        .with_style(
            DialogStyle::DefaultDialogStyle | DialogStyle::ResizeBorder | DialogStyle::MaximizeBox,
        )
        .build();
    dialogs::developer_font(&dialog);
    let sizer = BoxSizer::builder(Orientation::Vertical).build();

    // The label just before the text among the dialog's children names it.
    let label = StaticText::builder(&dialog).with_label("&Details").build();
    let text = TextCtrl::builder(&dialog)
        .with_style(TextCtrlStyle::MultiLine | TextCtrlStyle::ReadOnly | TextCtrlStyle::NoHideSel)
        .build();
    // Read-only text is grey by default; it is for reading.
    text.set_background_color(SystemSettings::get_colour(SystemColour::Window));
    text.change_value(&paragraphs.join("\n\n"));
    edit::hook(&text, None);

    let official = Button::builder(&dialog)
        .with_id(ID_OFFICIAL)
        .with_label("&Official page")
        .build();
    official.enable(url.is_some());
    let close = Button::builder(&dialog)
        .with_id(ID_CANCEL)
        .with_label("Close")
        .build();
    close.set_default();

    sizer.add(
        &label,
        0,
        SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
        8,
    );
    sizer.add(&text, 1, SizerFlag::Expand | SizerFlag::All, 8);
    let buttons = BoxSizer::builder(Orientation::Horizontal).build();
    buttons.add(&official, 0, SizerFlag::Right, 8);
    buttons.add(&close, 0, SizerFlag::empty(), 0);
    sizer.add_sizer(
        &buttons,
        0,
        SizerFlag::AlignRight | SizerFlag::Left | SizerFlag::Right | SizerFlag::Bottom,
        8,
    );
    dialog.set_sizer(sizer, true);

    // 0.1's 600 by 460 at Segoe UI 9 point, in characters, within the
    // screen; at least 400 by 300.
    let (w, h) = (dialog.get_char_width(), dialog.get_char_height());
    let mut size = Size::new(w * 86, h * 31);
    if let Some(display) = Display::from_window(parent) {
        let area = display.client_area();
        size = Size::new(size.width.min(area.width), size.height.min(area.height));
    }
    dialog.set_min_size(Size::new(w * 57, h * 20));
    dialog.set_size(size);
    dialog.centre();

    // A multi-line text box keeps Enter for itself, so it closes here.
    let d = dialog;
    text.on_key_down(move |event| {
        if let WindowEventData::Keyboard(key) = &event
            && matches!(key.get_key_code(), Some(WXK_RETURN | WXK_NUMPAD_ENTER))
            && !key.control_down()
            && !key.shift_down()
            && !key.alt_down()
        {
            d.end_modal(ID_CANCEL);
            return;
        }
        event.skip(true);
    });
    let target = url.map(str::to_string);
    let owner = dialog;
    let caption = title.to_string();
    official.on_click(move |_| {
        if let Some(target) = &target
            && !launch_default_browser(target, BrowserLaunchFlags::Default)
        {
            // Where it was going, so it can be reached by hand.
            MessageDialog::builder(
                &owner,
                &format!("Couldn't open the browser.\n\n{target}"),
                &caption,
            )
            .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconWarning)
            .build()
            .show_modal();
        }
    });

    // Focus in the text, caret at the top, nothing selected.
    text.set_focus();
    text.set_selection(0, 0);
    dialog
}
