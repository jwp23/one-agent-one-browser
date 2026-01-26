use super::painter::MacPainter;
use super::scale::ScaleFactor;
use super::scaled::ScaledPainter;
use super::WindowOptions;
use crate::app::App;
use crate::render::Viewport;
use core::ffi::{c_char, c_double, c_long, c_ulong, c_void};
use std::time::{Duration, Instant};

const MAX_EVENTS_PER_TICK: usize = 512;

const SCREENSHOT_RESOURCE_WAIT_TIMEOUT: Duration = Duration::from_secs(5);

const EVENT_TYPE_LEFT_MOUSE_DOWN: c_ulong = 1;
const EVENT_TYPE_KEY_DOWN: c_ulong = 10;
const EVENT_TYPE_SCROLL_WHEEL: c_ulong = 22;

type Id = *mut c_void;
type Sel = *mut c_void;
type ObjcBool = i8;

const NO: ObjcBool = 0;
const YES: ObjcBool = 1;

#[repr(C)]
struct NSPoint {
    x: c_double,
    y: c_double,
}

#[repr(C)]
struct NSSize {
    width: c_double,
    height: c_double,
}

#[repr(C)]
struct NSRect {
    origin: NSPoint,
    size: NSSize,
}

#[link(name = "objc")]
unsafe extern "C" {
    fn objc_getClass(name: *const c_char) -> *mut c_void;
    fn sel_registerName(name: *const c_char) -> Sel;
    fn objc_msgSend();
    #[cfg(target_arch = "x86_64")]
    fn objc_msgSend_stret();
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(cf: *const c_void);
    fn CFStringCreateWithBytes(
        alloc: *const c_void,
        bytes: *const u8,
        num_bytes: isize,
        encoding: u32,
        is_external_representation: u8,
    ) -> *const c_void;
}

#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {
    static NSDefaultRunLoopMode: Id;
}

