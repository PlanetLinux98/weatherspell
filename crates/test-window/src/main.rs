// A throwaway window that answers whether wxWidgets, through wxDragon,
// gives screen readers what Weatherspell needs before any porting starts
// (#24, step 1 of drafts/cross-platform-plan.md). It mirrors the C# main
// window: a Location box, the forecast as read-only text with the section
// keys, a native menu bar and a status bar, plus a Test menu for the
// experiments. Sample text only; nothing is fetched.
//
// Command line: --edit starts with a plain EDIT control instead of the
// RichEdit; --advertise starts with the window advertising its UI
// Automation provider (NVDA decides once per window whether it is a UI
// Automation window, so a fresh start is the fair comparison).

#![windows_subsystem = "windows"]

mod announce;
mod sample;

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use wxdragon::prelude::*;

use announce::Announcer;
use sample::{Layout, PLACES};

const ID_REFRESH: Id = ID_HIGHEST + 1;
const ID_MANAGE: Id = ID_HIGHEST + 2;
const ID_PLACE: Id = ID_HIGHEST + 10; // + 0..2
const ID_NEXT: Id = ID_HIGHEST + 20;
const ID_PREVIOUS: Id = ID_HIGHEST + 21;
const ID_ALERTS: Id = ID_HIGHEST + 22;
const ID_ANNOUNCE: Id = ID_HIGHEST + 30;
const ID_ANNOUNCE_LATER: Id = ID_HIGHEST + 31;
const ID_REFRESH_LATER: Id = ID_HIGHEST + 32;
const ID_PLAIN_EDIT: Id = ID_HIGHEST + 33;
const ID_ADVERTISE: Id = ID_HIGHEST + 34;
const ID_MESSAGE_ABOUT: Id = ID_HIGHEST + 36;
const ID_FONT_INFO: Id = ID_HIGHEST + 37;

struct App {
    frame: Frame,
    panel: Panel,
    body: BoxSizer,
    choice: Choice,
    text: RefCell<TextCtrl>,
    plain_edit: Cell<bool>,
    layout: RefCell<Layout>,
    place: Cell<usize>,
    refreshes: Cell<u32>,
    nicknames: RefCell<[Option<String>; 3]>,
    announcer: Announcer,
    fetch_timer: Timer<Frame>,
    later_timer: Timer<Frame>,
    later_refresh: Cell<bool>,
}

fn main() {
    let _ = wxdragon::main(|_| {
        let args: Vec<String> = std::env::args().collect();
        let app = build(args.iter().any(|a| a == "--edit"));
        if args.iter().any(|a| a == "--advertise") {
            app.announcer.advertise(true);
            app.frame
                .get_menu_bar()
                .unwrap()
                .check_item(ID_ADVERTISE, true);
        }
        app.show_place(0, false);
        app.frame.show(true);
    });
}

