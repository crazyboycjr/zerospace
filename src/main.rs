//! ZeroSpace: switch macOS desktops instantly and send windows between them, with no animation.
//!
//!   cmd+1..5               go to desktop N
//!   cmd+shift+1..5         send the focused window to desktop N and stay
//!   cmd+shift+left/right   take the focused window one desktop over and follow it
//!
//! Desktops are switched and windows moved with SkyLight's bridged window-management
//! operations (`SLSBridged*Operation` + `-performWithWMBridgeDelegate`), which macOS 27
//! accepts from a process with Accessibility access while SIP stays enabled. The approach
//! comes from InstantSpaceSwitcher PR #110 (apellis) and native-space-kit.
//!
//! A Unix socket at ~/.cache/zerospace.sock takes the same actions as text ("go 2", "send 3",
//! "carry right", "state"), for scripts and for testing over ssh.

mod sys;

use std::ffi::{c_char, c_void, CStr};
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::OnceLock;
use sys::*;

const AX_TIMEOUT: f32 = 0.25;
// The bridged operations land asynchronously; raising a window earlier reorders the old desktop
const FOCUS_DELAY_MS: i64 = 50;

fn log(message: &str) {
    let mut tm: Tm = unsafe { std::mem::zeroed() };
    unsafe {
        let now = time(null_mut());
        localtime_r(&now, &mut tm);
    }
    eprintln!("{:02}:{:02}:{:02} {message}", tm.hour, tm.min, tm.sec);
}

// ---------------------------------------------------------------------------------------------
// CoreFoundation helpers

/// An owned CoreFoundation object, released on drop.
struct Cf(CFTypeRef);

impl Cf {
    fn get(&self) -> CFTypeRef {
        self.0
    }
    fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

impl Drop for Cf {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe { CFRelease(self.0) }
        }
    }
}

fn cfstr(s: &str) -> Cf {
    Cf(unsafe { CFStringCreateWithBytes(null(), s.as_ptr(), s.len() as isize, kCFStringEncodingUTF8, false) })
}

fn string(s: CFStringRef) -> String {
    let mut buffer = [0 as c_char; 256];
    if s.is_null() || !unsafe { CFStringGetCString(s, buffer.as_mut_ptr(), buffer.len() as isize, kCFStringEncodingUTF8) } {
        return String::new();
    }
    unsafe { std::ffi::CStr::from_ptr(buffer.as_ptr()) }.to_string_lossy().into_owned()
}

fn number_i64(n: *const c_void) -> i64 {
    let mut value = 0i64;
    if !n.is_null() {
        unsafe { CFNumberGetValue(n, kCFNumberSInt64Type, &mut value as *mut i64 as *mut c_void) };
    }
    value
}

fn number_f64(n: *const c_void) -> f64 {
    let mut value = 0f64;
    if !n.is_null() {
        unsafe { CFNumberGetValue(n, kCFNumberDoubleType, &mut value as *mut f64 as *mut c_void) };
    }
    value
}

fn get(dict: CFDictionaryRef, key: &str) -> *const c_void {
    if dict.is_null() {
        return null();
    }
    let key = cfstr(key);
    unsafe { CFDictionaryGetValue(dict, key.get()) }
}

fn array(values: &[Cf]) -> Cf {
    let pointers: Vec<*const c_void> = values.iter().map(Cf::get).collect();
    Cf(unsafe { CFArrayCreate(null(), pointers.as_ptr(), pointers.len() as isize, &kCFTypeArrayCallBacks as *const c_void) })
}

fn numbers_i64(values: &[u64]) -> Cf {
    let numbers: Vec<Cf> = values
        .iter()
        .map(|v| Cf(unsafe { CFNumberCreate(null(), kCFNumberSInt64Type, v as *const u64 as *const c_void) }))
        .collect();
    array(&numbers)
}

fn numbers_i32(values: &[u32]) -> Cf {
    let numbers: Vec<Cf> = values
        .iter()
        .map(|v| Cf(unsafe { CFNumberCreate(null(), kCFNumberSInt32Type, v as *const u32 as *const c_void) }))
        .collect();
    array(&numbers)
}

fn each(list: CFArrayRef) -> impl Iterator<Item = *const c_void> {
    let count = if list.is_null() { 0 } else { unsafe { CFArrayGetCount(list) } };
    (0..count).map(move |i| unsafe { CFArrayGetValueAtIndex(list, i) })
}

