use crate::geom::Color;
use crate::render::TextStyle;
use core::ffi::{c_char, c_double, c_int, c_void};
use std::ffi::{CStr, CString};

use super::xlib::{Display, Drawable, Visual};

#[repr(C)]
pub(super) struct cairo_t {
    _private: [u8; 0],
}

#[repr(C)]
pub(super) struct cairo_surface_t {
    _private: [u8; 0],
}

#[allow(non_camel_case_types)]
type cairo_status_t = c_int;
const CAIRO_STATUS_SUCCESS: cairo_status_t = 0;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum cairo_format_t {
    CAIRO_FORMAT_ARGB32 = 0,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
enum cairo_font_slant_t {
    CAIRO_FONT_SLANT_NORMAL = 0,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
enum cairo_font_weight_t {
    CAIRO_FONT_WEIGHT_NORMAL = 0,
    CAIRO_FONT_WEIGHT_BOLD = 1,
}

#[link(name = "cairo")]
unsafe extern "C" {
    fn cairo_xlib_surface_create(
        dpy: *mut Display,
        drawable: Drawable,
        visual: *mut Visual,
        width: c_int,
        height: c_int,
    ) -> *mut cairo_surface_t;
    fn cairo_surface_destroy(surface: *mut cairo_surface_t);
    fn cairo_surface_status(surface: *mut cairo_surface_t) -> cairo_status_t;
    fn cairo_surface_flush(surface: *mut cairo_surface_t);

    fn cairo_create(surface: *mut cairo_surface_t) -> *mut cairo_t;
    fn cairo_destroy(cr: *mut cairo_t);
    fn cairo_status(cr: *mut cairo_t) -> cairo_status_t;
    fn cairo_status_to_string(status: cairo_status_t) -> *const c_char;

    fn cairo_set_source_rgba(
        cr: *mut cairo_t,
        red: c_double,
        green: c_double,
        blue: c_double,
        alpha: c_double,
    );
    fn cairo_rectangle(cr: *mut cairo_t, x: c_double, y: c_double, width: c_double, height: c_double);
    fn cairo_fill(cr: *mut cairo_t);
    fn cairo_new_path(cr: *mut cairo_t);

    fn cairo_move_to(cr: *mut cairo_t, x: c_double, y: c_double);
    fn cairo_line_to(cr: *mut cairo_t, x: c_double, y: c_double);
    fn cairo_arc(
        cr: *mut cairo_t,
        xc: c_double,
        yc: c_double,
        radius: c_double,
        angle1: c_double,
        angle2: c_double,
    );
    fn cairo_close_path(cr: *mut cairo_t);
    fn cairo_set_line_width(cr: *mut cairo_t, width: c_double);
    fn cairo_stroke(cr: *mut cairo_t);

    fn cairo_save(cr: *mut cairo_t);
    fn cairo_restore(cr: *mut cairo_t);
    fn cairo_translate(cr: *mut cairo_t, tx: c_double, ty: c_double);
    fn cairo_scale(cr: *mut cairo_t, sx: c_double, sy: c_double);
    fn cairo_set_source_surface(cr: *mut cairo_t, surface: *mut cairo_surface_t, x: c_double, y: c_double);
    fn cairo_paint(cr: *mut cairo_t);
    fn cairo_paint_with_alpha(cr: *mut cairo_t, alpha: c_double);
    fn cairo_push_group(cr: *mut cairo_t);
    fn cairo_pop_group_to_source(cr: *mut cairo_t);

    fn cairo_select_font_face(
        cr: *mut cairo_t,
        family: *const c_char,
        slant: cairo_font_slant_t,
        weight: cairo_font_weight_t,
    );
    fn cairo_set_font_size(cr: *mut cairo_t, size: c_double);
    fn cairo_show_text(cr: *mut cairo_t, utf8: *const c_char);

    fn cairo_image_surface_create_for_data(
        data: *mut u8,
        format: cairo_format_t,
        width: c_int,
        height: c_int,
        stride: c_int,
    ) -> *mut cairo_surface_t;
}

#[repr(C)]
struct RsvgHandle {
    _private: [u8; 0],
}

#[repr(C)]
struct GError {
    domain: u32,
    code: c_int,
    message: *mut c_char,
}

#[repr(C)]
struct RsvgRectangle {
    x: c_double,
    y: c_double,
    width: c_double,
    height: c_double,
}

#[link(name = "rsvg-2")]
unsafe extern "C" {
    fn rsvg_handle_new_from_data(
        data: *const u8,
        data_len: usize,
        error: *mut *mut GError,
    ) -> *mut RsvgHandle;
    fn rsvg_handle_render_document(
        handle: *mut RsvgHandle,
        cr: *mut cairo_t,
        viewport: *const RsvgRectangle,
        error: *mut *mut GError,
    ) -> c_int;
}

#[link(name = "gobject-2.0")]
unsafe extern "C" {
    fn g_object_unref(obj: *mut c_void);
}

#[link(name = "glib-2.0")]
unsafe extern "C" {
    fn g_error_free(error: *mut GError);
}

pub struct CairoCanvas {
    display: *mut Display,
    visual: *mut Visual,
    surface: *mut cairo_surface_t,
    cr: *mut cairo_t,
}

impl CairoCanvas {
    pub fn new(
        display: *mut Display,
        drawable: Drawable,
        visual: *mut Visual,
        width: i32,
        height: i32,
    ) -> Result<Self, String> {
        if width <= 0 || height <= 0 {
            return Err(format!("Invalid Cairo surface size: {width}x{height}"));
        }
        let surface = unsafe {
            cairo_xlib_surface_create(
                display,
                drawable,
                visual,
                width as c_int,
                height as c_int,
            )
        };
        if surface.is_null() {
            return Err("cairo_xlib_surface_create returned null".to_owned());
        }
        let status = unsafe { cairo_surface_status(surface) };
        if status != CAIRO_STATUS_SUCCESS {
            unsafe { cairo_surface_destroy(surface) };
            return Err(format!("cairo surface error: {}", cairo_status_message(status)));
        }

        let cr = unsafe { cairo_create(surface) };
        if cr.is_null() {
            unsafe { cairo_surface_destroy(surface) };
            return Err("cairo_create returned null".to_owned());
        }
        let status = unsafe { cairo_status(cr) };
        if status != CAIRO_STATUS_SUCCESS {
            unsafe { cairo_surface_destroy(surface) };
            return Err(format!("cairo context error: {}", cairo_status_message(status)));
        }

        Ok(Self {
            display,
            visual,
            surface,
            cr,
        })
    }

    pub fn recreate(&mut self, drawable: Drawable, width: i32, height: i32) -> Result<(), String> {
        self.destroy();
        let mut next = CairoCanvas::new(self.display, drawable, self.visual, width, height)?;
        self.surface = next.surface;
        self.cr = next.cr;
        next.surface = std::ptr::null_mut();
        next.cr = std::ptr::null_mut();
        Ok(())
    }

    pub fn destroy(&mut self) {
        if !self.cr.is_null() {
            let status = unsafe { cairo_status(self.cr) };
            if status == CAIRO_STATUS_SUCCESS {
                unsafe { cairo_destroy(self.cr) };
            }
            self.cr = std::ptr::null_mut();
        }
        if !self.surface.is_null() {
            unsafe { cairo_surface_destroy(self.surface) };
            self.surface = std::ptr::null_mut();
        }
    }

    pub fn fill_rect(&mut self, x_px: i32, y_px: i32, width_px: i32, height_px: i32, color: Color) {
        if self.cr.is_null() {
            return;
        }
        if width_px <= 0 || height_px <= 0 {
            return;
        }
        unsafe {
            cairo_set_source_rgba(
                self.cr,
                f64::from(color.r) / 255.0,
                f64::from(color.g) / 255.0,
                f64::from(color.b) / 255.0,
                f64::from(color.a) / 255.0,
            );
            cairo_rectangle(
                self.cr,
                f64::from(x_px),
                f64::from(y_px),
                f64::from(width_px),
                f64::from(height_px),
            );
            cairo_fill(self.cr);
            cairo_new_path(self.cr);
            cairo_surface_flush(self.surface);
        }
    }

    pub fn push_group(&mut self) {
        if self.cr.is_null() {
            return;
        }
        unsafe { cairo_push_group(self.cr) };
    }

    pub fn pop_group_with_alpha(&mut self, opacity: u8) {
        if self.cr.is_null() {
            return;
        }
        unsafe {
            cairo_pop_group_to_source(self.cr);
            if opacity >= 255 {
                cairo_paint(self.cr);
            } else {
                cairo_paint_with_alpha(self.cr, f64::from(opacity) / 255.0);
            }
            cairo_new_path(self.cr);
            cairo_surface_flush(self.surface);
        }
    }

    pub fn draw_text(&mut self, x_px: i32, y_px: i32, text: &str, style: TextStyle) -> Result<(), String> {
        if self.cr.is_null() {
            return Ok(());
        }
        if text.is_empty() {
            return Ok(());
        }

        let family = match style.font_family {
            crate::style::FontFamily::SansSerif => b"Verdana\0".as_ptr().cast::<c_char>(),
            crate::style::FontFamily::Monospace => b"monospace\0".as_ptr().cast::<c_char>(),
        };
        let weight = if style.bold {
            cairo_font_weight_t::CAIRO_FONT_WEIGHT_BOLD
        } else {
            cairo_font_weight_t::CAIRO_FONT_WEIGHT_NORMAL
        };

        let text = CString::new(text).map_err(|_| "text contains a NUL byte".to_owned())?;

        unsafe {
            cairo_save(self.cr);
            cairo_set_source_rgba(
                self.cr,
                f64::from(style.color.r) / 255.0,
                f64::from(style.color.g) / 255.0,
                f64::from(style.color.b) / 255.0,
                f64::from(style.color.a) / 255.0,
            );
            cairo_select_font_face(
                self.cr,
                family,
                cairo_font_slant_t::CAIRO_FONT_SLANT_NORMAL,
                weight,
            );
            cairo_set_font_size(self.cr, f64::from(style.font_size_px.max(1)));
            cairo_move_to(self.cr, f64::from(x_px), f64::from(y_px));
            cairo_show_text(self.cr, text.as_ptr());
            cairo_restore(self.cr);
            cairo_surface_flush(self.surface);
        }
        Ok(())
    }

    pub fn fill_rounded_rect(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        radius_px: i32,
        color: Color,
    ) {
        if self.cr.is_null() {
            return;
        }
        if width_px <= 0 || height_px <= 0 {
            return;
        }
        let radius_px = radius_px.max(0).min(width_px / 2).min(height_px / 2);
        unsafe {
            cairo_set_source_rgba(
                self.cr,
                f64::from(color.r) / 255.0,
                f64::from(color.g) / 255.0,
                f64::from(color.b) / 255.0,
                f64::from(color.a) / 255.0,
            );
            rounded_rect_path(
                self.cr,
                x_px,
                y_px,
                width_px,
                height_px,
                radius_px,
            );
            cairo_fill(self.cr);
            cairo_new_path(self.cr);
            cairo_surface_flush(self.surface);
        }
    }

    pub fn stroke_rounded_rect(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        radius_px: i32,
        stroke_px: i32,
        color: Color,
    ) {
        if self.cr.is_null() {
            return;
        }
        if width_px <= 0 || height_px <= 0 {
            return;
        }
        let stroke_px = stroke_px.max(0);
        if stroke_px == 0 {
            return;
        }
        let radius_px = radius_px.max(0).min(width_px / 2).min(height_px / 2);
        unsafe {
            cairo_set_source_rgba(
                self.cr,
                f64::from(color.r) / 255.0,
                f64::from(color.g) / 255.0,
                f64::from(color.b) / 255.0,
                f64::from(color.a) / 255.0,
            );
            cairo_set_line_width(self.cr, f64::from(stroke_px));
            rounded_rect_path(
                self.cr,
                x_px,
                y_px,
                width_px,
                height_px,
                radius_px,
            );
            cairo_stroke(self.cr);
            cairo_new_path(self.cr);
            cairo_surface_flush(self.surface);
        }
    }

    pub fn draw_image_surface(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        surface: *mut cairo_surface_t,
        surface_width_px: i32,
        surface_height_px: i32,
        opacity: u8,
    ) {
        if self.cr.is_null() || surface.is_null() {
            return;
        }
        if width_px <= 0 || height_px <= 0 {
            return;
        }
        if surface_width_px <= 0 || surface_height_px <= 0 {
            return;
        }
        if opacity == 0 {
            return;
        }

        unsafe {
            cairo_save(self.cr);
            cairo_translate(self.cr, f64::from(x_px), f64::from(y_px));
            cairo_scale(
                self.cr,
                f64::from(width_px) / f64::from(surface_width_px),
                f64::from(height_px) / f64::from(surface_height_px),
            );
            cairo_set_source_surface(self.cr, surface, 0.0, 0.0);
            if opacity == 255 {
                cairo_paint(self.cr);
            } else {
                cairo_paint_with_alpha(self.cr, f64::from(opacity) / 255.0);
            }
            cairo_restore(self.cr);
            cairo_surface_flush(self.surface);
        }
    }

    pub fn create_argb32_surface_for_data(
        &self,
        data: &mut [u8],
        width: i32,
        height: i32,
        stride: i32,
    ) -> Result<*mut cairo_surface_t, String> {
        if width <= 0 || height <= 0 {
            return Err("Invalid surface size".to_owned());
        }
        let surface = unsafe {
            cairo_image_surface_create_for_data(
                data.as_mut_ptr(),
                cairo_format_t::CAIRO_FORMAT_ARGB32,
                width as c_int,
                height as c_int,
                stride as c_int,
            )
        };
        if surface.is_null() {
            return Err("cairo_image_surface_create_for_data returned null".to_owned());
        }
        let status = unsafe { cairo_surface_status(surface) };
        if status != CAIRO_STATUS_SUCCESS {
            unsafe { cairo_surface_destroy(surface) };
            return Err(format!("cairo surface error: {}", cairo_status_message(status)));
        }
        Ok(surface)
    }

    pub fn destroy_surface(&self, surface: *mut cairo_surface_t) {
        if surface.is_null() {
            return;
        }
        unsafe { cairo_surface_destroy(surface) };
    }

    pub fn draw_svg(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        svg_xml: &str,
        opacity: u8,
    ) -> Result<(), String> {
        if self.cr.is_null() {
            return Ok(());
        }
        let status = unsafe { cairo_status(self.cr) };
        if status != CAIRO_STATUS_SUCCESS {
            return Ok(());
        }
        if width_px <= 0 || height_px <= 0 {
            return Ok(());
        }
        if opacity == 0 {
            return Ok(());
        }

        let svg_xml = svg_xml.trim();
        if svg_xml.is_empty() {
            return Ok(());
        }

        let mut parse_error: *mut GError = std::ptr::null_mut();
        let handle = unsafe {
            rsvg_handle_new_from_data(svg_xml.as_ptr(), svg_xml.len(), &mut parse_error)
        };

        if handle.is_null() {
            let message = gerror_message_and_free(parse_error);
            return Err(format!("Failed to parse SVG: {message}"));
        }

        struct Handle(*mut RsvgHandle);
        impl Drop for Handle {
            fn drop(&mut self) {
                unsafe { g_object_unref(self.0 as *mut c_void) };
            }
        }
        let handle = Handle(handle);

        let viewport = RsvgRectangle {
            x: 0.0,
            y: 0.0,
            width: f64::from(width_px),
            height: f64::from(height_px),
        };

        unsafe {
            cairo_save(self.cr);
            cairo_translate(self.cr, f64::from(x_px), f64::from(y_px));
            if opacity != 255 {
                cairo_push_group(self.cr);
            }
        }

        let status = unsafe { cairo_status(self.cr) };
        if status != CAIRO_STATUS_SUCCESS {
            unsafe {
                cairo_restore(self.cr);
                cairo_surface_flush(self.surface);
            }
            return Ok(());
        }

        let mut render_error: *mut GError = std::ptr::null_mut();
        let rendered =
            unsafe { rsvg_handle_render_document(handle.0, self.cr, &viewport, &mut render_error) };

        let result = if rendered == 0 {
            Err(format!(
                "Failed to render SVG: {}",
                gerror_message_and_free(render_error)
            ))
        } else {
            Ok(())
        };

        unsafe {
            if opacity != 255 {
                cairo_pop_group_to_source(self.cr);
                cairo_paint_with_alpha(self.cr, f64::from(opacity) / 255.0);
            }
            cairo_restore(self.cr);
            cairo_surface_flush(self.surface);
        }

        result
    }
}

impl Drop for CairoCanvas {
    fn drop(&mut self) {
        self.destroy();
    }
}

fn cairo_status_message(status: cairo_status_t) -> String {
    let ptr = unsafe { cairo_status_to_string(status) };
    if ptr.is_null() {
        return format!("cairo status {status}");
    }
    unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned()
}

fn gerror_message_and_free(error: *mut GError) -> String {
    if error.is_null() {
        return "unknown error".to_owned();
    }

    let message = unsafe {
        let ptr = (*error).message;
        if ptr.is_null() {
            "unknown error".to_owned()
        } else {
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    };

    unsafe { g_error_free(error) };
    message
}

fn rounded_rect_path(cr: *mut cairo_t, x_px: i32, y_px: i32, width_px: i32, height_px: i32, radius_px: i32) {
    let x = f64::from(x_px);
    let y = f64::from(y_px);
    let w = f64::from(width_px);
    let h = f64::from(height_px);
    let r = f64::from(radius_px);

    if radius_px <= 0 {
        unsafe { cairo_rectangle(cr, x, y, w, h) };
        return;
    }

    let right = x + w;
    let bottom = y + h;

    unsafe {
        cairo_move_to(cr, x + r, y);
        cairo_line_to(cr, right - r, y);
        cairo_arc(
            cr,
            right - r,
            y + r,
            r,
            -std::f64::consts::FRAC_PI_2,
            0.0,
        );
        cairo_line_to(cr, right, bottom - r);
        cairo_arc(
            cr,
            right - r,
            bottom - r,
            r,
            0.0,
            std::f64::consts::FRAC_PI_2,
        );
        cairo_line_to(cr, x + r, bottom);
        cairo_arc(
            cr,
            x + r,
            bottom - r,
            r,
            std::f64::consts::FRAC_PI_2,
            std::f64::consts::PI,
        );
        cairo_line_to(cr, x, y + r);
        cairo_arc(
            cr,
            x + r,
            y + r,
            r,
            std::f64::consts::PI,
            std::f64::consts::PI * 1.5,
        );
        cairo_close_path(cr);
    }
}
