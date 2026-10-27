// Windows' plain EDIT control, which the forecast, an alert's details and
// Hour by Hour are, where wxWidgets does not reach: its scroll position, a
// shorter context menu (with Hour by Hour on the forecast's), and the
// double-click. Positions here are the control's own, counting each line
// break as two ("\r\n"); the window converts them
// (SectionLayout::crlf_position). Elsewhere these do nothing yet.

use wxdragon::prelude::*;

// The view a rewrite keeps: the first line shown, and whether the caret
// was in sight (it is brought back into sight only if it was). Read on
// Windows only so far.
#[cfg_attr(not(windows), allow(dead_code))]
pub struct View {
    top: i32,
    caret_shown: bool,
}

#[cfg(windows)]
mod win {
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows::Win32::Graphics::Gdi::{ClientToScreen, ScreenToClient};
    use windows::Win32::UI::Controls::{EM_CHARFROMPOS, EM_GETSEL, EM_LINEINDEX, EM_SETSEL};
    use windows::Win32::UI::Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::*;
    use windows::core::w;

    pub fn hwnd(window: &impl wxdragon::prelude::WxWidget) -> HWND {
        HWND(window.get_handle())
    }

    pub fn send(hwnd: HWND, msg: u32, wparam: usize, lparam: isize) -> isize {
        unsafe { SendMessageW(hwnd, msg, Some(WPARAM(wparam)), Some(LPARAM(lparam))).0 }
    }

    pub fn selection(hwnd: HWND) -> (u32, u32) {
        let (mut start, mut end) = (0u32, 0u32);
        send(
            hwnd,
            EM_GETSEL,
            &mut start as *mut u32 as usize,
            &mut end as *mut u32 as isize,
        );
        (start, end)
    }

    const SUBCLASS_ID: usize = 0x5753; // "WS"
    const COPY: usize = 1;
    const SELECT_ALL: usize = 2;
    const HOUR_BY_HOUR: usize = 3;

    // The handlers a control was hooked with, owned by its subclass until
    // the control goes.
    struct Handlers {
        double_click: Option<fn(usize)>,
        hour_by_hour: Option<fn(usize)>,
    }

    pub fn hook(hwnd: HWND, double_click: Option<fn(usize)>, hour_by_hour: Option<fn(usize)>) {
        let data = Box::into_raw(Box::new(Handlers {
            double_click,
            hour_by_hour,
        })) as usize;
        unsafe {
            let _ = SetWindowSubclass(hwnd, Some(proc), SUBCLASS_ID, data);
        }
    }