// ---------------------------------------------------------------------------------------------
// Running on the main thread, now or a little later

extern "C" fn run_boxed(context: *mut c_void) {
    let f = unsafe { Box::from_raw(context as *mut Box<dyn FnOnce()>) };
    f();
}

fn after_ms(ms: i64, f: impl FnOnce() + 'static) {
    let boxed: Box<Box<dyn FnOnce()>> = Box::new(Box::new(f));
    unsafe {
        dispatch_after_f(dispatch_time(0, ms * 1_000_000), &_dispatch_main_q, Box::into_raw(boxed) as *mut c_void, run_boxed);
    }
}

struct SyncCall<F, R> {
    f: Option<F>,
    result: Option<R>,
}

extern "C" fn run_sync<F: FnOnce() -> R, R>(context: *mut c_void) {
    let call = unsafe { &mut *(context as *mut SyncCall<F, R>) };
    call.result = call.f.take().map(|f| f());
}

fn on_main<F: FnOnce() -> R + Send, R: Send>(f: F) -> R {
    let mut call = SyncCall { f: Some(f), result: None };
    unsafe { dispatch_sync_f(&_dispatch_main_q, &mut call as *mut SyncCall<F, R> as *mut c_void, run_sync::<F, R>) };
    call.result.expect("main queue ran the call")
}

// ---------------------------------------------------------------------------------------------
// Desktops

struct Display {
    uuid: Cf,
    /// Every space on the display, in Mission Control order.
    spaces: Vec<u64>,
    /// The ordinary desktops among them; full-screen apps excluded.
    desktops: Vec<u64>,
    current: u64,
}

fn cursor_display_uuid() -> Option<Cf> {
    unsafe {
        let event = Cf(CGEventCreate(null()));
        let point = CGEventGetLocation(event.get());
        let (mut display, mut count) = (0u32, 0u32);
        if CGGetDisplaysWithPoint(point, 1, &mut display, &mut count) != 0 || count == 0 {
            return None;
        }
        let uuid = Cf(CGDisplayCreateUUIDFromDisplayID(display));
        if uuid.is_null() {
            return None;
        }
        Some(Cf(CFUUIDCreateString(null(), uuid.get())))
    }
}

fn active_display() -> Option<Display> {
    let displays = Cf(unsafe { CGSCopyManagedDisplaySpaces(CGSMainConnectionID(), null()) });
    let wanted = cursor_display_uuid();
    let mut picked: CFDictionaryRef = null();
    for d in each(displays.get()) {
        if picked.is_null() {
            picked = d;
        }
        let id = get(d, "Display Identifier");
        if let Some(w) = &wanted {
            if !id.is_null() && unsafe { CFStringCompare(id, w.get(), kCFCompareCaseInsensitive) } == 0 {
                picked = d;
            }
        }
    }
    if picked.is_null() {
        return None;
    }
    let (mut spaces, mut desktops) = (Vec::new(), Vec::new());
    for space in each(get(picked, "Spaces")) {
        let sid = number_i64(get(space, "id64")) as u64;
        if sid == 0 {
            continue;
        }
        spaces.push(sid);
        if number_i64(get(space, "type")) == 0 {
            desktops.push(sid);
        }
    }
    let uuid = get(picked, "Display Identifier");
    Some(Display {
        uuid: Cf(unsafe { CFRetain(uuid) }),
        spaces,
        desktops,
        current: number_i64(get(get(picked, "Current Space"), "id64")) as u64,
    })
}

fn space_of_window(wid: u32) -> u64 {
    let windows = numbers_i32(&[wid]);
    let spaces = Cf(unsafe { SLSCopySpacesForWindows(CGSMainConnectionID(), 0x7, windows.get()) });
    each(spaces.get()).next().map(number_i64).unwrap_or(0) as u64
}

// ---------------------------------------------------------------------------------------------
// SkyLight's bridged window-management operations

struct Operations {
    move_windows: usize,
    show: usize,
    hide: usize,
    set_current: usize,
}

static OPERATIONS: OnceLock<Option<Operations>> = OnceLock::new();