pub(super) fn run<A: App>(title: &str, options: WindowOptions, app: &mut A) -> Result<(), String> {
    let initial_width_css = options.initial_width_px.unwrap_or(1024);
    let initial_height_css = options.initial_height_px.unwrap_or(768);
    if initial_width_css <= 0 || initial_height_css <= 0 {
        return Err(format!(
            "Invalid initial window size: {initial_width_css}x{initial_height_css}"
        ));
    }

    let mut cocoa = CocoaApp::new(title, initial_width_css, initial_height_css)?;
    let mut scale = ScaleFactor::detect(false, Some(cocoa.backing_scale_factor()));

    let mut viewport = cocoa.device_viewport(scale)?;
    let mut css_viewport = Viewport {
        width_px: scale.device_size_to_css_px(viewport.width_px),
        height_px: scale.device_size_to_css_px(viewport.height_px),
    };

    let mut painter = MacPainter::new(viewport)?;

    let mut screenshot_path = options.screenshot_path;
    let mut needs_redraw = true;
    let mut should_exit = false;
    let mut has_rendered_ready_state = false;
    let mut resource_wait_started: Option<Instant> = None;
    let mut scroll_accum_y: c_double = 0.0;

    loop {
        let _pool = AutoreleasePool::new();

        if !cocoa.window_is_visible() {
            break;
        }

        let mut processed = 0usize;
        while processed < MAX_EVENTS_PER_TICK {
            let Some(event) = cocoa.next_event(Duration::from_millis(0))? else {
                break;
            };

            let event_type = cocoa.event_type(event);
            match event_type {
                EVENT_TYPE_LEFT_MOUSE_DOWN => {
                    if let Some((x_css, y_css)) = cocoa.event_location_css(event) {
                        let tick = app.mouse_down(x_css, y_css, css_viewport)?;
                        if tick.needs_redraw {
                            needs_redraw = true;
                        }
                    }
                    cocoa.send_event(event);
                }
                EVENT_TYPE_SCROLL_WHEEL => {
                    scroll_accum_y += cocoa.event_scroll_delta_y(event);
                    let delta_y_css = (-scroll_accum_y).trunc() as i32;
                    if delta_y_css != 0 {
                        scroll_accum_y += delta_y_css as c_double;
                        let tick = app.mouse_wheel(delta_y_css, css_viewport)?;
                        if tick.needs_redraw {
                            needs_redraw = true;
                        }
                    }
                    cocoa.send_event(event);
                }
                EVENT_TYPE_KEY_DOWN => {
                    should_exit = true;
                    break;
                }
                _ => {
                    cocoa.send_event(event);
                }
            }

            processed += 1;
        }

        if should_exit {
            break;
        }

        if let Some(backing) = cocoa.backing_scale_factor_checked() {
            let next_scale = ScaleFactor::detect(false, Some(backing));
            let next_viewport = cocoa.device_viewport(next_scale)?;
            if next_scale != scale || next_viewport != viewport {
                scale = next_scale;
                viewport = next_viewport;
                css_viewport = Viewport {
                    width_px: scale.device_size_to_css_px(viewport.width_px),
                    height_px: scale.device_size_to_css_px(viewport.height_px),
                };
                painter.ensure_back_buffer(viewport)?;
                cocoa.set_contents_scale(backing);
                needs_redraw = true;
                has_rendered_ready_state = false;
                resource_wait_started = None;
            }
        }

        let tick = app.tick()?;
        if tick.needs_redraw {
            needs_redraw = true;
        }

        let ready_for_screenshot = tick.ready_for_screenshot;
        if !ready_for_screenshot {
            has_rendered_ready_state = false;
            resource_wait_started = None;
        }

        let should_wait_for_resources = tick.pending_resources > 0;
        let timed_out_waiting_for_resources = resource_wait_started
            .is_some_and(|started| started.elapsed() >= SCREENSHOT_RESOURCE_WAIT_TIMEOUT);
        let can_complete = !should_wait_for_resources || timed_out_waiting_for_resources;

        let wants_screenshot = screenshot_path.is_some();
        let should_complete_screenshot =
            wants_screenshot && ready_for_screenshot && has_rendered_ready_state;

        let mut capture_now = false;
        let mut capture_after_render = false;

        if ready_for_screenshot && wants_screenshot && !has_rendered_ready_state {
            needs_redraw = true;
        } else if ready_for_screenshot && should_wait_for_resources && has_rendered_ready_state {
            resource_wait_started.get_or_insert(Instant::now());
        } else if ready_for_screenshot && has_rendered_ready_state {
            resource_wait_started = None;
        }

        if ready_for_screenshot && has_rendered_ready_state && can_complete {
            if should_complete_screenshot {
                if needs_redraw {
                    capture_after_render = true;
                } else {
                    capture_now = true;
                }
            }
        }

        if capture_now {
            let Some(path) = screenshot_path.take() else {
                return Err("Internal error: capture_now set but screenshot path missing".to_owned());
            };
            let rgb = painter.capture_back_buffer_rgb()?;
            crate::png::write_rgb_png(&path, &rgb)?;
            break;
        }

        if needs_redraw {
            painter.ensure_back_buffer(viewport)?;
            let mut scaled_painter = ScaledPainter::new(&mut painter, scale);
            app.render(&mut scaled_painter, css_viewport)?;
            needs_redraw = false;

            let image = painter.create_cgimage()?;
            cocoa.present_image(image);
            unsafe { CFRelease(image as *const c_void) };

            if ready_for_screenshot {
                has_rendered_ready_state = true;
                if capture_after_render {
                    let Some(path) = screenshot_path.take() else {
                        return Err("Internal error: capture_after_render set but screenshot path missing".to_owned());
                    };
                    let rgb = painter.capture_back_buffer_rgb()?;
                    crate::png::write_rgb_png(&path, &rgb)?;
                    break;
                }
            }
        }

        if processed == 0 && !needs_redraw {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    cocoa.close();
    Ok(())
}

struct CocoaApp {
    app: Id,
    window: Id,
    view: Id,
    layer: Id,
}

impl CocoaApp {
    fn new(title: &str, initial_width_css: i32, initial_height_css: i32) -> Result<Self, String> {
        let _pool = AutoreleasePool::new();

        let app_cls = class(b"NSApplication\0");
        let app: Id = unsafe {
            let f: unsafe extern "C" fn(Id, Sel) -> Id =
                std::mem::transmute(objc_msg_send_ptr());
            f(app_cls, sel(b"sharedApplication\0"))
        };
        if app.is_null() {
            return Err("NSApplication.sharedApplication returned null".to_owned());
        }

        unsafe {
            let f: unsafe extern "C" fn(Id, Sel, c_long) -> ObjcBool =
                std::mem::transmute(objc_msg_send_ptr());
            let ok = f(app, sel(b"setActivationPolicy:\0"), 0);
            let _ = ok;
        }

        let window = create_window(initial_width_css, initial_height_css, title)?;
        let view = unsafe {
            let f: unsafe extern "C" fn(Id, Sel) -> Id =
                std::mem::transmute(objc_msg_send_ptr());
            f(window, sel(b"contentView\0"))
        };
        if view.is_null() {
            return Err("NSWindow.contentView returned null".to_owned());
        }

        unsafe {
            let f: unsafe extern "C" fn(Id, Sel, ObjcBool) =
                std::mem::transmute(objc_msg_send_ptr());
            f(view, sel(b"setWantsLayer:\0"), YES);
        }

        let layer = unsafe {
            let f: unsafe extern "C" fn(Id, Sel) -> Id =
                std::mem::transmute(objc_msg_send_ptr());
            f(view, sel(b"layer\0"))
        };
        if layer.is_null() {
            return Err("NSView.layer returned null".to_owned());
        }

        let backing = unsafe {
            let f: unsafe extern "C" fn(Id, Sel) -> c_double =
                std::mem::transmute(objc_msg_send_ptr());
            f(window, sel(b"backingScaleFactor\0"))
        };
        if backing.is_finite() && backing > 0.0 {
            unsafe {
                let f: unsafe extern "C" fn(Id, Sel, c_double) =
                    std::mem::transmute(objc_msg_send_ptr());
                f(layer, sel(b"setContentsScale:\0"), backing);
            }
        }

        unsafe {
            let f: unsafe extern "C" fn(Id, Sel, ObjcBool) =
                std::mem::transmute(objc_msg_send_ptr());
            f(app, sel(b"activateIgnoringOtherApps:\0"), YES);
        }

        Ok(Self {
            app,
            window,
            view,
            layer,
        })
    }

    fn window_is_visible(&self) -> bool {
        unsafe {
            let f: unsafe extern "C" fn(Id, Sel) -> ObjcBool =
                std::mem::transmute(objc_msg_send_ptr());
            f(self.window, sel(b"isVisible\0")) != NO
        }
    }

    fn close(&mut self) {
        unsafe {
            let f: unsafe extern "C" fn(Id, Sel) = std::mem::transmute(objc_msg_send_ptr());
            f(self.window, sel(b"close\0"));
        }
    }

    fn present_image(&self, image: *mut c_void) {
        unsafe {
            let f: unsafe extern "C" fn(Id, Sel, Id) = std::mem::transmute(objc_msg_send_ptr());
            f(self.layer, sel(b"setContents:\0"), image as Id);
        }
    }

    fn set_contents_scale(&self, scale: c_double) {
        unsafe {
            let f: unsafe extern "C" fn(Id, Sel, c_double) =
                std::mem::transmute(objc_msg_send_ptr());
            f(self.layer, sel(b"setContentsScale:\0"), scale);
        }
    }

    fn backing_scale_factor(&self) -> c_double {
        unsafe {
            let f: unsafe extern "C" fn(Id, Sel) -> c_double =
                std::mem::transmute(objc_msg_send_ptr());
            f(self.window, sel(b"backingScaleFactor\0"))
        }
    }

    fn backing_scale_factor_checked(&self) -> Option<c_double> {
        let s = self.backing_scale_factor();
        if s.is_finite() && s > 0.0 {
            Some(s)
        } else {
            None
        }
    }

    fn device_viewport(&self, scale: ScaleFactor) -> Result<Viewport, String> {
        let bounds = view_bounds(self.view);
        let width_css = bounds.size.width.round() as i32;
        let height_css = bounds.size.height.round() as i32;
        if width_css <= 0 || height_css <= 0 {
            return Err(format!("Invalid content size: {width_css}x{height_css}"));
        }
        Ok(Viewport {
            width_px: scale.css_size_to_device_px(width_css),
            height_px: scale.css_size_to_device_px(height_css),
        })
    }

    fn next_event(&self, timeout: Duration) -> Result<Option<Id>, String> {
        let seconds = timeout.as_secs_f64();
        let date = date_with_interval(seconds);
        let mask = u64::MAX;
        let mode = unsafe { NSDefaultRunLoopMode };

        let event = unsafe {
            let f: unsafe extern "C" fn(Id, Sel, u64, Id, Id, ObjcBool) -> Id =
                std::mem::transmute(objc_msg_send_ptr());
            f(
                self.app,
                sel(b"nextEventMatchingMask:untilDate:inMode:dequeue:\0"),
                mask,
                date,
                mode,
                YES,
            )
        };
        Ok(if event.is_null() { None } else { Some(event) })
    }

    fn send_event(&self, event: Id) {
        unsafe {
            let f: unsafe extern "C" fn(Id, Sel, Id) = std::mem::transmute(objc_msg_send_ptr());
            f(self.app, sel(b"sendEvent:\0"), event);
        }
    }

    fn event_type(&self, event: Id) -> c_ulong {
        unsafe {
            let f: unsafe extern "C" fn(Id, Sel) -> c_ulong =
                std::mem::transmute(objc_msg_send_ptr());
            f(event, sel(b"type\0"))
        }
    }

    fn event_location_css(&self, event: Id) -> Option<(i32, i32)> {
        let point = event_location_in_window(event);
        let bounds = view_bounds(self.view);
        let height = bounds.size.height;
        if !height.is_finite() || height <= 0.0 {
            return None;
        }
        let x = point.x.round() as i64;
        let y = (height - point.y).round() as i64;
        let x = x.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        let y = y.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
        Some((x, y))
    }

    fn event_scroll_delta_y(&self, event: Id) -> c_double {
        unsafe {
            let f: unsafe extern "C" fn(Id, Sel) -> c_double =
                std::mem::transmute(objc_msg_send_ptr());
            f(event, sel(b"scrollingDeltaY\0"))
        }
    }
}

struct AutoreleasePool(Id);

impl AutoreleasePool {
    fn new() -> Self {
        unsafe {
            let cls = class(b"NSAutoreleasePool\0");
            let f: unsafe extern "C" fn(Id, Sel) -> Id =
                std::mem::transmute(objc_msg_send_ptr());
            let pool = f(cls, sel(b"new\0"));
            Self(pool)
        }
    }
}

impl Drop for AutoreleasePool {
    fn drop(&mut self) {
        if self.0.is_null() {
            return;
        }
        unsafe {
            let f: unsafe extern "C" fn(Id, Sel) = std::mem::transmute(objc_msg_send_ptr());
            f(self.0, sel(b"drain\0"));
        }
    }
}

fn create_window(width_css: i32, height_css: i32, title: &str) -> Result<Id, String> {
    let window_cls = class(b"NSWindow\0");

    let alloc: unsafe extern "C" fn(Id, Sel) -> Id = unsafe { std::mem::transmute(objc_msg_send_ptr()) };
    let window = unsafe { alloc(window_cls, sel(b"alloc\0")) };
    if window.is_null() {
        return Err("NSWindow.alloc returned null".to_owned());
    }

    let rect = NSRect {
        origin: NSPoint { x: 0.0, y: 0.0 },
        size: NSSize {
            width: width_css as c_double,
            height: height_css as c_double,
        },
    };
    let style_mask: u64 = (1 << 0) | (1 << 1) | (1 << 2) | (1 << 3);
    let backing: u64 = 2;

    let init: unsafe extern "C" fn(Id, Sel, NSRect, u64, u64, ObjcBool) -> Id =
        unsafe { std::mem::transmute(objc_msg_send_ptr()) };
    let window = unsafe {
        init(
            window,
            sel(b"initWithContentRect:styleMask:backing:defer:\0"),
            rect,
            style_mask,
            backing,
            NO,
        )
    };
    if window.is_null() {
        return Err("NSWindow.initWithContentRect returned null".to_owned());
    }

    let title = nsstring(title)?;
    let set_title: unsafe extern "C" fn(Id, Sel, Id) = unsafe { std::mem::transmute(objc_msg_send_ptr()) };
    unsafe { set_title(window, sel(b"setTitle:\0"), title) };
    unsafe { CFRelease(title as *const c_void) };

    let released: unsafe extern "C" fn(Id, Sel, ObjcBool) = unsafe { std::mem::transmute(objc_msg_send_ptr()) };
    unsafe { released(window, sel(b"setReleasedWhenClosed:\0"), NO) };

    let make_key: unsafe extern "C" fn(Id, Sel, Id) = unsafe { std::mem::transmute(objc_msg_send_ptr()) };
    unsafe { make_key(window, sel(b"makeKeyAndOrderFront:\0"), std::ptr::null_mut()) };

    Ok(window)
}

fn nsstring(input: &str) -> Result<Id, String> {
    const K_CFSTRING_ENCODING_UTF8: u32 = 0x0800_0100;
    let bytes = input.as_bytes();
    let len: isize = bytes
        .len()
        .try_into()
        .map_err(|_| "String is too large".to_owned())?;
    let cf = unsafe {
        CFStringCreateWithBytes(
            std::ptr::null(),
            bytes.as_ptr(),
            len,
            K_CFSTRING_ENCODING_UTF8,
            0,
        )
    };
    if cf.is_null() {
        return Err("CFStringCreateWithBytes failed".to_owned());
    }
    Ok(cf as Id)
}

fn date_with_interval(seconds: c_double) -> Id {
    let cls = class(b"NSDate\0");
    let f: unsafe extern "C" fn(Id, Sel, c_double) -> Id =
        unsafe { std::mem::transmute(objc_msg_send_ptr()) };
    unsafe { f(cls, sel(b"dateWithTimeIntervalSinceNow:\0"), seconds) }
}

#[cfg(target_arch = "x86_64")]
fn view_bounds(view: Id) -> NSRect {
    let mut rect = NSRect {
        origin: NSPoint { x: 0.0, y: 0.0 },
        size: NSSize {
            width: 0.0,
            height: 0.0,
        },
    };
    let f: unsafe extern "C" fn(*mut NSRect, Id, Sel) =
        unsafe { std::mem::transmute(objc_msg_send_stret_ptr()) };
    unsafe { f(&mut rect, view, sel(b"bounds\0")) };
    rect
}

#[cfg(target_arch = "aarch64")]
fn view_bounds(view: Id) -> NSRect {
    let f: unsafe extern "C" fn(Id, Sel) -> NSRect =
        unsafe { std::mem::transmute(objc_msg_send_ptr()) };
    unsafe { f(view, sel(b"bounds\0")) }
}

fn event_location_in_window(event: Id) -> NSPoint {
    let f: unsafe extern "C" fn(Id, Sel) -> NSPoint =
        unsafe { std::mem::transmute(objc_msg_send_ptr()) };
    unsafe { f(event, sel(b"locationInWindow\0")) }
}

fn class(name: &'static [u8]) -> Id {
    debug_assert!(name.last() == Some(&0));
    unsafe { objc_getClass(name.as_ptr().cast::<c_char>()) as Id }
}

fn sel(name: &'static [u8]) -> Sel {
    debug_assert!(name.last() == Some(&0));
    unsafe { sel_registerName(name.as_ptr().cast::<c_char>()) }
}

fn objc_msg_send_ptr() -> *const c_void {
    objc_msgSend as *const () as *const c_void
}

#[cfg(target_arch = "x86_64")]
fn objc_msg_send_stret_ptr() -> *const c_void {
    unsafe { objc_msgSend_stret as *const () as *const c_void }
}
