// A UI Automation provider of our own that answers nothing but its window:
// UI Automation fills in everything else from the window itself
// (UiaHostProviderFromHwnd), so an event raised on it comes from that
// window's element as screen readers already know it. wxWidgets has no UI
// Automation at all; this is how events it never raises get raised.
//
// The provider is never handed out on WM_GETOBJECT. A window that
// advertises one makes NVDA read it through UI Automation instead of MSAA,
// which the test window showed reads worse (NOTES.md, "Rust and
// wxWidgets").

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Accessibility::*;
use windows::core::{Error, IUnknown, Result, implement};

#[implement(IRawElementProviderSimple)]
struct Source {
    hwnd: HWND,
}

impl IRawElementProviderSimple_Impl for Source_Impl {
    fn ProviderOptions(&self) -> Result<ProviderOptions> {
        Ok(ProviderOptions_ServerSideProvider)
    }

    // S_OK with no object: no patterns.
    fn GetPatternProvider(&self, _: UIA_PATTERN_ID) -> Result<IUnknown> {
        Err(Error::empty())
    }

    // An empty VARIANT: every property comes from the host window.
    fn GetPropertyValue(&self, _: UIA_PROPERTY_ID) -> Result<VARIANT> {
        Ok(VARIANT::default())
    }

    fn HostRawElementProvider(&self) -> Result<IRawElementProviderSimple> {
        unsafe { UiaHostProviderFromHwnd(self.hwnd) }
    }
}

pub fn provider(hwnd: *mut std::ffi::c_void) -> IRawElementProviderSimple {
    Source { hwnd: HWND(hwnd) }.into()
}