fn operations() -> Option<&'static Operations> {
    OPERATIONS
        .get_or_init(|| unsafe {
            let responds = |class: Id, selector: &std::ffi::CStr| !class.is_null() && !class_getInstanceMethod(class, sel(selector)).is_null();
            let perform = c"performWithWMBridgeDelegate";
            let move_windows = class(c"SLSBridgedMoveWindowsToManagedSpaceOperation");
            let show = class(c"SLSBridgedShowSpacesOperation");
            let hide = class(c"SLSBridgedHideSpacesOperation");
            let set_current = class(c"SLSBridgedManagedDisplaySetCurrentSpaceOperation");
            let complete = responds(move_windows, c"initWithWindows:spaceID:") && responds(move_windows, perform)
                && responds(show, c"initWithSpaces:") && responds(show, perform)
                && responds(hide, c"initWithSpaces:") && responds(hide, perform)
                && responds(set_current, c"initWithDisplayIdentifier:spaceID:") && responds(set_current, perform);
            complete.then_some(Operations {
                move_windows: move_windows as usize,
                show: show as usize,
                hide: hide as usize,
                set_current: set_current as usize,
            })
        })
        .as_ref()
}

unsafe fn perform_and_release(operation: Id) {
    msg(operation, sel(c"performWithWMBridgeDelegate"));
    msg(operation, sel(c"release"));
}

fn activate_space(display: &Display, sid: u64) {
    let Some(ops) = operations() else { return };
    let others: Vec<u64> = display.spaces.iter().copied().filter(|&s| s != sid).collect();
    let (shown, hidden) = (numbers_i64(&[sid]), numbers_i64(&others));
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let alloc = sel(c"alloc");
        let show = msg_ptr(msg(ops.show as Id, alloc), sel(c"initWithSpaces:"), shown.get());
        let hide = msg_ptr(msg(ops.hide as Id, alloc), sel(c"initWithSpaces:"), hidden.get());
        let current = msg_ptr_u64(msg(ops.set_current as Id, alloc), sel(c"initWithDisplayIdentifier:spaceID:"), display.uuid.get(), sid);
        // Show the target, hide its siblings, then make it current: the order that works on macOS 27
        for operation in [show, hide, current] {
            if !operation.is_null() {
                perform_and_release(operation);
            }
        }
        objc_autoreleasePoolPop(pool);
    }
}

fn move_window(wid: u32, sid: u64) {
    let Some(ops) = operations() else { return };
    let windows = numbers_i32(&[wid]);
    unsafe {
        let pool = objc_autoreleasePoolPush();
        let operation = msg_ptr_u64(msg(ops.move_windows as Id, sel(c"alloc")), sel(c"initWithWindows:spaceID:"), windows.get(), sid);
        if !operation.is_null() {
            perform_and_release(operation);
        }
        objc_autoreleasePoolPop(pool);
    }
}

// ---------------------------------------------------------------------------------------------
// Windows and focus

fn ax_attribute(element: AXUIElementRef, name: &str) -> Option<Cf> {
    let key = cfstr(name);
    let mut value: CFTypeRef = null();
    (unsafe { AXUIElementCopyAttributeValue(element, key.get(), &mut value) } == 0 && !value.is_null()).then(|| Cf(value))
}

fn focused_window() -> u32 {
    let wid = ax_focused_window();
    if wid != 0 { wid } else { front_window_of(frontmost_pid()) }
}

fn ax_focused_window() -> u32 {
    unsafe {
        let system = Cf(AXUIElementCreateSystemWide());
        AXUIElementSetMessagingTimeout(system.get(), AX_TIMEOUT);
        let Some(app) = ax_attribute(system.get(), "AXFocusedApplication") else { return 0 };
        AXUIElementSetMessagingTimeout(app.get(), AX_TIMEOUT);
        let Some(window) = ax_attribute(app.get(), "AXFocusedWindow") else { return 0 };
        let mut wid = 0u32;
        _AXUIElementGetWindow(window.get(), &mut wid);
        wid
    }
}

fn frontmost_pid() -> i32 {
    unsafe {
        let workspace = msg(class(c"NSWorkspace"), sel(c"sharedWorkspace"));
        let app = msg(workspace, sel(c"frontmostApplication"));
        if app.is_null() { 0 } else { msg_i32(app, sel(c"processIdentifier")) }
    }
}

