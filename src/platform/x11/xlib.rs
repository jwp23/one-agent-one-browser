use core::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void};

pub type Atom = c_ulong;
pub type Bool = c_int;
pub type Colormap = c_ulong;
pub type Display = c_void;
pub type Drawable = c_ulong;
pub type GC = *mut c_void;
pub type Pixmap = c_ulong;
pub type Window = c_ulong;

#[repr(C)]
pub struct Visual {
    pub ext_data: *mut c_void,
    pub visualid: c_ulong,
    pub class_: c_int,
    pub red_mask: c_ulong,
    pub green_mask: c_ulong,
    pub blue_mask: c_ulong,
    pub bits_per_rgb: c_int,
    pub map_entries: c_int,
}

pub const ALL_PLANES: c_ulong = !0;
pub const EVENT_TYPE_KEY_PRESS: c_int = 2;
pub const EVENT_TYPE_EXPOSE: c_int = 12;
pub const EVENT_TYPE_CONFIGURE_NOTIFY: c_int = 22;
pub const EVENT_TYPE_CLIENT_MESSAGE: c_int = 33;

pub const EVENT_MASK_KEY_PRESS: c_long = 1 << 0;
pub const EVENT_MASK_EXPOSURE: c_long = 1 << 15;
pub const EVENT_MASK_STRUCTURE_NOTIFY: c_long = 1 << 17;

pub const IMAGE_FORMAT_Z_PIXMAP: c_int = 2;

#[repr(C)]
pub struct XExposeEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
    pub count: c_int,
}

#[repr(C)]
pub struct XConfigureEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub event: Window,
    pub window: Window,
    pub x: c_int,
    pub y: c_int,
    pub width: c_int,
    pub height: c_int,
    pub border_width: c_int,
    pub above: Window,
    pub override_redirect: Bool,
}

#[repr(C)]
pub union XClientMessageData {
    pub l: [c_long; 5],
}

#[repr(C)]
pub struct XClientMessageEvent {
    pub type_: c_int,
    pub serial: c_ulong,
    pub send_event: Bool,
    pub display: *mut Display,
    pub window: Window,
    pub message_type: Atom,
    pub format: c_int,
    pub data: XClientMessageData,
}

#[repr(C)]
pub struct XImageFuncs {
    pub create_image: Option<
        unsafe extern "C" fn(
            *mut Display,
            *mut c_void,
            c_uint,
            c_int,
            c_int,
            *mut c_char,
            c_uint,
            c_uint,
            c_int,
            c_int,
        ) -> *mut XImage,
    >,
    pub destroy_image: Option<unsafe extern "C" fn(*mut XImage) -> c_int>,
    pub get_pixel: Option<unsafe extern "C" fn(*mut XImage, c_int, c_int) -> c_ulong>,
    pub put_pixel: Option<unsafe extern "C" fn(*mut XImage, c_int, c_int, c_ulong) -> c_int>,
    pub sub_image:
        Option<unsafe extern "C" fn(*mut XImage, c_int, c_int, c_uint, c_uint) -> *mut XImage>,
    pub add_pixel: Option<unsafe extern "C" fn(*mut XImage, c_long) -> c_int>,
}

#[repr(C)]
pub struct XImage {
    pub width: c_int,
    pub height: c_int,
    pub xoffset: c_int,
    pub format: c_int,
    pub data: *mut c_char,
    pub byte_order: c_int,
    pub bitmap_unit: c_int,
    pub bitmap_bit_order: c_int,
    pub bitmap_pad: c_int,
    pub depth: c_int,
    pub bytes_per_line: c_int,
    pub bits_per_pixel: c_int,
    pub red_mask: c_ulong,
    pub green_mask: c_ulong,
    pub blue_mask: c_ulong,
    pub obdata: *mut c_void,
    pub f: XImageFuncs,
}

pub struct XImageHandle(pub *mut XImage);

impl Drop for XImageHandle {
    fn drop(&mut self) {
        let image = self.0;
        if image.is_null() {
            return;
        }
        unsafe {
            if let Some(destroy) = (*image).f.destroy_image {
                destroy(image);
            }
        }
    }
}

#[repr(C)]
pub struct XEvent {
    pub inner: [c_long; 24],
}

impl XEvent {
    pub fn event_type(&self) -> c_int {
        unsafe { *(self as *const XEvent as *const c_int) }
    }
}

#[link(name = "X11")]
unsafe extern "C" {
    pub fn XOpenDisplay(display_name: *const c_char) -> *mut Display;
    pub fn XCloseDisplay(display: *mut Display) -> c_int;

    pub fn XDefaultScreen(display: *mut Display) -> c_int;
    pub fn XDefaultVisual(display: *mut Display, screen_number: c_int) -> *mut Visual;
    pub fn XDefaultDepth(display: *mut Display, screen_number: c_int) -> c_int;
    pub fn XDefaultColormap(display: *mut Display, screen_number: c_int) -> Colormap;
    pub fn XRootWindow(display: *mut Display, screen_number: c_int) -> Window;
    pub fn XBlackPixel(display: *mut Display, screen_number: c_int) -> c_ulong;
    pub fn XWhitePixel(display: *mut Display, screen_number: c_int) -> c_ulong;

    pub fn XCreatePixmap(
        display: *mut Display,
        drawable: Drawable,
        width: c_uint,
        height: c_uint,
        depth: c_uint,
    ) -> Pixmap;

    pub fn XCreateSimpleWindow(
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

    pub fn XStoreName(display: *mut Display, window: Window, window_name: *const c_char) -> c_int;
    pub fn XSelectInput(display: *mut Display, window: Window, event_mask: c_long) -> c_int;
    pub fn XMapWindow(display: *mut Display, window: Window) -> c_int;

    pub fn XDefaultGC(display: *mut Display, screen_number: c_int) -> GC;
    pub fn XSetForeground(display: *mut Display, gc: GC, foreground: c_ulong) -> c_int;
    pub fn XSetBackground(display: *mut Display, gc: GC, background: c_ulong) -> c_int;
    pub fn XFillRectangle(
        display: *mut Display,
        drawable: Drawable,
        gc: GC,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
    ) -> c_int;
    pub fn XCopyArea(
        display: *mut Display,
        src: Drawable,
        dest: Drawable,
        gc: GC,
        src_x: c_int,
        src_y: c_int,
        width: c_uint,
        height: c_uint,
        dest_x: c_int,
        dest_y: c_int,
    ) -> c_int;

    pub fn XFreePixmap(display: *mut Display, pixmap: Pixmap) -> c_int;

    pub fn XInternAtom(display: *mut Display, atom_name: *const c_char, only_if_exists: Bool)
        -> Atom;
    pub fn XSetWMProtocols(
        display: *mut Display,
        window: Window,
        protocols: *mut Atom,
        count: c_int,
    ) -> c_int;

    pub fn XPending(display: *mut Display) -> c_int;
    pub fn XNextEvent(display: *mut Display, event_return: *mut XEvent) -> c_int;
    pub fn XDestroyWindow(display: *mut Display, window: Window) -> c_int;
    pub fn XFlush(display: *mut Display) -> c_int;
    pub fn XSync(display: *mut Display, discard: Bool) -> c_int;

    pub fn XGetImage(
        display: *mut Display,
        drawable: Drawable,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
        plane_mask: c_ulong,
        format: c_int,
    ) -> *mut XImage;
}