fn build(plain_edit: bool) -> Rc<App> {
    let frame = Frame::builder()
        .with_title("Weatherspell test window")
        .build();
    frame.set_menu_bar(menu_bar());
    frame.create_status_bar(1, 0, -1, "statusBar");

    let panel = Panel::builder(&frame).build();
    let outer = BoxSizer::builder(Orientation::Vertical).build();

    // Alt+O: Alt+L belongs to the Locations menu. The label comes just
    // before the box among the panel's children, which is what names it.
    let header = BoxSizer::builder(Orientation::Horizontal).build();
    let location_label = StaticText::builder(&panel).with_label("L&ocation").build();
    let names: Vec<String> = PLACES.iter().map(|p| p.to_string()).collect();
    let choice = Choice::builder(&panel).with_choices(names).build();
    choice.set_selection(0);
    header.add(
        &location_label,
        0,
        SizerFlag::AlignCenterVertical | SizerFlag::Right,
        8,
    );
    header.add(&choice, 1, SizerFlag::Expand, 0);
    outer.add_sizer(&header, 0, SizerFlag::Expand | SizerFlag::All, 8);

    let body = BoxSizer::builder(Orientation::Vertical).build();
    let forecast_label = StaticText::builder(&panel).with_label("Forecast").build();
    body.add(
        &forecast_label,
        0,
        SizerFlag::Left | SizerFlag::Right | SizerFlag::Bottom,
        8,
    );
    let text = forecast_box(&panel, plain_edit);
    body.add(&text, 1, SizerFlag::Expand, 0);
    outer.add_sizer(&body, 1, SizerFlag::Expand, 0);
    panel.set_sizer(outer, true);

    // Scale by font, as the C# app does: 103 by 37 characters is its
    // 720 by 560 design at Segoe UI 9 point.
    frame.set_size(Size::new(
        frame.get_char_width() * 103,
        frame.get_char_height() * 37,
    ));
    frame.centre();

    let menu_bar = frame.get_menu_bar().unwrap();
    menu_bar.check_item(ID_PLAIN_EDIT, plain_edit);

    let app = Rc::new(App {
        announcer: Announcer::new(frame.get_handle()),
        fetch_timer: Timer::new(&frame),
        later_timer: Timer::new(&frame),
        frame,
        panel,
        body,
        choice,
        text: RefCell::new(text),
        plain_edit: Cell::new(plain_edit),
        layout: RefCell::new(Layout {
            text: String::new(),
            headings: Vec::new(),
        }),
        place: Cell::new(0),
        refreshes: Cell::new(0),
        nicknames: RefCell::new([None, None, None]),
        later_refresh: Cell::new(false),
    });

    let a = app.clone();
    app.frame.on_menu(move |event| a.command(event.get_id()));
    let a = app.clone();
    app.choice.on_selection_changed(move |event| {
        if let Some(i) = event.get_selection() {
            a.show_place(i as usize, false);
        }
    });
    let a = app.clone();
    app.fetch_timer.on_tick(move |_| a.fetched());
    let a = app.clone();
    app.later_timer.on_tick(move |_| {
        if a.later_refresh.get() {
            a.refresh(true);
        } else {
            a.announcer
                .say("This announcement was made five seconds after you asked.");
        }
    });
    app
}

fn menu_bar() -> MenuBar {
    let file = Menu::builder()
        .append_item(ID_REFRESH, "&Refresh\tF5", "")
        .append_separator()
        .append_item(ID_EXIT, "E&xit\tAlt+F4", "")
        .build();
    let mut locations = Menu::builder()
        .append_item(ID_MANAGE, "&Rename Location...", "")
        .append_separator();
    for (i, place) in PLACES.iter().enumerate() {
        locations = locations.append_radio_item(
            ID_PLACE + i as Id,
            &place_item(i, sample::short(place)),
            "",
        );
    }
    let view = Menu::builder()
        .append_item(ID_NEXT, "&Next Section\tCtrl+PageDown", "")
        .append_item(ID_PREVIOUS, "&Previous Section\tCtrl+PageUp", "")
        .append_separator()
        .append_item(ID_ALERTS, "&Alerts\tCtrl+Shift+A", "")
        .build();
    let test = Menu::builder()
        .append_item(ID_ANNOUNCE, "&Announce\tCtrl+Shift+N", "")
        .append_item(ID_ANNOUNCE_LATER, "Announce in &5 Seconds", "")
        .append_item(ID_REFRESH_LATER, "&Refresh in 5 Seconds", "")
        .append_separator()
        .append_check_item(ID_PLAIN_EDIT, "&Plain Edit Control", "")
        .append_check_item(ID_ADVERTISE, "Ad&vertise UI Automation Provider", "")
        .append_separator()
        .append_item(ID_MESSAGE_ABOUT, "About as a &Message Box", "")
        .append_item(ID_FONT_INFO, "&Font and Scale", "")
        .build();
    let help = Menu::builder()
        .append_item(ID_ABOUT, "&About Weatherspell Test Window", "")
        .build();
    MenuBar::builder()
        .append(file, "&File")
        .append(locations.build(), "&Locations")
        .append(view, "&View")
        .append(test, "&Test")
        .append(help, "&Help")
        .build()
}

fn place_item(i: usize, name: &str) -> String {
    format!("&{} {}\tCtrl+{}", i + 1, name, i + 1)
}

// RichEdit (RICHEDIT50W, which wx picks for wxTE_RICH2) is what the C#
// app settled on for Narrator (#23); the plain EDIT is here to compare,
// since without WinForms in the way it may read just as well.
fn forecast_box(panel: &Panel, plain_edit: bool) -> TextCtrl {
    let mut style = TextCtrlStyle::MultiLine | TextCtrlStyle::ReadOnly | TextCtrlStyle::NoHideSel;
    if !plain_edit {
        style |= TextCtrlStyle::Rich2;
    }
    let text = TextCtrl::builder(panel).with_style(style).build();
    // Read-only text is grey by default; it is for reading.
    text.set_background_color(SystemSettings::get_colour(SystemColour::Window));
    text
}