/// Chrome often leaves the Accessibility focus query unanswered; the frontmost app's
/// topmost on-screen window is its focused one.
fn front_window_of(pid: i32) -> u32 {
    if pid <= 0 {
        return 0;
    }
    let list = Cf(unsafe { CGWindowListCopyWindowInfo(kCGWindowListOptionOnScreenOnly | kCGWindowListExcludeDesktopElements, 0) });
    for w in each(list.get()) {
        let field = |key: CFStringRef| unsafe { CFDictionaryGetValue(w, key) };
        let bounds = field(unsafe { kCGWindowBounds });
        if number_i64(field(unsafe { kCGWindowOwnerPID })) as i32 == pid
            && number_i64(field(unsafe { kCGWindowLayer })) == 0
            && number_f64(get(bounds, "Width")) >= 100.0
            && number_f64(get(bounds, "Height")) >= 100.0
        {
            return number_i64(field(unsafe { kCGWindowNumber })) as u32;
        }
    }
    0
}

fn owner_of_window(wid: u32) -> i32 {
    let list = Cf(unsafe { CGWindowListCopyWindowInfo(kCGWindowListOptionIncludingWindow, wid) });
    each(list.get()).next().map(|w| number_i64(unsafe { CFDictionaryGetValue(w, kCGWindowOwnerPID) }) as i32).unwrap_or(0)
}

fn bring_to_front(pid: i32) {
    unsafe {
        let app = Cf(AXUIElementCreateApplication(pid));
        AXUIElementSetAttributeValue(app.get(), cfstr("AXFrontmost").get(), kCFBooleanTrue);
    }
}

fn focus_window(wid: u32) {
    let pid = owner_of_window(wid);
    if pid <= 0 {
        return;
    }
    unsafe {
        let app = Cf(AXUIElementCreateApplication(pid));
        AXUIElementSetMessagingTimeout(app.get(), AX_TIMEOUT);
        if let Some(windows) = ax_attribute(app.get(), "AXWindows") {
            for window in each(windows.get()) {
                let mut candidate = 0u32;
                if _AXUIElementGetWindow(window, &mut candidate) == 0 && candidate == wid {
                    AXUIElementPerformAction(window, cfstr("AXRaise").get());
                    AXUIElementSetAttributeValue(window, cfstr("AXMain").get(), kCFBooleanTrue);
                    break;
                }
            }
        }
        AXUIElementSetAttributeValue(app.get(), cfstr("AXFrontmost").get(), kCFBooleanTrue);
    }
}

/// The frontmost ordinary window on a desktop, skipping `except`.
fn top_window_on(sid: u64, except: u32) -> u32 {
    let list = Cf(unsafe { CGWindowListCopyWindowInfo(kCGWindowListOptionAll | kCGWindowListExcludeDesktopElements, 0) });
    for w in each(list.get()) {
        let field = |key: CFStringRef| unsafe { CFDictionaryGetValue(w, key) };
        let wid = number_i64(field(unsafe { kCGWindowNumber })) as u32;
        let bounds = field(unsafe { kCGWindowBounds });
        if wid == except
            || number_i64(field(unsafe { kCGWindowLayer })) != 0
            || number_f64(field(unsafe { kCGWindowAlpha })) <= 0.0
            || number_f64(get(bounds, "Width")) < 100.0
            || number_f64(get(bounds, "Height")) < 100.0
        {
            continue;
        }
        if space_of_window(wid) == sid {
            return wid;
        }
    }
    0
}

fn finder_pid() -> i32 {
    unsafe {
        let apps = msg_ptr(class(c"NSRunningApplication"), sel(c"runningApplicationsWithBundleIdentifier:"), cfstr("com.apple.finder").get());
        let finder = msg(apps, sel(c"firstObject"));
        if finder.is_null() { 0 } else { msg_i32(finder, sel(c"processIdentifier")) }
    }
}

fn focus_soon(sid: u64, preferred: u32, except: u32) {
    after_ms(FOCUS_DELAY_MS, move || {
        let wid = if preferred != 0 { preferred } else { top_window_on(sid, except) };
        if wid != 0 {
            focus_window(wid);
        } else {
            // An empty desktop: hand the keyboard to Finder, as macOS does
            let pid = finder_pid();
            if pid > 0 {
                bring_to_front(pid);
            }
        }
    });
}

// ---------------------------------------------------------------------------------------------
// On-screen indicator: a borderless "N" shown for half a second after a switch

