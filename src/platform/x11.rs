use super::WindowOptions;
use crate::image::RgbImage;
use crate::render::{Painter, TextMeasurer, TextStyle, Viewport};
use core::ffi::{c_char, c_int, c_long, c_short, c_uint, c_ulong, c_ushort, c_void};
use std::ffi::CString;

type Atom = c_ulong;
type Bool = c_int;
type Display = c_void;
type Drawable = c_ulong;
type Font = c_ulong;
type GC = *mut c_void;
type Pixmap = c_ulong;
type Window = c_ulong;

#[repr(C)]
struct Visual {
    ext_data: *mut c_void,
    visualid: c_ulong,
    class_: c_int,
    red_mask: c_ulong,
    green_mask: c_ulong,
    blue_mask: c_ulong,
    bits_per_rgb: c_int,
    map_entries: c_int,
}

const ALL_PLANES: c_ulong = !0;
const EVENT_TYPE_KEY_PRESS: c_int = 2;
const EVENT_TYPE_EXPOSE: c_int = 12;
const EVENT_TYPE_CONFIGURE_NOTIFY: c_int = 22;
const EVENT_TYPE_CLIENT_MESSAGE: c_int = 33;

const EVENT_MASK_KEY_PRESS: c_long = 1 << 0;
const EVENT_MASK_EXPOSURE: c_long = 1 << 15;
const EVENT_MASK_STRUCTURE_NOTIFY: c_long = 1 << 17;

const IMAGE_FORMAT_Z_PIXMAP: c_int = 2;

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
struct XConfigureEvent {
    type_: c_int,
    serial: c_ulong,
    send_event: Bool,
    display: *mut Display,
    event: Window,
    window: Window,
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
    border_width: c_int,
    above: Window,
    override_redirect: Bool,
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
struct XCharStruct {
    lbearing: c_short,
    rbearing: c_short,
    width: c_short,
    ascent: c_short,
    descent: c_short,
    attributes: c_ushort,
}

#[repr(C)]
struct XFontStruct {
    ext_data: *mut c_void,
    fid: Font,
    direction: c_uint,
    min_char_or_byte2: c_uint,
    max_char_or_byte2: c_uint,
    min_byte1: c_uint,
    max_byte1: c_uint,
    all_chars_exist: Bool,
    default_char: c_uint,
    n_properties: c_int,
    properties: *mut c_void,
    min_bounds: XCharStruct,
    max_bounds: XCharStruct,
    per_char: *mut XCharStruct,
    ascent: c_int,
    descent: c_int,
}

#[repr(C)]
struct XImageFuncs {
    create_image: Option<
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
    destroy_image: Option<unsafe extern "C" fn(*mut XImage) -> c_int>,
    get_pixel: Option<unsafe extern "C" fn(*mut XImage, c_int, c_int) -> c_ulong>,
    put_pixel: Option<unsafe extern "C" fn(*mut XImage, c_int, c_int, c_ulong) -> c_int>,
    sub_image: Option<unsafe extern "C" fn(*mut XImage, c_int, c_int, c_uint, c_uint) -> *mut XImage>,
    add_pixel: Option<unsafe extern "C" fn(*mut XImage, c_long) -> c_int>,
}

#[repr(C)]
struct XImage {
    width: c_int,
    height: c_int,
    xoffset: c_int,
    format: c_int,
    data: *mut c_char,
    byte_order: c_int,
    bitmap_unit: c_int,
    bitmap_bit_order: c_int,
    bitmap_pad: c_int,
    depth: c_int,
    bytes_per_line: c_int,
    bits_per_pixel: c_int,
    red_mask: c_ulong,
    green_mask: c_ulong,
    blue_mask: c_ulong,
    obdata: *mut c_void,
    f: XImageFuncs,
}

struct XImageHandle(*mut XImage);

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
    fn XDefaultVisual(display: *mut Display, screen_number: c_int) -> *mut Visual;
    fn XDefaultDepth(display: *mut Display, screen_number: c_int) -> c_int;
    fn XRootWindow(display: *mut Display, screen_number: c_int) -> Window;
    fn XBlackPixel(display: *mut Display, screen_number: c_int) -> c_ulong;
    fn XWhitePixel(display: *mut Display, screen_number: c_int) -> c_ulong;

