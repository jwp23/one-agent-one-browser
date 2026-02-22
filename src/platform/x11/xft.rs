use crate::geom::Color;
use crate::render::{FontMetricsPx, TextStyle};
use crate::style::FontFamily;
use core::ffi::{c_char, c_int, c_short, c_uchar, c_ulong, c_ushort, c_void};
use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::CString;

use super::xlib::{Bool, Colormap, Display, Drawable, Visual};

pub type XftDraw = c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct XRenderColor {
    red: c_ushort,
    green: c_ushort,
    blue: c_ushort,
    alpha: c_ushort,
}

#[repr(C)]
struct XGlyphInfo {
    _width: c_ushort,
    _height: c_ushort,
    _x: c_short,
    _y: c_short,
    x_off: c_short,
    _y_off: c_short,
}

#[repr(C)]
struct XftColor {
    pixel: c_ulong,
    color: XRenderColor,
}

#[repr(C)]
struct XftFont {
    ascent: c_int,
    descent: c_int,
    _height: c_int,
    _max_advance_width: c_int,
    _charset: *mut c_void,
    _pattern: *mut c_void,
}

#[link(name = "Xft")]
unsafe extern "C" {
    fn XftDrawCreate(
        dpy: *mut Display,
        drawable: Drawable,
        visual: *mut Visual,
        colormap: Colormap,
    ) -> *mut XftDraw;
    fn XftDrawDestroy(draw: *mut XftDraw);

    fn XftFontOpenName(dpy: *mut Display, screen: c_int, name: *const c_char) -> *mut XftFont;
    fn XftFontClose(dpy: *mut Display, font: *mut XftFont);

    fn XftTextExtentsUtf8(
        dpy: *mut Display,
        font: *mut XftFont,
        string: *const c_uchar,
        len: c_int,
        extents: *mut XGlyphInfo,
    );
    fn XftDrawStringUtf8(
        draw: *mut XftDraw,
        color: *const XftColor,
        font: *mut XftFont,
        x: c_int,
        y: c_int,
        string: *const c_uchar,
        len: c_int,
    );

    fn XftColorAllocValue(
        dpy: *mut Display,
        visual: *mut Visual,
        colormap: Colormap,
        color: *const XRenderColor,
        result: *mut XftColor,
    ) -> Bool;
    fn XftColorFree(
        dpy: *mut Display,
        visual: *mut Visual,
        colormap: Colormap,
        color: *mut XftColor,
    );
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct FontKey {
    family: FontFamily,
    size_px: i32,
    bold: bool,
}

pub struct XftRenderer {
    display: *mut Display,
    visual: *mut Visual,
    colormap: Colormap,
    screen: c_int,
    draw: *mut XftDraw,
    fallback_font: *mut XftFont,
    font_cache: RefCell<HashMap<FontKey, *mut XftFont>>,
    color_cache: HashMap<u32, XftColor>,
}

impl XftRenderer {
    pub fn new(
        display: *mut Display,
        visual: *mut Visual,
        colormap: Colormap,
        screen: c_int,
        drawable: Drawable,
    ) -> Result<Self, String> {
        let draw = unsafe { XftDrawCreate(display, drawable, visual, colormap) };
        if draw.is_null() {
            return Err("XftDrawCreate failed".to_owned());
        }

        let fallback_key = FontKey {
            family: FontFamily::SansSerif,
            size_px: 13,
            bold: false,
        };
        let fallback_font = open_xft_font(display, screen, fallback_key)?;
        let mut font_cache = HashMap::new();
        font_cache.insert(fallback_key, fallback_font);

        Ok(Self {
            display,
            visual,
            colormap,
            screen,
            draw,
            fallback_font,
            font_cache: RefCell::new(font_cache),
            color_cache: HashMap::new(),
        })
    }

    pub fn recreate_draw(&mut self, drawable: Drawable) -> Result<(), String> {
        unsafe {
            XftDrawDestroy(self.draw);
        }
        self.draw = unsafe { XftDrawCreate(self.display, drawable, self.visual, self.colormap) };
        if self.draw.is_null() {
            return Err("XftDrawCreate failed".to_owned());
        }
        Ok(())
    }

    pub fn destroy(&mut self) {
        if self.draw.is_null() {
            return;
        }

        unsafe {
            XftDrawDestroy(self.draw);
        }
        self.draw = std::ptr::null_mut();

        for (_, mut color) in self.color_cache.drain() {
            unsafe {
                XftColorFree(self.display, self.visual, self.colormap, &mut color);
            }
        }

        for (_, font) in self.font_cache.borrow_mut().drain() {
            unsafe {
                XftFontClose(self.display, font);
            }
        }
    }

    pub fn font_metrics_px(&self, style: TextStyle) -> FontMetricsPx {
        let font = self.font_for(style);
        unsafe {
            FontMetricsPx {
                ascent_px: (*font).ascent.max(1),
                descent_px: (*font).descent.max(0),
            }
        }
    }

    pub fn text_width_px(&self, text: &str, style: TextStyle) -> Result<i32, String> {
        if style.letter_spacing_px == 0 {
            return self.text_width_px_no_spacing(text, style);
        }

        let mut total_width: i64 = 0;
        let mut first = true;
        for ch in text.chars() {
            if !first {
                total_width += i64::from(style.letter_spacing_px);
            }
            first = false;

            let mut buf = [0u8; 4];
            let ch = ch.encode_utf8(&mut buf);
            total_width += i64::from(self.text_width_px_no_spacing(ch, style)?);
        }

        Ok(total_width.clamp(0, i64::from(i32::MAX)) as i32)
    }

    pub fn draw_text(
        &mut self,
        x_px: i32,
        y_px: i32,
        text: &str,
        style: TextStyle,
    ) -> Result<(), String> {
        if text.is_empty() {
            return Ok(());
        }
        let font = self.font_for(style);
        let color = self.ensure_color(style.color)?;
        if style.letter_spacing_px == 0 {
            let len: c_int = text
                .len()
                .try_into()
                .map_err(|_| "text length out of range for Xft".to_owned())?;
            unsafe {
                XftDrawStringUtf8(
                    self.draw,
                    color,
                    font,
                    x_px,
                    y_px,
                    text.as_ptr().cast::<c_uchar>(),
                    len,
                );
            }
            return Ok(());
        }

        let mut cursor_x = x_px;
        let mut first = true;
        for ch in text.chars() {
            if !first {
                cursor_x = cursor_x.saturating_add(style.letter_spacing_px);
            }
            first = false;

            let mut buf = [0u8; 4];
            let ch = ch.encode_utf8(&mut buf);
            let len: c_int = ch
                .len()
                .try_into()
                .map_err(|_| "text length out of range for Xft".to_owned())?;
            unsafe {
                XftDrawStringUtf8(
                    self.draw,
                    color,
                    font,
                    cursor_x,
                    y_px,
                    ch.as_ptr().cast::<c_uchar>(),
                    len,
                );
            }
            cursor_x = cursor_x.saturating_add(self.text_width_px_no_spacing(ch, style)?);
        }
        Ok(())
    }

    fn ensure_color(&mut self, color: Color) -> Result<*const XftColor, String> {
        let key = (u32::from(color.r) << 24)
            | (u32::from(color.g) << 16)
            | (u32::from(color.b) << 8)
            | u32::from(color.a);
        if !self.color_cache.contains_key(&key) {
            let render = XRenderColor {
                red: (c_ushort::from(color.r) << 8) | c_ushort::from(color.r),
                green: (c_ushort::from(color.g) << 8) | c_ushort::from(color.g),
                blue: (c_ushort::from(color.b) << 8) | c_ushort::from(color.b),
                alpha: (c_ushort::from(color.a) << 8) | c_ushort::from(color.a),
            };
            let mut xft_color = XftColor {
                pixel: 0,
                color: render,
            };
            let ok = unsafe {
                XftColorAllocValue(
                    self.display,
                    self.visual,
                    self.colormap,
                    &render,
                    &mut xft_color,
                )
            };
            if ok == 0 {
                return Err("XftColorAllocValue failed".to_owned());
            }
            self.color_cache.insert(key, xft_color);
        }

        Ok(self.color_cache.get(&key).expect("color was just inserted") as *const XftColor)
    }

    fn font_for(&self, style: TextStyle) -> *mut XftFont {
        let key = FontKey {
            family: style.font_family,
            size_px: style.font_size_px.max(1),
            bold: style.bold,
        };

        if let Some(&font) = self.font_cache.borrow().get(&key) {
            return font;
        }

        match open_xft_font(self.display, self.screen, key) {
            Ok(font) => {
                self.font_cache.borrow_mut().insert(key, font);
                font
            }
            Err(_) => self.fallback_font,
        }
    }

    fn text_width_px_no_spacing(&self, text: &str, style: TextStyle) -> Result<i32, String> {
        if text.is_empty() {
            return Ok(0);
        }
        let len: c_int = text
            .len()
            .try_into()
            .map_err(|_| "text length out of range for Xft".to_owned())?;
        let font = self.font_for(style);
        let mut extents = XGlyphInfo {
            _width: 0,
            _height: 0,
            _x: 0,
            _y: 0,
            x_off: 0,
            _y_off: 0,
        };
        unsafe {
            XftTextExtentsUtf8(
                self.display,
                font,
                text.as_ptr().cast::<c_uchar>(),
                len,
                &mut extents,
            );
        }
        Ok((extents.x_off as i32).max(0))
    }
}

fn open_xft_font(
    display: *mut Display,
    screen: c_int,
    key: FontKey,
) -> Result<*mut XftFont, String> {
    let family = match key.family {
        FontFamily::SansSerif => "Verdana",
        FontFamily::Serif => "serif",
        FontFamily::Monospace => "monospace",
    };
    let weight = if key.bold { "bold" } else { "regular" };
    let size_px = key.size_px.max(1);
    let pattern = format!("{family}:pixelsize={size_px}:weight={weight}");
    let pattern =
        CString::new(pattern).map_err(|_| "Font pattern contains a NUL byte".to_owned())?;
    let font = unsafe { XftFontOpenName(display, screen, pattern.as_ptr()) };
    if font.is_null() {
        return Err("XftFontOpenName failed".to_owned());
    }
    Ok(font)
}