static OSD_WINDOW: AtomicUsize = AtomicUsize::new(0);
static OSD_FIELD: AtomicUsize = AtomicUsize::new(0);
static OSD_GENERATION: AtomicU64 = AtomicU64::new(0);
const OSD_MS: i64 = 500;

fn ns_color(name: &CStr) -> Id {
    unsafe { msg(class(c"NSColor"), sel(name)) }
}

fn create_osd() {
    unsafe {
        let frame = CGRect { origin: CGPoint { x: 0.0, y: 0.0 }, size: CGSize { width: 200.0, height: 110.0 } };
        let window = msg_window_init(msg(class(c"NSWindow"), sel(c"alloc")), sel(c"initWithContentRect:styleMask:backing:defer:"),
                                     frame, 0 /* borderless */, 2 /* buffered */, false);
        msg_set_isize(window, sel(c"setLevel:"), 1000);
        msg_set_bool(window, sel(c"setOpaque:"), false);
        msg_set_id(window, sel(c"setBackgroundColor:"), ns_color(c"clearColor"));
        msg_set_bool(window, sel(c"setIgnoresMouseEvents:"), true);
        msg_set_bool(window, sel(c"setHasShadow:"), true);
        msg_set_bool(window, sel(c"setReleasedWhenClosed:"), false);
        // canJoinAllSpaces | stationary | ignoresCycle | fullScreenAuxiliary, so it shows on the space we land on
        msg_set_u64(window, sel(c"setCollectionBehavior:"), (1 << 0) | (1 << 4) | (1 << 6) | (1 << 8));

        let view = msg_rect(msg(class(c"NSView"), sel(c"alloc")), sel(c"initWithFrame:"), frame);
        msg_set_bool(view, sel(c"setWantsLayer:"), true);
        let layer = msg(view, sel(c"layer"));
        msg_set_f64(layer, sel(c"setCornerRadius:"), 22.0);
        msg_set_bool(layer, sel(c"setMasksToBounds:"), true);
        let background = msg_f64(ns_color(c"blackColor"), sel(c"colorWithAlphaComponent:"), 0.78);
        msg_set_id(layer, sel(c"setBackgroundColor:"), msg(background, sel(c"CGColor")));
        msg_set_id(window, sel(c"setContentView:"), view);

        let field_frame = CGRect { origin: CGPoint { x: 0.0, y: 28.0 }, size: CGSize { width: 200.0, height: 58.0 } };
        let field = msg_rect(msg(class(c"NSTextField"), sel(c"alloc")), sel(c"initWithFrame:"), field_frame);
        msg_set_bool(field, sel(c"setEditable:"), false);
        msg_set_bool(field, sel(c"setSelectable:"), false);
        msg_set_bool(field, sel(c"setBezeled:"), false);
        msg_set_bool(field, sel(c"setDrawsBackground:"), false);
        msg_set_isize(field, sel(c"setAlignment:"), 1); // centered (macOS NSTextAlignmentCenter)
        msg_set_id(field, sel(c"setTextColor:"), ns_color(c"whiteColor"));
        msg_set_id(field, sel(c"setFont:"), msg_f64(class(c"NSFont"), sel(c"boldSystemFontOfSize:"), 46.0));
        msg_set_id(view, sel(c"addSubview:"), field);

        // Keep the window ordered front on every space (canJoinAllSpaces) but invisible; a
        // switch then only has to raise its alpha, so it shows on the space we land on, not
        // the one we left. Ordering it front per-switch would pin it to the pre-switch space.
        msg_set_f64(window, sel(c"setAlphaValue:"), 0.0);
        msg(window, sel(c"orderFrontRegardless"));
        OSD_WINDOW.store(window as usize, Ordering::SeqCst);
        OSD_FIELD.store(field as usize, Ordering::SeqCst);
    }
}

fn show_osd(text: &str) {
    if OSD_WINDOW.load(Ordering::SeqCst) == 0 {
        create_osd();
    }
    let window = OSD_WINDOW.load(Ordering::SeqCst) as Id;
    let field = OSD_FIELD.load(Ordering::SeqCst) as Id;
    if window.is_null() {
        return;
    }
    unsafe {
        let value = cfstr(text);
        msg_set_id(field, sel(c"setStringValue:"), value.get() as Id);
        msg(window, sel(c"center"));
        msg_set_f64(window, sel(c"setAlphaValue:"), 1.0);
    }
    let generation = OSD_GENERATION.fetch_add(1, Ordering::SeqCst) + 1;
    after_ms(OSD_MS, move || {
        // Only hide if no newer switch has shown the indicator since
        if OSD_GENERATION.load(Ordering::SeqCst) == generation {
            unsafe { msg_set_f64(OSD_WINDOW.load(Ordering::SeqCst) as Id, sel(c"setAlphaValue:"), 0.0) };
        }
    });
}