impl App {
    fn command(&self, id: Id) {
        match id {
            ID_REFRESH => self.refresh(false),
            ID_EXIT => self.frame.close(false),
            ID_MANAGE => self.rename(),
            ID_NEXT => self.jump(1),
            ID_PREVIOUS => self.jump(-1),
            ID_ALERTS => {
                if let Some((heading, at)) = self.layout.borrow().headings.first().cloned() {
                    self.move_caret(&heading, at);
                }
            }
            ID_ANNOUNCE => self
                .announcer
                .say("This is a test announcement from the test window."),
            ID_ANNOUNCE_LATER => {
                self.later_refresh.set(false);
                self.later_timer.start(5000, true);
            }
            ID_REFRESH_LATER => {
                self.later_refresh.set(true);
                self.later_timer.start(5000, true);
            }
            ID_PLAIN_EDIT => self.swap_text_control(),
            ID_ADVERTISE => {
                self.announcer.advertise(!self.announcer.advertised());
                self.check(ID_ADVERTISE, self.announcer.advertised());
            }
            ID_MESSAGE_ABOUT => {
                MessageDialog::builder(&self.frame, ABOUT_TEXT, "About Weatherspell Test Window")
                    .with_style(MessageDialogStyle::OK | MessageDialogStyle::IconInformation)
                    .build()
                    .show_modal();
            }
            ID_FONT_INFO => {
                MessageDialog::builder(&self.frame, &self.font_info(), "Font and Scale")
                    .with_style(MessageDialogStyle::OK)
                    .build()
                    .show_modal();
            }
            ID_ABOUT => self.about(),
            _ if (ID_PLACE..ID_PLACE + PLACES.len() as Id).contains(&id) => {
                let i = (id - ID_PLACE) as usize;
                self.choice.set_selection(i as u32);
                self.show_place(i, true);
            }
            _ => {}
        }
    }

    fn check(&self, id: Id, on: bool) {
        if let Some(bar) = self.frame.get_menu_bar() {
            bar.check_item(id, on);
        }
    }

    fn name(&self, place: usize) -> String {
        self.nicknames.borrow()[place]
            .clone()
            .unwrap_or_else(|| sample::short(PLACES[place]).to_string())
    }

    // A launch or a switch: "Fetching..." first, then the forecast with a
    // spoken "forecast ready", as the C# app does. From Ctrl+1..3 the
    // place's name is spoken too, since focus may not be on the box.
    fn show_place(&self, place: usize, speak_name: bool) {
        self.place.set(place);
        self.check(ID_PLACE + place as Id, true);
        if speak_name {
            self.announcer.say(&self.name(place));
        }
        self.set_layout(Layout::fetching(PLACES[place]), 0);
        self.fetch_timer.start(700, true);
    }

    fn fetched(&self) {
        let place = self.place.get();
        self.set_layout(sample::forecast(place, self.refreshes.get()), 0);
        self.status();
        let alerts = sample::alerts_in_effect(place, self.refreshes.get());
        let mut line = format!("{}: forecast ready", self.name(place));
        if !alerts.is_empty() {
            line += &format!("; {} in effect", alerts.join(" and "));
        }
        self.announcer.say(&(line + "."));
    }

    // F5, or the timer's rewrite: the caret stays on the same words.
    fn refresh(&self, automatic: bool) {
        let place = self.place.get();
        let before = self.refreshes.get();
        self.refreshes.set(before + 1);
        let next = sample::forecast(place, before + 1);
        let caret = next.map_caret(&self.layout.borrow(), self.caret());
        self.set_layout(next, caret);
        self.status();
        let new_alert = place == 0 && before == 0;
        if new_alert {
            self.announcer.alert(&format!(
                "{}: frost advisory in effect from 11:00 pm tonight until 9:00 am tomorrow.",
                self.name(place)
            ));
        } else if !automatic {
            self.announcer.say("Forecast updated.");
        }
    }

    fn status(&self) {
        let minutes = 42 + 5 * self.refreshes.get();
        self.frame.set_status_text(
            &format!("Updated at {}:{:02} pm", 3 + minutes / 60, minutes % 60),
            0,
        );
    }

    fn set_layout(&self, layout: Layout, caret: i64) {
        let text = self.text.borrow();
        text.change_value(&layout.text);
        *self.layout.borrow_mut() = layout;
        drop(text);
        self.set_caret(caret);
    }