    unsafe extern "system" fn proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        data: usize,
    ) -> LRESULT {
        unsafe {
            let handlers = &*(data as *const Handlers);
            match msg {
                WM_CONTEXTMENU => {
                    context_menu(hwnd, lparam, handlers.hour_by_hour);
                    LRESULT(0)
                }
                WM_LBUTTONDBLCLK if handlers.double_click.is_some() => {
                    // The control selects the word first; its start is
                    // where the click landed.
                    let result = DefSubclassProc(hwnd, msg, wparam, lparam);
                    if let Some(handler) = handlers.double_click {
                        handler(selection(hwnd).0 as usize);
                    }
                    result
                }
                WM_NCDESTROY => {
                    let _ = RemoveWindowSubclass(hwnd, Some(proc), SUBCLASS_ID);
                    drop(Box::from_raw(data as *mut Handlers));
                    DefSubclassProc(hwnd, msg, wparam, lparam)
                }
                _ => DefSubclassProc(hwnd, msg, wparam, lparam),
            }
        }
    }

    // Copy and Select All only, in place of the control's own menu, whose
    // Undo, Cut, Paste and Delete never apply to text that is only for
    // reading (Elliott, 2026-10-05), with Hour by Hour first on the
    // forecast's. A native menu, so it reads as any context menu does;
    // from the keyboard it opens at the caret, where a magnifier is
    // looking, and Hour by Hour opens at the caret's line; from the mouse,
    // at the line clicked.
    fn context_menu(hwnd: HWND, lparam: LPARAM, hour_by_hour: Option<fn(usize)>) {
        let mut at = POINT {
            x: (lparam.0 & 0xFFFF) as i16 as i32,
            y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32,
        };
        unsafe {
            let keyboard = lparam.0 as i32 == -1;
            if keyboard {
                at = POINT::default();
                let _ = GetCaretPos(&mut at);
                let _ = ClientToScreen(hwnd, &mut at);
            }
            let line_start = if keyboard {
                selection(hwnd).0 as usize
            } else {
                let mut client = at;
                let _ = ScreenToClient(hwnd, &mut client);
                let packed = (client.x as u16 as usize) | ((client.y as u16 as usize) << 16);
                // The line clicked, in the high word; its first character.
                let line = (send(hwnd, EM_CHARFROMPOS, 0, packed as isize) as usize >> 16) & 0xFFFF;
                send(hwnd, EM_LINEINDEX, line, 0).max(0) as usize
            };
            let Ok(menu) = CreatePopupMenu() else {
                return;
            };
            if hour_by_hour.is_some() {
                let _ = AppendMenuW(menu, MF_STRING, HOUR_BY_HOUR, w!("&Hour by Hour\tCtrl+H"));
                let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
            }
            let (start, end) = selection(hwnd);
            let copy = if start == end { MF_GRAYED } else { MF_ENABLED };
            let _ = AppendMenuW(menu, MF_STRING | copy, COPY, w!("&Copy\tCtrl+C"));
            let _ = AppendMenuW(menu, MF_STRING, SELECT_ALL, w!("Select &All\tCtrl+A"));
            let chosen = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                at.x,
                at.y,
                None,
                hwnd,
                None,
            );
            let _ = DestroyMenu(menu);
            match chosen.0 as usize {
                COPY => {
                    send(hwnd, WM_COPY, 0, 0);
                }
                SELECT_ALL => {
                    send(hwnd, EM_SETSEL, 0, -1);
                }
                HOUR_BY_HOUR => {
                    if let Some(handler) = hour_by_hour {
                        handler(line_start);
                    }
                }
                _ => {}
            }
        }
    }
}

// The shorter context menu, and the handlers if any: the double-click's is
// given the position of the word clicked, Hour by Hour's (a context menu
// item when given) the start of the line it was asked for on. Call once
// per control.
pub fn hook(
    text: &TextCtrl,
    on_double_click: Option<fn(usize)>,
    on_hour_by_hour: Option<fn(usize)>,
) {
    #[cfg(windows)]
    win::hook(win::hwnd(text), on_double_click, on_hour_by_hour);
    #[cfg(not(windows))]
    let _ = (text, on_double_click, on_hour_by_hour);
}

pub fn view(text: &TextCtrl) -> View {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Controls::{EM_GETFIRSTVISIBLELINE, EM_LINEFROMCHAR};
        let hwnd = win::hwnd(text);
        let top = win::send(hwnd, EM_GETFIRSTVISIBLELINE, 0, 0) as i32;
        let caret = win::selection(hwnd).0 as usize;
        let caret_line = win::send(hwnd, EM_LINEFROMCHAR, caret, 0) as i32;
        let lines = (text.get_client_size().height / text.get_char_height().max(1)).max(1);
        View {
            top,
            caret_shown: caret_line >= top && caret_line < top + lines,
        }
    }
    #[cfg(not(windows))]
    {
        let _ = text;
        View {
            top: 0,
            caret_shown: true,
        }
    }
}

// After the text and the caret are set: the same first line as before,
// and the caret in sight if it was.
pub fn restore(text: &TextCtrl, view: View) {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Controls::{EM_GETFIRSTVISIBLELINE, EM_LINESCROLL, EM_SCROLLCARET};
        let hwnd = win::hwnd(text);
        let scroll = view.top - win::send(hwnd, EM_GETFIRSTVISIBLELINE, 0, 0) as i32;
        if scroll != 0 {
            win::send(hwnd, EM_LINESCROLL, 0, scroll as isize);
        }
        if view.caret_shown {
            win::send(hwnd, EM_SCROLLCARET, 0, 0);
        }
    }
    #[cfg(not(windows))]
    let _ = (text, view);
}