// ---------------------------------------------------------------------------------------------
// Actions. Desktops are numbered by their position in the Mission Control bar, full-screen
// spaces included, so cmd+N can always reach the space the user sees in slot N.

fn position_of(display: &Display, sid: u64) -> usize {
    display.spaces.iter().position(|&s| s == sid).map_or(0, |i| i + 1)
}

fn go_to_desktop(n: usize) -> String {
    let Some(display) = active_display() else { return "no display".into() };
    let Some(&target) = display.spaces.get(n) else {
        return format!("no desktop {} (have {})", n + 1, display.spaces.len());
    };
    if target != display.current {
        activate_space(&display, target);
        focus_soon(target, 0, 0);
    }
    show_osd(&(n + 1).to_string());
    format!("desktop {} (space {target})", n + 1)
}

fn send_to_desktop(n: usize) -> String {
    let Some(display) = active_display() else { return "no display".into() };
    let Some(&target) = display.spaces.get(n) else { return format!("no desktop {}", n + 1) };
    if !display.desktops.contains(&target) {
        return "slot is a full-screen app, can't send a window there".into();
    }
    let wid = focused_window();
    if wid == 0 {
        return "no focused window".into();
    }
    if space_of_window(wid) == target {
        return "window is already there".into();
    }
    move_window(wid, target);
    focus_soon(display.current, 0, wid);
    show_osd(&format!("\u{2192}{}", n + 1)); // →N
    format!("window {wid} to desktop {}", n + 1)
}

fn carry(step: isize) -> String {
    let Some(display) = active_display() else { return "no display".into() };
    // Carry only lands on ordinary desktops; a window cannot sit in a full-screen app's space.
    let here = match display.desktops.iter().position(|&s| s == display.current) {
        Some(here) => here,
        None => {
            // On a full-screen app: step to the nearest ordinary desktop that way, no window moved
            let from = display.spaces.iter().position(|&s| s == display.current).unwrap_or(0) as isize;
            let mut i = from + step;
            while i >= 0 && (i as usize) < display.spaces.len() {
                let sid = display.spaces[i as usize];
                if display.desktops.contains(&sid) {
                    activate_space(&display, sid);
                    focus_soon(sid, 0, 0);
                    show_osd(&position_of(&display, sid).to_string());
                    return "left the full-screen app".into();
                }
                i += step;
            }
            return "no desktop that way".into();
        }
    };
    let next = here as isize + step;
    if next < 0 || next as usize >= display.desktops.len() {
        return "no desktop that way".into();
    }
    let target = display.desktops[next as usize];
    let wid = focused_window();
    if wid != 0 && space_of_window(wid) == display.current {
        move_window(wid, target);
    }
    activate_space(&display, target);
    focus_soon(target, wid, 0);
    show_osd(&position_of(&display, target).to_string());
    format!("window {wid} to desktop {}, following", position_of(&display, target))
}

fn state() -> String {
    let Some(display) = active_display() else { return "no display".into() };
    format!(
        "display {} spaces {:?} desktops {:?} current {} (slot {}) focused window {} (ax {}, front {}) trusted {} operations {}",
        string(display.uuid.get()),
        display.spaces,
        display.desktops,
        display.current,
        position_of(&display, display.current),
        focused_window(),
        ax_focused_window(),
        front_window_of(frontmost_pid()),
        unsafe { AXIsProcessTrusted() },
        operations().is_some()
    )
}