    // Positions in our units: UTF-16, "\n" as one. A plain EDIT counts
    // each line break as two ("\r\n"), so its positions are converted.
    fn caret(&self) -> i64 {
        let native = self.text.borrow().get_insertion_point();
        if !self.plain_edit.get() {
            return native;
        }
        let mut position = 0;
        let mut counted = 0;
        for unit in self.layout.borrow().text.encode_utf16() {
            let width = if unit == '\n' as u16 { 2 } else { 1 };
            if counted + width > native {
                break;
            }
            counted += width;
            position += 1;
        }
        position
    }

    fn set_caret(&self, position: i64) {
        let native = if self.plain_edit.get() {
            let breaks = self
                .layout
                .borrow()
                .text
                .encode_utf16()
                .take(position as usize)
                .filter(|u| *u == '\n' as u16)
                .count();
            position + breaks as i64
        } else {
            position
        };
        self.text.borrow().set_insertion_point(native);
    }

    fn jump(&self, direction: i32) {
        let here = self.caret();
        let target = {
            let layout = self.layout.borrow();
            if direction > 0 {
                layout.headings.iter().find(|(_, at)| *at > here).cloned()
            } else {
                layout
                    .headings
                    .iter()
                    .rev()
                    .find(|(_, at)| *at < here)
                    .cloned()
            }
        };
        match target {
            Some((heading, at)) => self.move_caret(&heading, at),
            None => self.announcer.say(if direction > 0 {
                "No next section."
            } else {
                "No previous section."
            }),
        }
    }

    // NVDA reads the new line only after the navigation keys it knows, so
    // the heading is spoken (#17); when the jump brings focus into the
    // text, NVDA reads the line itself as focus arrives.
    fn move_caret(&self, heading: &str, at: i64) {
        let speak = self.text.borrow().has_focus();
        self.text.borrow().set_focus();
        self.set_caret(at);
        if speak {
            self.announcer.say(heading);
        }
    }

    fn swap_text_control(&self) {
        let caret = self.caret();
        let plain = !self.plain_edit.get();
        let had_focus = self.text.borrow().has_focus();
        // A destroyed window leaves its sizer, so the new box takes the
        // old one's place at the end; it is also the panel's last child,
        // right after its label, as before.
        self.text.borrow().destroy();
        let text = forecast_box(&self.panel, plain);
        self.body.add(&text, 1, SizerFlag::Expand, 0);
        *self.text.borrow_mut() = text;
        self.plain_edit.set(plain);
        self.check(ID_PLAIN_EDIT, plain);
        let layout = std::mem::replace(
            &mut *self.layout.borrow_mut(),
            Layout {
                text: String::new(),
                headings: Vec::new(),
            },
        );
        self.set_layout(layout, caret);
        self.panel.layout();
        if had_focus {
            self.text.borrow().set_focus();
        }
        self.frame.set_status_text(
            if plain {
                "Plain EDIT control"
            } else {
                "RichEdit control (RICHEDIT50W)"
            },
            0,
        );
    }

    // A label just before an input names it (check 6); OK is the default
    // button and Escape cancels (check 8).
    fn rename(&self) {
        let place = self.place.get();
        let dialog = Dialog::builder(&self.frame, "Rename Location").build();
        let sizer = BoxSizer::builder(Orientation::Vertical).build();
        let label = StaticText::builder(&dialog)
            .with_label(&format!("&Nickname for {}:", PLACES[place]))
            .build();
        let field = TextCtrl::builder(&dialog)
            .with_value(&self.name(place))
            .build();
        let notify = CheckBox::builder(&dialog)
            .with_label("Notify me about &alerts")
            .build();
        notify.set_value(true);
        let ok = Button::builder(&dialog)
            .with_id(ID_OK)
            .with_label("OK")
            .build();
        let cancel = Button::builder(&dialog)
            .with_id(ID_CANCEL)
            .with_label("Cancel")
            .build();
        let buttons = StdDialogButtonSizerBuilder::new().build();
        buttons.add_button(&ok);
        buttons.add_button(&cancel);
        buttons.realize();
        ok.set_default();
        sizer.add(
            &label,
            0,
            SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
            12,
        );
        sizer.add(&field, 0, SizerFlag::Expand | SizerFlag::All, 12);
        sizer.add(
            &notify,
            0,
            SizerFlag::Left | SizerFlag::Right | SizerFlag::Bottom,
            12,
        );
        sizer.add_sizer(&buttons, 0, SizerFlag::Expand | SizerFlag::All, 12);
        dialog.set_sizer_and_fit(sizer, true);
        dialog.centre();
        field.set_focus();
        field.select_all();
        if dialog.show_modal() == ID_OK {
            let nickname = field.get_value().trim().to_string();
            self.nicknames.borrow_mut()[place] = if nickname.is_empty() {
                None
            } else {
                Some(nickname)
            };
            let name = self.name(place);
            if let Some(item) = self
                .frame
                .get_menu_bar()
                .and_then(|b| b.find_item(ID_PLACE + place as Id))
            {
                item.set_label(&place_item(place, &name));
            }
            self.choice.clear();
            for (place, nickname) in PLACES.iter().zip(self.nicknames.borrow().iter()) {
                match nickname {
                    Some(nick) => self.choice.append(&format!("{nick} ({place})")),
                    None => self.choice.append(place),
                }
            }
            self.choice.set_selection(place as u32);
        }
        dialog.destroy();
    }

