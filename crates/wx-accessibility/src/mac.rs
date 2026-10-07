// The few Objective-C calls the Mac needs (AppKit's accessibility), made
// straight through objc_msgSend rather than with a binding crate: a
// handful of messages do not justify one. Each helper casts objc_msgSend
// to the method's own signature, which arm64 requires (it is not a
// variadic call there). Main thread only, as AppKit is; whatever these
// return is autoreleased, kept alive by the event loop's pool. That, and
// a receiver that is a live object answering the message, is the safety
// rule for every helper here.
#![allow(clippy::missing_safety_doc)]

use std::ffi::{CStr, c_char, c_void};

pub type Id = *mut c_void;
type Sel = *const c_void;

#[link(name = "objc")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> Id;
    fn sel_registerName(name: *const c_char) -> Sel;
    fn objc_msgSend();
}

// AppKit itself is linked by wxWidgets.
#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    pub static NSAccessibilityAnnouncementRequestedNotification: Id;
    pub static NSAccessibilityAnnouncementKey: Id;
    pub static NSAccessibilityPriorityKey: Id;
    pub fn NSAccessibilityPostNotificationWithUserInfo(element: Id, notification: Id, info: Id);
}

pub fn class(name: &CStr) -> Id {
    unsafe { objc_getClass(name.as_ptr()) }
}

fn sel(name: &CStr) -> Sel {
    unsafe { sel_registerName(name.as_ptr()) }
}

// objc_msgSend as a function of the given signature.
macro_rules! send {
    ($ty:ty) => {
        unsafe { std::mem::transmute::<unsafe extern "C" fn(), $ty>(objc_msgSend) }
    };
}

// [receiver selector]
pub unsafe fn id(receiver: Id, selector: &CStr) -> Id {
    let f = send!(unsafe extern "C" fn(Id, Sel) -> Id);
    unsafe { f(receiver, sel(selector)) }
}

// [receiver selector:object], for an object or a C string.
pub unsafe fn id_with(receiver: Id, selector: &CStr, argument: *const c_void) -> Id {
    let f = send!(unsafe extern "C" fn(Id, Sel, *const c_void) -> Id);
    unsafe { f(receiver, sel(selector), argument) }
}

// [receiver selector:number], for an NSInteger or NSUInteger.
pub unsafe fn id_with_number(receiver: Id, selector: &CStr, argument: isize) -> Id {
    let f = send!(unsafe extern "C" fn(Id, Sel, isize) -> Id);
    unsafe { f(receiver, sel(selector), argument) }
}

// A BOOL answer: [receiver selector] or [receiver selector:object].
pub unsafe fn yes(receiver: Id, selector: &CStr) -> bool {
    let f = send!(unsafe extern "C" fn(Id, Sel) -> bool);
    unsafe { f(receiver, sel(selector)) }
}

pub unsafe fn yes_with(receiver: Id, selector: &CStr, argument: Id) -> bool {
    let f = send!(unsafe extern "C" fn(Id, Sel, Id) -> bool);
    unsafe { f(receiver, sel(selector), argument) }
}

// [NSArray count]
pub unsafe fn count(array: Id) -> usize {
    let f = send!(unsafe extern "C" fn(Id, Sel) -> usize);
    unsafe { f(array, sel(c"count")) }
}

// [NSDictionary dictionaryWithObjects:forKeys:count:]
pub unsafe fn dictionary(objects: &[Id], keys: &[Id]) -> Id {
    let f = send!(unsafe extern "C" fn(Id, Sel, *const Id, *const Id, usize) -> Id);
    let count = objects.len().min(keys.len());
    unsafe {
        f(
            class(c"NSDictionary"),
            sel(c"dictionaryWithObjects:forKeys:count:"),
            objects.as_ptr(),
            keys.as_ptr(),
            count,
        )
    }
}

// An NSString of the text (a NUL inside it ends it early).
pub unsafe fn string(text: &str) -> Id {
    let text = std::ffi::CString::new(text.replace('\0', "")).unwrap_or_default();
    unsafe {
        id_with(
            class(c"NSString"),
            c"stringWithUTF8String:",
            text.as_ptr().cast(),
        )
    }
}

// An NSString's text, or nothing for nil.
pub unsafe fn text(string: Id) -> String {
    if string.is_null() {
        return String::new();
    }
    let f = send!(unsafe extern "C" fn(Id, Sel) -> *const c_char);
    let chars = unsafe { f(string, sel(c"UTF8String")) };
    if chars.is_null() {
        String::new()
    } else {
        unsafe { CStr::from_ptr(chars) }
            .to_string_lossy()
            .into_owned()
    }
}

// [object isKindOfClass:NSClassName]
pub unsafe fn is(object: Id, class_name: &CStr) -> bool {
    let class = class(class_name);
    !object.is_null() && !class.is_null() && unsafe { yes_with(object, c"isKindOfClass:", class) }
}