fn run_command(line: &str) -> String {
    let mut words = line.split_whitespace();
    let (verb, arg) = (words.next().unwrap_or(""), words.next().unwrap_or(""));
    let n = arg.parse::<usize>().unwrap_or(1).max(1) - 1;
    match verb {
        "go" => go_to_desktop(n),
        "send" => send_to_desktop(n),
        "carry" => carry(if arg == "left" { -1 } else { 1 }),
        "move" => {
            // move WINDOW N: send a given window to desktop N, for scripts
            let wid = arg.parse::<u32>().unwrap_or(0);
            let n = words.next().and_then(|w| w.parse::<usize>().ok()).unwrap_or(1).max(1) - 1;
            match active_display().and_then(|d| d.desktops.get(n).copied()) {
                Some(target) if wid != 0 => {
                    move_window(wid, target);
                    format!("window {wid} to desktop {}", n + 1)
                }
                _ => "usage: move WINDOW N".into(),
            }
        }
        "state" => state(),
        _ => "commands: go N | send N | carry left|right | move WINDOW N | state".into(),
    }
}

// ---------------------------------------------------------------------------------------------
// Hot keys and the command socket

extern "C" fn hot_key_pressed(_next: *mut c_void, event: *mut c_void, _data: *mut c_void) -> i32 {
    let mut key = EventHotKeyID { signature: 0, id: 0 };
    unsafe {
        GetEventParameter(event, kEventParamDirectObject, typeEventHotKeyID, null_mut(), std::mem::size_of::<EventHotKeyID>(),
                          null_mut(), &mut key as *mut EventHotKeyID as *mut c_void);
    }
    let result = match key.id {
        1..=9 => go_to_desktop(key.id as usize - 1),
        11..=19 => send_to_desktop(key.id as usize - 11),
        21 => carry(-1),
        _ => carry(1),
    };
    log(&format!("hot key {}: {result}", key.id));
    0
}

fn register_hot_keys() {
    let signature = fourcc(b"hop!");
    unsafe {
        let target = GetApplicationEventTarget();
        let spec = EventTypeSpec { event_class: kEventClassKeyboard, event_kind: kEventHotKeyPressed };
        InstallEventHandler(target, hot_key_pressed, 1, &spec, null_mut(), null_mut());
        let register = |code: u32, modifiers: u32, id: u32| {
            let mut reference = null_mut();
            let status = RegisterEventHotKey(code, modifiers, EventHotKeyID { signature, id }, target, 0, &mut reference);
            if status != 0 {
                log(&format!("could not register hot key {id}: {status}"));
            }
        };
        for (i, code) in [kVK_ANSI_1, kVK_ANSI_2, kVK_ANSI_3, kVK_ANSI_4, kVK_ANSI_5].into_iter().enumerate() {
            register(code, cmdKey, i as u32 + 1);
            register(code, cmdKey | shiftKey, i as u32 + 11);
        }
        register(kVK_LeftArrow, cmdKey | shiftKey, 21);
        register(kVK_RightArrow, cmdKey | shiftKey, 22);
    }
}

fn serve(mut stream: UnixStream) {
    let mut buffer = [0u8; 256];
    let n = stream.read(&mut buffer).unwrap_or(0);
    let line = String::from_utf8_lossy(&buffer[..n]).into_owned();
    let reply = on_main(move || run_command(&line));
    let _ = stream.write_all(format!("{reply}\n").as_bytes());
}

fn listen_on_socket() {
    let path = format!("{}/.cache/zerospace.sock", std::env::var("HOME").unwrap_or_default());
    let _ = std::fs::remove_file(&path);
    match UnixListener::bind(&path) {
        Ok(listener) => {
            std::thread::spawn(move || listener.incoming().flatten().for_each(serve));
        }
        Err(e) => log(&format!("socket {path}: {e}")),
    }
}

fn main() {
    unsafe {
        let app = msg(class(c"NSApplication"), sel(c"sharedApplication"));
        msg_isize(app, sel(c"setActivationPolicy:"), 1); // accessory: no Dock icon, no menu bar
        if operations().is_none() {
            log("SkyLight's bridged operations are missing on this macOS; switching will not work");
        }
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let options = Cf(CFDictionaryCreate(null(), keys.as_ptr() as *const *const c_void, values.as_ptr() as *const *const c_void, 1,
                                            &kCFTypeDictionaryKeyCallBacks as *const c_void, &kCFTypeDictionaryValueCallBacks as *const c_void));
        if !AXIsProcessTrustedWithOptions(options.get()) {
            log("waiting for Accessibility access (System Settings > Privacy & Security > Accessibility)");
        }
        register_hot_keys();
        listen_on_socket();
        log(&format!("ZeroSpace ready: {}", state()));
        msg(app, sel(c"run"));
    }
}