    // Read aloud on opening like a message box (check 8): static text and
    // an OK button. A wx dialog is a real Windows dialog (class #32770),
    // so it is a dialog to MSAA and UI Automation without the role the C#
    // About had to set. wxDragon's role setter must not be used: its
    // replacement accessible object leaves the dialog's children unnamed.
    fn about(&self) {
        let dialog = Dialog::builder(&self.frame, "About Weatherspell Test Window").build();
        let sizer = BoxSizer::builder(Orientation::Vertical).build();
        for paragraph in ABOUT_TEXT.split("\n\n") {
            let line = StaticText::builder(&dialog).with_label(paragraph).build();
            line.wrap(dialog.get_char_width() * 50);
            sizer.add(
                &line,
                0,
                SizerFlag::Left | SizerFlag::Right | SizerFlag::Top,
                12,
            );
        }
        let ok = Button::builder(&dialog)
            .with_id(ID_OK)
            .with_label("OK")
            .build();
        ok.set_default();
        sizer.add(&ok, 0, SizerFlag::AlignRight | SizerFlag::All, 12);
        dialog.set_sizer_and_fit(sizer, true);
        dialog.centre();
        ok.set_focus();
        dialog.show_modal();
        dialog.destroy();
    }

    fn font_info(&self) -> String {
        let font = self.frame.get_font();
        let (face, points) = font.as_ref().map_or((String::new(), 0), |f| {
            (f.get_face_name(), f.get_point_size())
        });
        let mut info = format!(
            "Window font: {face}, {points} point.\nAverage character: {} by {} pixels.",
            self.frame.get_char_width(),
            self.frame.get_char_height()
        );
        #[cfg(windows)]
        info.push_str(&system_font_info(self.frame.get_handle()));
        info
    }
}

// What Windows itself says, to compare with wx's choice of font: the
// message font follows the Text size setting, the DPI the display scale.
#[cfg(windows)]
fn system_font_info(hwnd: *mut std::ffi::c_void) -> String {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::HiDpi::{GetDpiForWindow, SystemParametersInfoForDpi};
    use windows::Win32::UI::WindowsAndMessaging::{NONCLIENTMETRICSW, SPI_GETNONCLIENTMETRICS};
    unsafe {
        let dpi = GetDpiForWindow(HWND(hwnd));
        let mut metrics = NONCLIENTMETRICSW {
            cbSize: size_of::<NONCLIENTMETRICSW>() as u32,
            ..Default::default()
        };
        let ok = SystemParametersInfoForDpi(
            SPI_GETNONCLIENTMETRICS.0,
            metrics.cbSize,
            Some(&mut metrics as *mut _ as *mut _),
            0,
            dpi,
        );
        let face = String::from_utf16_lossy(&metrics.lfMessageFont.lfFaceName)
            .trim_end_matches('\0')
            .to_string();
        let pixels = metrics.lfMessageFont.lfHeight.abs();
        format!(
            "\nWindow DPI: {dpi} ({} percent).\nWindows message font: {face}, {pixels} pixels high ({:.1} point){}.",
            dpi * 100 / 96,
            pixels as f64 * 72.0 / dpi as f64,
            if ok.is_ok() { "" } else { ", not read" }
        )
    }
}

const ABOUT_TEXT: &str = "Weatherspell test window\n\n\
A throwaway build that checks what wxWidgets gives screen readers before Weatherspell moves to it. The forecasts are samples.\n\n\
Built with wxWidgets 3.3 through wxDragon 0.9.";
