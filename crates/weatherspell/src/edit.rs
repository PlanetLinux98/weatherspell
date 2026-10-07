// Windows' plain EDIT control, which the forecast and an alert's details
// are, where wxWidgets does not reach: its scroll position, a shorter
// context menu, and the double-click. Positions here are the control's
// own, counting each line break as two ("\r\n"); the window converts them
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
    use windows::Win32::Graphics::Gdi::ClientToScreen;
    use windows::Win32::UI::Controls::{EM_GETSEL, EM_SETSEL};
    use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
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

    // data is the double-click handler, or 0.
    pub fn hook(hwnd: HWND, on_double_click: Option<fn(usize)>) {
        let data = on_double_click.map_or(0, |f| f as usize);
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
            match msg {
                WM_CONTEXTMENU => {
                    context_menu(hwnd, lparam);
                    LRESULT(0)
                }
                WM_LBUTTONDBLCLK if data != 0 => {
                    // The control selects the word first; its start is
                    // where the click landed.
                    let result = DefSubclassProc(hwnd, msg, wparam, lparam);
                    let handler: fn(usize) = std::mem::transmute(data as *const ());
                    handler(selection(hwnd).0 as usize);
                    result
                }
                _ => DefSubclassProc(hwnd, msg, wparam, lparam),
            }
        }
    }

    // Copy and Select All only, in place of the control's own menu, whose
    // Undo, Cut, Paste and Delete never apply to text that is only for
    // reading (Elliott, 2026-10-05). A native menu, so it reads as any
    // context menu does; from the keyboard it opens at the caret, where a
    // magnifier is looking.
    fn context_menu(hwnd: HWND, lparam: LPARAM) {
        let mut at = POINT {
            x: (lparam.0 & 0xFFFF) as i16 as i32,
            y: ((lparam.0 >> 16) & 0xFFFF) as i16 as i32,
        };
        unsafe {
            if lparam.0 as i32 == -1 {
                at = POINT::default();
                let _ = GetCaretPos(&mut at);
                let _ = ClientToScreen(hwnd, &mut at);
            }
            let Ok(menu) = CreatePopupMenu() else {
                return;
            };
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
                _ => {}
            }
        }
    }
}

// The shorter context menu, and the double-click handler if any, which is
// given the position of the word clicked. Call once per control.
pub fn hook(text: &TextCtrl, on_double_click: Option<fn(usize)>) {
    #[cfg(windows)]
    win::hook(win::hwnd(text), on_double_click);
    #[cfg(not(windows))]
    let _ = (text, on_double_click);
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
