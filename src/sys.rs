//! The macOS C interfaces Hop needs, declared by hand: CoreFoundation, CoreGraphics,
//! SkyLight (private), Accessibility, Carbon hot keys, libdispatch and the Objective-C runtime.
#![allow(non_upper_case_globals, non_snake_case, dead_code)]

use std::ffi::{c_char, c_void, CStr};

pub type CFTypeRef = *const c_void;
pub type CFStringRef = *const c_void;
pub type CFArrayRef = *const c_void;
pub type CFDictionaryRef = *const c_void;
pub type CFNumberRef = *const c_void;
pub type CFBooleanRef = *const c_void;
pub type AXUIElementRef = *const c_void;
pub type Id = *mut c_void;
pub type Sel = *const c_void;

pub const kCFStringEncodingUTF8: u32 = 0x0800_0100;
pub const kCFNumberSInt32Type: isize = 3;
pub const kCFNumberSInt64Type: isize = 4;
pub const kCFNumberDoubleType: isize = 13;
pub const kCFCompareCaseInsensitive: usize = 1;

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    pub static kCFTypeArrayCallBacks: c_void;
    pub static kCFTypeDictionaryKeyCallBacks: c_void;
    pub static kCFTypeDictionaryValueCallBacks: c_void;
    pub static kCFBooleanTrue: CFBooleanRef;
    pub fn CFRelease(cf: CFTypeRef);
    pub fn CFRetain(cf: CFTypeRef) -> CFTypeRef;
    pub fn CFArrayGetCount(array: CFArrayRef) -> isize;
    pub fn CFArrayGetValueAtIndex(array: CFArrayRef, index: isize) -> *const c_void;
    pub fn CFArrayCreate(alloc: *const c_void, values: *const *const c_void, count: isize, callbacks: *const c_void) -> CFArrayRef;
    pub fn CFDictionaryGetValue(dict: CFDictionaryRef, key: *const c_void) -> *const c_void;
    pub fn CFDictionaryCreate(alloc: *const c_void, keys: *const *const c_void, values: *const *const c_void, count: isize,
                              key_callbacks: *const c_void, value_callbacks: *const c_void) -> CFDictionaryRef;
    pub fn CFNumberCreate(alloc: *const c_void, kind: isize, value: *const c_void) -> CFNumberRef;
    pub fn CFNumberGetValue(number: CFNumberRef, kind: isize, value: *mut c_void) -> bool;
    pub fn CFStringCreateWithBytes(alloc: *const c_void, bytes: *const u8, len: isize, encoding: u32, external: bool) -> CFStringRef;
    pub fn CFStringGetCString(string: CFStringRef, buffer: *mut c_char, size: isize, encoding: u32) -> bool;
    pub fn CFStringCompare(a: CFStringRef, b: CFStringRef, options: usize) -> isize;
    pub fn CFUUIDCreateString(alloc: *const c_void, uuid: *const c_void) -> CFStringRef;
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct CGPoint {
    pub x: f64,
    pub y: f64,
}

pub const kCGWindowListOptionAll: u32 = 0;
pub const kCGWindowListOptionIncludingWindow: u32 = 1 << 3;
pub const kCGWindowListExcludeDesktopElements: u32 = 1 << 4;

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    pub static kCGWindowNumber: CFStringRef;
    pub static kCGWindowLayer: CFStringRef;
    pub static kCGWindowAlpha: CFStringRef;
    pub static kCGWindowBounds: CFStringRef;
    pub static kCGWindowOwnerPID: CFStringRef;
    pub fn CGEventCreate(source: *const c_void) -> CFTypeRef;
    pub fn CGEventGetLocation(event: CFTypeRef) -> CGPoint;
    pub fn CGGetDisplaysWithPoint(point: CGPoint, max: u32, displays: *mut u32, count: *mut u32) -> i32;
    pub fn CGDisplayCreateUUIDFromDisplayID(display: u32) -> *const c_void;
    pub fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> CFArrayRef;
}

#[link(name = "SkyLight", kind = "framework")]
extern "C" {
    pub fn CGSMainConnectionID() -> i32;
    pub fn CGSCopyManagedDisplaySpaces(cid: i32, display: CFStringRef) -> CFArrayRef;
    pub fn SLSCopySpacesForWindows(cid: i32, mask: i32, windows: CFArrayRef) -> CFArrayRef;
}

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    pub static kAXTrustedCheckOptionPrompt: CFStringRef;
    pub fn AXIsProcessTrusted() -> bool;
    pub fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
    pub fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    pub fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    pub fn AXUIElementSetMessagingTimeout(element: AXUIElementRef, seconds: f32) -> i32;
    pub fn AXUIElementCopyAttributeValue(element: AXUIElementRef, attribute: CFStringRef, value: *mut CFTypeRef) -> i32;
    pub fn AXUIElementSetAttributeValue(element: AXUIElementRef, attribute: CFStringRef, value: CFTypeRef) -> i32;
    pub fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> i32;
    pub fn _AXUIElementGetWindow(element: AXUIElementRef, wid: *mut u32) -> i32;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct EventHotKeyID {
    pub signature: u32,
    pub id: u32,
}