    fn XCreatePixmap(
        display: *mut Display,
        drawable: Drawable,
        width: c_uint,
        height: c_uint,
        depth: c_uint,
    ) -> Pixmap;

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
    fn XSetFont(display: *mut Display, gc: GC, font: Font) -> c_int;
    fn XDrawString(
        display: *mut Display,
        drawable: Drawable,
        gc: GC,
        x: c_int,
        y: c_int,
        string: *const c_char,
        length: c_int,
    ) -> c_int;
    fn XFillRectangle(
        display: *mut Display,
        drawable: Drawable,
        gc: GC,
        x: c_int,
        y: c_int,
        width: c_uint,
        height: c_uint,
    ) -> c_int;
    fn XCopyArea(
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

    fn XLoadQueryFont(display: *mut Display, name: *const c_char) -> *mut XFontStruct;
    fn XFreeFont(display: *mut Display, font_struct: *mut XFontStruct) -> c_int;
    fn XFreePixmap(display: *mut Display, pixmap: Pixmap) -> c_int;
    fn XTextWidth(font_struct: *mut XFontStruct, string: *const c_char, count: c_int) -> c_int;

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
    fn XSync(display: *mut Display, discard: Bool) -> c_int;

    fn XGetImage(
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

pub fn run_window<F>(title: &str, options: WindowOptions, render: F) -> Result<(), String>
where
    F: FnMut(&mut dyn Painter, Viewport) -> Result<(), String>,
{
    let display = unsafe { XOpenDisplay(std::ptr::null()) };
    if display.is_null() {
        return Err("XOpenDisplay failed: is $DISPLAY set and an X server available?".to_owned());
    }

    let result = run_window_with_display(display, title, options, render);

    unsafe {
        XCloseDisplay(display);
    }

    result
}

fn run_window_with_display<F>(
    display: *mut Display,
    title: &str,
    options: WindowOptions,
    mut render: F,
) -> Result<(), String>
where
    F: FnMut(&mut dyn Painter, Viewport) -> Result<(), String>,
{
    let screen = unsafe { XDefaultScreen(display) };
    let visual = unsafe { XDefaultVisual(display, screen) };
    if visual.is_null() {
        return Err("XDefaultVisual returned null".to_owned());
    }
    let visual_masks = unsafe { ((*visual).red_mask, (*visual).green_mask, (*visual).blue_mask) };
    let root_window = unsafe { XRootWindow(display, screen) };
    let black_pixel = unsafe { XBlackPixel(display, screen) };
    let white_pixel = unsafe { XWhitePixel(display, screen) };

    let initial_width: c_uint = 640;
    let initial_height: c_uint = 480;

    let window = unsafe {
        XCreateSimpleWindow(
            display,
            root_window,
            0,
            0,
            initial_width,
            initial_height,
            1,
            black_pixel,
            white_pixel,
        )
    };

    let window_title = CString::new(title).map_err(|_| "Window title contains a NUL byte".to_owned())?;
    unsafe {
        XStoreName(display, window, window_title.as_ptr());
    }

    let wm_protocols_atom_name =
        CString::new("WM_PROTOCOLS").map_err(|_| "Invalid atom name".to_owned())?;
    let wm_protocols_atom = unsafe { XInternAtom(display, wm_protocols_atom_name.as_ptr(), 0) };

    let wm_delete_window_atom_name =
        CString::new("WM_DELETE_WINDOW").map_err(|_| "Invalid atom name".to_owned())?;
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
        XSelectInput(
            display,
            window,
            EVENT_MASK_EXPOSURE | EVENT_MASK_KEY_PRESS | EVENT_MASK_STRUCTURE_NOTIFY,
        );
        XMapWindow(display, window);
    }

    let depth_i32 = unsafe { XDefaultDepth(display, screen) };
    let depth: c_uint = depth_i32
        .try_into()
        .map_err(|_| format!("XDefaultDepth returned an invalid value: {depth_i32}"))?;

    let gc = unsafe { XDefaultGC(display, screen) };
    unsafe {
        XSetForeground(display, gc, black_pixel);
        XSetBackground(display, gc, white_pixel);
    }

    let mut metrics = FontMetrics::Approx {
        char_width_px: 8,
        line_height_px: 16,
    };

    let font_name = CString::new("fixed").map_err(|_| "Invalid font name".to_owned())?;
    let font = unsafe { XLoadQueryFont(display, font_name.as_ptr()) };
    if !font.is_null() {
        unsafe {
            XSetFont(display, gc, (*font).fid);
        }
        let line_height_px = unsafe { ((*font).ascent + (*font).descent).max(1) };
        metrics = FontMetrics::X11 {
            font,
            line_height_px,
        };
    }

    let back_buffer = unsafe { XCreatePixmap(display, window, initial_width, initial_height, depth) };
    if back_buffer == 0 {
        return Err("XCreatePixmap failed".to_owned());
    }

    let mut painter = X11Painter {
        display,
        window,
        gc,
        back_buffer,
        back_buffer_width: initial_width,
        back_buffer_height: initial_height,
        back_buffer_depth: depth,
        black_pixel,
        white_pixel,
        visual_masks,
        metrics,
    };

    let mut viewport = Viewport {
        width_px: initial_width as i32,
        height_px: initial_height as i32,
    };

    let mut screenshot_path = options.screenshot_path;

    let loop_result = (|| {
        let mut needs_redraw = true;

        loop {
            let mut event = XEvent { inner: [0; 24] };
            unsafe {
                XNextEvent(display, &mut event);
            }

            match event.event_type() {
                EVENT_TYPE_EXPOSE => {
                    let expose: &XExposeEvent =
                        unsafe { &*(event.inner.as_ptr() as *const XExposeEvent) };
                    if expose.count == 0 {
                        needs_redraw = true;
                    }
                }
                EVENT_TYPE_CONFIGURE_NOTIFY => {
                    let configure: &XConfigureEvent =
                        unsafe { &*(event.inner.as_ptr() as *const XConfigureEvent) };
                    viewport = Viewport {
                        width_px: configure.width,
                        height_px: configure.height,
                    };
                    needs_redraw = true;
                }
                EVENT_TYPE_KEY_PRESS => break,
                EVENT_TYPE_CLIENT_MESSAGE => {
                    let message: &XClientMessageEvent =
                        unsafe { &*(event.inner.as_ptr() as *const XClientMessageEvent) };
                    let data = unsafe { message.data.l };
                    if message.message_type == wm_protocols_atom
                        && data[0] as c_ulong == wm_delete_window
                    {
                        break;
                    }
                }
                _ => {}
            }

            if needs_redraw {
                painter.ensure_back_buffer(viewport)?;
                render(&mut painter, viewport)?;
                needs_redraw = false;

                if let Some(path) = screenshot_path.take() {
                    unsafe {
                        XSync(display, 0);
                    }
                    let rgb = capture_back_buffer_rgb(&painter)?;
                    crate::png::write_rgb_png(&path, &rgb)?;
                    break;
                }
            }
        }

        Ok(())
    })();

    unsafe {
        if let FontMetrics::X11 { font, .. } = painter.metrics {
            XFreeFont(display, font);
        }
        XFreePixmap(display, painter.back_buffer);
        XDestroyWindow(display, window);
        XFlush(display);
    }

    loop_result
}

fn capture_back_buffer_rgb(painter: &X11Painter) -> Result<RgbImage, String> {
    let width_u32: u32 = painter
        .back_buffer_width
        .try_into()
        .map_err(|_| "Screenshot width out of range".to_owned())?;
    let height_u32: u32 = painter
        .back_buffer_height
        .try_into()
        .map_err(|_| "Screenshot height out of range".to_owned())?;

    let ximage = unsafe {
        XGetImage(
            painter.display,
            painter.back_buffer,
            0,
            0,
            painter.back_buffer_width,
            painter.back_buffer_height,
            ALL_PLANES,
            IMAGE_FORMAT_Z_PIXMAP,
        )
    };
    if ximage.is_null() {
        return Err("XGetImage returned null".to_owned());
    }
    let ximage = XImageHandle(ximage);

    let (masks, get_pixel) = unsafe {
        let masks = ((*ximage.0).red_mask, (*ximage.0).green_mask, (*ximage.0).blue_mask);
        let masks = if masks.0 == 0 && masks.1 == 0 && masks.2 == 0 {
            painter.visual_masks
        } else {
            masks
        };

        (masks, (*ximage.0).f.get_pixel)
    };

    let get_pixel = get_pixel.ok_or_else(|| "XImage is missing get_pixel".to_owned())?;

    let width = width_u32 as usize;
    let height = height_u32 as usize;
    let expected_len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(3))
        .ok_or_else(|| "Screenshot buffer size overflow".to_owned())?;
    let mut rgb = Vec::with_capacity(expected_len);

    for y in 0..height {
        for x in 0..width {
            let pixel_u64 = unsafe { get_pixel(ximage.0, x as c_int, y as c_int) as u64 };
            let r = extract_channel(pixel_u64, masks.0 as u64);
            let g = extract_channel(pixel_u64, masks.1 as u64);
            let b = extract_channel(pixel_u64, masks.2 as u64);
            rgb.push(r);
            rgb.push(g);
            rgb.push(b);
        }
    }

    RgbImage::new(width_u32, height_u32, rgb)
}

fn extract_channel(pixel: u64, mask: u64) -> u8 {
    if mask == 0 {
        return 0;
    }

    let shift = mask.trailing_zeros();
    let bits = mask.count_ones();
    if bits == 0 || shift >= 64 {
        return 0;
    }

    let value = (pixel & mask) >> shift;
    let max = if bits == 64 { u64::MAX } else { (1u64 << bits) - 1 };
    if max == 0 {
        return 0;
    }

    if max == 255 {
        return value as u8;
    }

    let scaled = (value * 255 + max / 2) / max;
    scaled as u8
}

enum FontMetrics {
    X11 { font: *mut XFontStruct, line_height_px: i32 },
    Approx { char_width_px: i32, line_height_px: i32 },
}

struct X11Painter {
    display: *mut Display,
    window: Window,
    gc: GC,
    back_buffer: Pixmap,
    back_buffer_width: c_uint,
    back_buffer_height: c_uint,
    back_buffer_depth: c_uint,
    black_pixel: c_ulong,
    white_pixel: c_ulong,
    visual_masks: (c_ulong, c_ulong, c_ulong),
    metrics: FontMetrics,
}

impl X11Painter {
    fn ensure_back_buffer(&mut self, viewport: Viewport) -> Result<(), String> {
        let width_i32 = viewport.width_px;
        let height_i32 = viewport.height_px;
        if width_i32 <= 0 || height_i32 <= 0 {
            return Err(format!(
                "Invalid window size: {width_i32}x{height_i32}"
            ));
        }

        let width: c_uint = width_i32
            .try_into()
            .map_err(|_| "Window width out of range".to_owned())?;
        let height: c_uint = height_i32
            .try_into()
            .map_err(|_| "Window height out of range".to_owned())?;

        if width == self.back_buffer_width && height == self.back_buffer_height {
            return Ok(());
        }

        let back_buffer =
            unsafe { XCreatePixmap(self.display, self.window, width, height, self.back_buffer_depth) };
        if back_buffer == 0 {
            return Err("XCreatePixmap failed during resize".to_owned());
        }

        unsafe {
            XFreePixmap(self.display, self.back_buffer);
        }

        self.back_buffer = back_buffer;
        self.back_buffer_width = width;
        self.back_buffer_height = height;
        Ok(())
    }
}

impl TextMeasurer for X11Painter {
    fn line_height_px(&self) -> i32 {
        match self.metrics {
            FontMetrics::X11 { line_height_px, .. } => line_height_px,
            FontMetrics::Approx { line_height_px, .. } => line_height_px,
        }
    }

