use core::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void};
use std::ffi::CString;

type Atom = c_ulong;
type Bool = c_int;
type Display = c_void;
type GC = *mut c_void;
type Window = c_ulong;

const EVENT_TYPE_KEY_PRESS: c_int = 2;
const EVENT_TYPE_EXPOSE: c_int = 12;
const EVENT_TYPE_CLIENT_MESSAGE: c_int = 33;

const EVENT_MASK_KEY_PRESS: c_long = 1 << 0;
const EVENT_MASK_EXPOSURE: c_long = 1 << 15;

#[repr(C)]
struct XExposeEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: Bool,
    display: *mut Display,
    window: Window,
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
    count: c_int,
}

#[repr(C)]
union XClientMessageData {
    l: [c_long; 5],
}

#[repr(C)]
struct XClientMessageEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: Bool,
    display: *mut Display,
    window: Window,
    message_type: Atom,
    format: c_int,
    data: XClientMessageData,
}

#[repr(C)]
struct XEvent {
    inner: [c_long; 24],
}

impl XEvent {
    fn event_type(&self) -> c_int {
        unsafe { *(self as *const XEvent as *const c_int) }
    }
}

#[link(name = "X11")]
unsafe extern "C" {
    fn XOpenDisplay(display_name: *const c_char) -> *mut Display;
    fn XCloseDisplay(display: *mut Display) -> c_int;

    fn XDefaultScreen(display: *mut Display) -> c_int;
    fn XRootWindow(display: *mut Display, screen_number: c_int) -> Window;
    fn XBlackPixel(display: *mut Display, screen_number: c_int) -> c_ulong;
    fn XWhitePixel(display: *mut Display, screen_number: c_int) -> c_ulong;

    fn XCreateSimpleWindow(
        display: *mut Display,
        parent: Window,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
        border_width: c_uint,
        border: c_ulong,
        background: c_ulong,
    ) -> Window;

    fn XStoreName(display: *mut Display, window: Window, window_name: *const c_char) -> c_int;
    fn XSelectInput(display: *mut Display, window: Window, event_mask: c_long) -> c_int;
    fn XMapWindow(display: *mut Display, window: Window) -> c_int;

    fn XDefaultGC(display: *mut Display, screen_number: c_int) -> GC;
    fn XSetForeground(display: *mut Display, gc: GC, foreground: c_ulong) -> c_int;
    fn XSetBackground(display: *mut Display, gc: GC, background: c_ulong) -> c_int;
    fn XDrawString(
        display: *mut Display,
        drawable: Window,
        gc: GC,
        x: c_int,
        y: c_int,
        string: *const c_char,
        length: c_int,
    ) -> c_int;

    fn XInternAtom(display: *mut Display, atom_name: *const c_char, only_if_exists: Bool)
        -> Atom;
    fn XSetWMProtocols(
        display: *mut Display,
        window: Window,
        protocols: *mut Atom,
        count: c_int,
    ) -> c_int;

    fn XNextEvent(display: *mut Display, event_return: *mut XEvent) -> c_int;
    fn XDestroyWindow(display: *mut Display, window: Window) -> c_int;
    fn XFlush(display: *mut Display) -> c_int;
}

pub fn run_hello_world_window() -> Result<(), String> {
    let display = unsafe { XOpenDisplay(std::ptr::null()) };
    if display.is_null() {
        return Err("XOpenDisplay failed: is $DISPLAY set and an X server available?".to_owned());
    }

    let result = run_hello_world_window_with_display(display);

    unsafe {
        XCloseDisplay(display);
    }

    result
}

fn run_hello_world_window_with_display(display: *mut Display) -> Result<(), String> {
    let screen = unsafe { XDefaultScreen(display) };

    let root_window = unsafe { XRootWindow(display, screen) };
    let black_pixel = unsafe { XBlackPixel(display, screen) };
    let white_pixel = unsafe { XWhitePixel(display, screen) };

    let window_width: c_uint = 640;
    let window_height: c_uint = 480;
    let window = unsafe {
        XCreateSimpleWindow(
            display,
            root_window,
            0,
            0,
            window_width,
            window_height,
            1,
            black_pixel,
            white_pixel,
        )
    };

    let window_title = CString::new("Hello World").map_err(|_| "invalid window title".to_owned())?;
    unsafe {
        XStoreName(display, window, window_title.as_ptr());
    }

    let wm_delete_window_atom_name =
        CString::new("WM_DELETE_WINDOW").map_err(|_| "invalid atom name".to_owned())?;
    let wm_delete_window = unsafe { XInternAtom(display, wm_delete_window_atom_name.as_ptr(), 0) };
    let mut wm_protocols = [wm_delete_window];
    unsafe {
        XSetWMProtocols(
            display,
            window,
            wm_protocols.as_mut_ptr(),
            wm_protocols.len() as c_int,
        );
    }

    unsafe {
        XSelectInput(display, window, EVENT_MASK_EXPOSURE | EVENT_MASK_KEY_PRESS);
        XMapWindow(display, window);
    }

    let gc = unsafe { XDefaultGC(display, screen) };
    unsafe {
        XSetForeground(display, gc, black_pixel);
        XSetBackground(display, gc, white_pixel);
    }

    let text = CString::new("Hello World").map_err(|_| "invalid window text".to_owned())?;
    let text_len = text.as_bytes().len();
    let text_len: c_int = text_len
        .try_into()
        .map_err(|_| "window text length out of range".to_owned())?;

    loop {
        let mut event = XEvent { inner: [0; 24] };
        unsafe {
            XNextEvent(display, &mut event);
        }

        match event.event_type() {
            EVENT_TYPE_EXPOSE => {
                let expose: &XExposeEvent = unsafe { &*(event.inner.as_ptr() as *const XExposeEvent) };
                if expose.count != 0 {
                    continue;
                }

                unsafe {
                    XDrawString(display, window, gc, 24, 48, text.as_ptr(), text_len);
                    XFlush(display);
                }
            }
            EVENT_TYPE_KEY_PRESS => break,
            EVENT_TYPE_CLIENT_MESSAGE => {
                let message: &XClientMessageEvent =
                    unsafe { &*(event.inner.as_ptr() as *const XClientMessageEvent) };
                let data = unsafe { message.data.l };
                if data[0] as c_ulong == wm_delete_window {
                    break;
                }
            }
            _ => {}
        }
    }

    unsafe {
        XDestroyWindow(display, window);
        XFlush(display);
    }

    Ok(())
}