#[repr(C)]
pub struct EventTypeSpec {
    pub event_class: u32,
    pub event_kind: u32,
}

pub type EventHandler = extern "C" fn(next: *mut c_void, event: *mut c_void, data: *mut c_void) -> i32;

pub const fn fourcc(code: &[u8; 4]) -> u32 {
    (code[0] as u32) << 24 | (code[1] as u32) << 16 | (code[2] as u32) << 8 | code[3] as u32
}

pub const kEventClassKeyboard: u32 = fourcc(b"keyb");
pub const kEventHotKeyPressed: u32 = 5;
pub const kEventParamDirectObject: u32 = fourcc(b"----");
pub const typeEventHotKeyID: u32 = fourcc(b"hkid");
pub const cmdKey: u32 = 1 << 8;
pub const shiftKey: u32 = 1 << 9;
pub const kVK_ANSI_1: u32 = 0x12;
pub const kVK_ANSI_2: u32 = 0x13;
pub const kVK_ANSI_3: u32 = 0x14;
pub const kVK_ANSI_4: u32 = 0x15;
pub const kVK_ANSI_5: u32 = 0x17;
pub const kVK_LeftArrow: u32 = 0x7B;
pub const kVK_RightArrow: u32 = 0x7C;

#[link(name = "Carbon", kind = "framework")]
extern "C" {
    pub fn GetApplicationEventTarget() -> *mut c_void;
    pub fn InstallEventHandler(target: *mut c_void, handler: EventHandler, count: u32, types: *const EventTypeSpec,
                               data: *mut c_void, out: *mut *mut c_void) -> i32;
    pub fn RegisterEventHotKey(code: u32, modifiers: u32, id: EventHotKeyID, target: *mut c_void, options: u32,
                               out: *mut *mut c_void) -> i32;
    pub fn GetEventParameter(event: *mut c_void, name: u32, desired: u32, actual: *mut u32, size: usize,
                             actual_size: *mut usize, data: *mut c_void) -> i32;
}

pub type DispatchWork = extern "C" fn(context: *mut c_void);

extern "C" {
    pub static _dispatch_main_q: c_void;
    pub fn dispatch_sync_f(queue: *const c_void, context: *mut c_void, work: DispatchWork);
    pub fn dispatch_after_f(when: u64, queue: *const c_void, context: *mut c_void, work: DispatchWork);
    pub fn dispatch_time(when: u64, delta: i64) -> u64;
}

#[link(name = "objc")]
extern "C" {
    pub fn objc_getClass(name: *const c_char) -> Id;
    pub fn sel_registerName(name: *const c_char) -> Sel;
    pub fn class_getInstanceMethod(class: Id, sel: Sel) -> *const c_void;
    pub fn objc_msgSend();
    pub fn objc_autoreleasePoolPush() -> *mut c_void;
    pub fn objc_autoreleasePoolPop(pool: *mut c_void);
}

#[link(name = "AppKit", kind = "framework")]
extern "C" {}

#[repr(C)]
pub struct Tm {
    pub sec: i32,
    pub min: i32,
    pub hour: i32,
    pub mday: i32,
    pub mon: i32,
    pub year: i32,
    pub wday: i32,
    pub yday: i32,
    pub isdst: i32,
    pub gmtoff: i64,
    pub zone: *const c_char,
}

extern "C" {
    pub fn time(out: *mut i64) -> i64;
    pub fn localtime_r(t: *const i64, tm: *mut Tm) -> *mut Tm;
}

// Objective-C messages. objc_msgSend must be called through a pointer of the exact type.
pub unsafe fn sel(name: &CStr) -> Sel {
    sel_registerName(name.as_ptr())
}

pub unsafe fn class(name: &CStr) -> Id {
    objc_getClass(name.as_ptr())
}

pub unsafe fn msg(obj: Id, sel: Sel) -> Id {
    let f: extern "C" fn(Id, Sel) -> Id = std::mem::transmute(objc_msgSend as *const c_void);
    f(obj, sel)
}

pub unsafe fn msg_i32(obj: Id, sel: Sel) -> i32 {
    let f: extern "C" fn(Id, Sel) -> i32 = std::mem::transmute(objc_msgSend as *const c_void);
    f(obj, sel)
}

pub unsafe fn msg_ptr(obj: Id, sel: Sel, a: *const c_void) -> Id {
    let f: extern "C" fn(Id, Sel, *const c_void) -> Id = std::mem::transmute(objc_msgSend as *const c_void);
    f(obj, sel, a)
}

pub unsafe fn msg_ptr_u64(obj: Id, sel: Sel, a: *const c_void, b: u64) -> Id {
    let f: extern "C" fn(Id, Sel, *const c_void, u64) -> Id = std::mem::transmute(objc_msgSend as *const c_void);
    f(obj, sel, a, b)
}

pub unsafe fn msg_isize(obj: Id, sel: Sel, a: isize) -> bool {
    let f: extern "C" fn(Id, Sel, isize) -> bool = std::mem::transmute(objc_msgSend as *const c_void);
    f(obj, sel, a)
}