    fn text_width_px(&self, text: &str) -> Result<i32, String> {
        let len: c_int = text
            .len()
            .try_into()
            .map_err(|_| "text length out of range for X11".to_owned())?;

        match self.metrics {
            FontMetrics::X11 { font, .. } => unsafe {
                Ok(XTextWidth(font, text.as_ptr().cast::<c_char>(), len))
            },
            FontMetrics::Approx { char_width_px, .. } => {
                let width = char_width_px.saturating_mul(text.as_bytes().len() as i32);
                Ok(width)
            }
        }
    }
}

impl Painter for X11Painter {
    fn clear(&mut self) -> Result<(), String> {
        unsafe {
            XSetForeground(self.display, self.gc, self.white_pixel);
            XFillRectangle(
                self.display,
                self.back_buffer,
                self.gc,
                0,
                0,
                self.back_buffer_width,
                self.back_buffer_height,
            );
            XSetForeground(self.display, self.gc, self.black_pixel);
        }
        Ok(())
    }

    fn draw_text(
        &mut self,
        x_px: i32,
        y_px: i32,
        text: &str,
        style: TextStyle,
    ) -> Result<(), String> {
        let len: c_int = text
            .len()
            .try_into()
            .map_err(|_| "text length out of range for X11".to_owned())?;

        unsafe {
            XDrawString(
                self.display,
                self.back_buffer,
                self.gc,
                x_px,
                y_px,
                text.as_ptr().cast::<c_char>(),
                len,
            );
            if style.bold {
                XDrawString(
                    self.display,
                    self.back_buffer,
                    self.gc,
                    x_px.saturating_add(1),
                    y_px,
                    text.as_ptr().cast::<c_char>(),
                    len,
                );
            }
        }

        Ok(())
    }

    fn flush(&mut self) -> Result<(), String> {
        unsafe {
            XCopyArea(
                self.display,
                self.back_buffer,
                self.window,
                self.gc,
                0,
                0,
                self.back_buffer_width,
                self.back_buffer_height,
                0,
                0,
            );
            XFlush(self.display);
        }
        Ok(())
    }
}
