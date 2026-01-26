use crate::geom::Color;
use crate::image::{Argb32Image, RgbImage};
use crate::render::{FontMetricsPx, Painter, TextMeasurer, TextStyle, Viewport};
use crate::style::FontFamily;
use core::ffi::{c_double, c_int, c_uint, c_void};
use std::cell::RefCell;
use std::collections::HashMap;

type CGFloat = c_double;
type SizeT = usize;

type CFIndex = isize;
type CFAllocatorRef = *const c_void;
type CFAttributedStringRef = *const c_void;
type CFBooleanRef = *const c_void;
type CFDictionaryRef = *const c_void;
type CFStringRef = *const c_void;
type CFTypeRef = *const c_void;

type CTFontRef = *const c_void;
type CTLineRef = *const c_void;

type CGColorSpaceRef = *mut c_void;
type CGContextRef = *mut c_void;
type CGImageRef = *mut c_void;
type CGPathRef = *mut c_void;

#[repr(C)]
struct CGPoint {
    x: CGFloat,
    y: CGFloat,
}

#[repr(C)]
struct CGSize {
    width: CGFloat,
    height: CGFloat,
}

#[repr(C)]
struct CGRect {
    origin: CGPoint,
    size: CGSize,
}

#[repr(C)]
struct CGAffineTransform {
    a: CGFloat,
    b: CGFloat,
    c: CGFloat,
    d: CGFloat,
    tx: CGFloat,
    ty: CGFloat,
}

const IDENTITY_TRANSFORM: CGAffineTransform = CGAffineTransform {
    a: 1.0,
    b: 0.0,
    c: 0.0,
    d: 1.0,
    tx: 0.0,
    ty: 0.0,
};

const K_CGIMAGE_ALPHA_PREMULTIPLIED_FIRST: c_uint = 2;
const K_CGBITMAP_BYTEORDER32LITTLE: c_uint = 2 << 12;
const BITMAP_INFO_BGRA_PREMULTIPLIED: c_uint =
    K_CGIMAGE_ALPHA_PREMULTIPLIED_FIRST | K_CGBITMAP_BYTEORDER32LITTLE;

const BLEND_MODE_NORMAL: c_int = 0;

const LINE_CAP_BUTT: c_int = 0;
const LINE_JOIN_MITER: c_int = 0;

type CTFontSymbolicTraits = u32;
const K_CTFONT_BOLD_TRAIT: CTFontSymbolicTraits = 1 << 1;

#[allow(non_upper_case_globals)]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFTypeDictionaryKeyCallBacks: c_void;
    static kCFTypeDictionaryValueCallBacks: c_void;
    static kCFBooleanTrue: CFBooleanRef;

    fn CFRelease(cf: CFTypeRef);

    fn CFStringCreateWithBytes(
        alloc: CFAllocatorRef,
        bytes: *const u8,
        num_bytes: CFIndex,
        encoding: u32,
        is_external_representation: u8,
    ) -> CFStringRef;

    fn CFDictionaryCreate(
        allocator: CFAllocatorRef,
        keys: *const *const c_void,
        values: *const *const c_void,
        num_values: CFIndex,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> CFDictionaryRef;

    fn CFAttributedStringCreate(
        alloc: CFAllocatorRef,
        string: CFStringRef,
        attributes: CFDictionaryRef,
    ) -> CFAttributedStringRef;
}

#[allow(non_upper_case_globals)]
#[link(name = "CoreText", kind = "framework")]
unsafe extern "C" {
    static kCTFontAttributeName: CFStringRef;
    static kCTForegroundColorFromContextAttributeName: CFStringRef;

    fn CTFontCreateWithName(name: CFStringRef, size: CGFloat, matrix: *const c_void) -> CTFontRef;
    fn CTFontCreateCopyWithSymbolicTraits(
        font: CTFontRef,
        size: CGFloat,
        matrix: *const c_void,
        sym_trait_value: CTFontSymbolicTraits,
        sym_trait_mask: CTFontSymbolicTraits,
    ) -> CTFontRef;
    fn CTFontGetAscent(font: CTFontRef) -> CGFloat;
    fn CTFontGetDescent(font: CTFontRef) -> CGFloat;

    fn CTLineCreateWithAttributedString(string: CFAttributedStringRef) -> CTLineRef;
    fn CTLineGetTypographicBounds(
        line: CTLineRef,
        ascent: *mut CGFloat,
        descent: *mut CGFloat,
        leading: *mut CGFloat,
    ) -> CGFloat;
    fn CTLineDraw(line: CTLineRef, context: CGContextRef);
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGColorSpaceCreateDeviceRGB() -> CGColorSpaceRef;
    fn CGColorSpaceRelease(space: CGColorSpaceRef);

    fn CGBitmapContextCreate(
        data: *mut c_void,
        width: SizeT,
        height: SizeT,
        bits_per_component: SizeT,
        bytes_per_row: SizeT,
        space: CGColorSpaceRef,
        bitmap_info: c_uint,
    ) -> CGContextRef;
    fn CGBitmapContextCreateImage(context: CGContextRef) -> CGImageRef;

    fn CGContextRelease(c: CGContextRef);
    fn CGImageRelease(image: CGImageRef);

    fn CGContextSetRGBFillColor(c: CGContextRef, r: CGFloat, g: CGFloat, b: CGFloat, a: CGFloat);
    fn CGContextFillRect(c: CGContextRef, rect: CGRect);

    fn CGContextSetRGBStrokeColor(c: CGContextRef, r: CGFloat, g: CGFloat, b: CGFloat, a: CGFloat);
    fn CGContextSetLineWidth(c: CGContextRef, width: CGFloat);
    fn CGContextSetLineCap(c: CGContextRef, cap: c_int);
    fn CGContextSetLineJoin(c: CGContextRef, join: c_int);
    fn CGContextAddPath(c: CGContextRef, path: CGPathRef);
    fn CGContextDrawPath(c: CGContextRef, mode: c_int);

    fn CGPathCreateWithRoundedRect(rect: CGRect, corner_width: CGFloat, corner_height: CGFloat, transform: *const CGAffineTransform) -> CGPathRef;
    fn CGPathRelease(path: CGPathRef);

    fn CGContextSetAlpha(c: CGContextRef, alpha: CGFloat);
    fn CGContextBeginTransparencyLayer(c: CGContextRef, auxiliary_info: *const c_void);
    fn CGContextEndTransparencyLayer(c: CGContextRef);

    fn CGContextSetBlendMode(c: CGContextRef, mode: c_int);
    fn CGContextSaveGState(c: CGContextRef);
    fn CGContextRestoreGState(c: CGContextRef);

    fn CGContextSetTextMatrix(c: CGContextRef, t: CGAffineTransform);
    fn CGContextSetTextPosition(c: CGContextRef, x: CGFloat, y: CGFloat);
    fn CGContextDrawImage(c: CGContextRef, rect: CGRect, image: CGImageRef);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct FontKey {
    family: FontFamily,
    size_px: i32,
    bold: bool,
}

pub struct MacPainter {
    ctx: CGContextRef,
    width_px: i32,
    height_px: i32,
    data: Vec<u8>,
    opacity_depth: usize,
    font_cache: RefCell<HashMap<FontKey, CTFontRef>>,
}

impl MacPainter {
    pub fn new(viewport: Viewport) -> Result<Self, String> {
        let width_px = viewport.width_px;
        let height_px = viewport.height_px;
        if width_px <= 0 || height_px <= 0 {
            return Err(format!("Invalid viewport size: {width_px}x{height_px}"));
        }

        let (ctx, data) = create_bitmap_context(width_px, height_px)?;
        Ok(Self {
            ctx,
            width_px,
            height_px,
            data,
            opacity_depth: 0,
            font_cache: RefCell::new(HashMap::new()),
        })
    }

    pub fn ensure_back_buffer(&mut self, viewport: Viewport) -> Result<(), String> {
        let width_px = viewport.width_px;
        let height_px = viewport.height_px;
        if width_px <= 0 || height_px <= 0 {
            return Err(format!("Invalid viewport size: {width_px}x{height_px}"));
        }
        if width_px == self.width_px && height_px == self.height_px {
            return Ok(());
        }

        unsafe {
            CGContextRelease(self.ctx);
        }

        let (ctx, data) = create_bitmap_context(width_px, height_px)?;
        self.ctx = ctx;
        self.width_px = width_px;
        self.height_px = height_px;
        self.data = data;
        self.opacity_depth = 0;
        Ok(())
    }

    pub fn capture_back_buffer_rgb(&self) -> Result<RgbImage, String> {
        let width_u32: u32 = self
            .width_px
            .try_into()
            .map_err(|_| "Screenshot width out of range".to_owned())?;
        let height_u32: u32 = self
            .height_px
            .try_into()
            .map_err(|_| "Screenshot height out of range".to_owned())?;

        let expected_len = (width_u32 as usize)
            .checked_mul(height_u32 as usize)
            .and_then(|pixels| pixels.checked_mul(3))
            .ok_or_else(|| "Screenshot buffer size overflow".to_owned())?;
        let mut rgb = Vec::with_capacity(expected_len);

        for chunk in self.data.chunks_exact(4) {
            rgb.push(chunk[2]);
            rgb.push(chunk[1]);
            rgb.push(chunk[0]);
        }

        RgbImage::new(width_u32, height_u32, rgb)
    }

    pub fn create_cgimage(&self) -> Result<CGImageRef, String> {
        let image = unsafe { CGBitmapContextCreateImage(self.ctx) };
        if image.is_null() {
            return Err("CGBitmapContextCreateImage failed".to_owned());
        }
        Ok(image)
    }

    fn rect_to_quartz(&self, x_px: i32, y_px: i32, width_px: i32, height_px: i32) -> CGRect {
        let x = x_px as CGFloat;
        let width = width_px.max(0) as CGFloat;
        let height = height_px.max(0) as CGFloat;
        let y = (self.height_px.saturating_sub(y_px).saturating_sub(height_px)) as CGFloat;
        CGRect {
            origin: CGPoint { x, y },
            size: CGSize { width, height },
        }
    }

    fn baseline_y_to_quartz(&self, y_px: i32) -> CGFloat {
        self.height_px.saturating_sub(y_px) as CGFloat
    }

    fn font_for(&self, style: TextStyle) -> CTFontRef {
        let key = FontKey {
            family: style.font_family,
            size_px: style.font_size_px.max(1),
            bold: style.bold,
        };
        if let Some(existing) = self.font_cache.borrow().get(&key) {
            return *existing;
        }

        let base_name = match key.family {
            FontFamily::SansSerif => "Helvetica",
            FontFamily::Serif => "Times",
            FontFamily::Monospace => "Menlo",
        };
        let name =
            cf_string(base_name).unwrap_or_else(|| cf_string("Helvetica").expect("fallback font"));
        let size = key.size_px as CGFloat;
        let base_font = unsafe { CTFontCreateWithName(name, size, std::ptr::null()) };
        unsafe { CFRelease(name as CFTypeRef) };

        let font = if key.bold && !base_font.is_null() {
            let bold_font = unsafe {
                CTFontCreateCopyWithSymbolicTraits(
                    base_font,
                    size,
                    std::ptr::null(),
                    K_CTFONT_BOLD_TRAIT,
                    K_CTFONT_BOLD_TRAIT,
                )
            };
            if bold_font.is_null() {
                base_font
            } else {
                unsafe { CFRelease(base_font as CFTypeRef) };
                bold_font
            }
        } else {
            base_font
        };

        if font.is_null() {
            return std::ptr::null();
        }

        self.font_cache.borrow_mut().insert(key, font);
        font
    }

    fn draw_text_run(&self, x_px: i32, y_baseline_px: i32, text: &str, style: TextStyle) -> Result<(), String> {
        let font = self.font_for(style);
        if font.is_null() {
            return Ok(());
        }

        let cf_text = cf_string(text).ok_or_else(|| "Text contains invalid UTF-8".to_owned())?;

        let keys: [*const c_void; 2] = [
            unsafe { kCTFontAttributeName as *const c_void },
            unsafe { kCTForegroundColorFromContextAttributeName as *const c_void },
        ];
        let values: [*const c_void; 2] = [font as *const c_void, unsafe { kCFBooleanTrue as *const c_void }];
        let attrs = unsafe {
            CFDictionaryCreate(
                std::ptr::null(),
                keys.as_ptr(),
                values.as_ptr(),
                keys.len() as CFIndex,
                &raw const kCFTypeDictionaryKeyCallBacks,
                &raw const kCFTypeDictionaryValueCallBacks,
            )
        };
        if attrs.is_null() {
            unsafe { CFRelease(cf_text as CFTypeRef) };
            return Err("CFDictionaryCreate failed".to_owned());
        }

        let attr_str = unsafe { CFAttributedStringCreate(std::ptr::null(), cf_text, attrs) };
        unsafe {
            CFRelease(attrs as CFTypeRef);
            CFRelease(cf_text as CFTypeRef);
        }
        if attr_str.is_null() {
            return Err("CFAttributedStringCreate failed".to_owned());
        }

        let line = unsafe { CTLineCreateWithAttributedString(attr_str) };
        unsafe { CFRelease(attr_str as CFTypeRef) };
        if line.is_null() {
            return Err("CTLineCreateWithAttributedString failed".to_owned());
        }

        unsafe {
            CGContextSetTextMatrix(self.ctx, IDENTITY_TRANSFORM);
            CGContextSetTextPosition(
                self.ctx,
                x_px as CGFloat,
                self.baseline_y_to_quartz(y_baseline_px),
            );
            CTLineDraw(line, self.ctx);
            CFRelease(line as CFTypeRef);
        }
        Ok(())
    }

    fn text_width_no_spacing(&self, text: &str, style: TextStyle) -> Result<i32, String> {
        let font = self.font_for(style);
        if font.is_null() {
            return Ok(0);
        }

        let cf_text = cf_string(text).ok_or_else(|| "Text contains invalid UTF-8".to_owned())?;
        let keys: [*const c_void; 1] = [unsafe { kCTFontAttributeName as *const c_void }];
        let values: [*const c_void; 1] = [font as *const c_void];
        let attrs = unsafe {
            CFDictionaryCreate(
                std::ptr::null(),
                keys.as_ptr(),
                values.as_ptr(),
                1,
                &raw const kCFTypeDictionaryKeyCallBacks,
                &raw const kCFTypeDictionaryValueCallBacks,
            )
        };
        if attrs.is_null() {
            unsafe { CFRelease(cf_text as CFTypeRef) };
            return Err("CFDictionaryCreate failed".to_owned());
        }

        let attr_str = unsafe { CFAttributedStringCreate(std::ptr::null(), cf_text, attrs) };
        unsafe {
            CFRelease(attrs as CFTypeRef);
            CFRelease(cf_text as CFTypeRef);
        }
        if attr_str.is_null() {
            return Err("CFAttributedStringCreate failed".to_owned());
        }

        let line = unsafe { CTLineCreateWithAttributedString(attr_str) };
        unsafe { CFRelease(attr_str as CFTypeRef) };
        if line.is_null() {
            return Err("CTLineCreateWithAttributedString failed".to_owned());
        }

        let mut ascent: CGFloat = 0.0;
        let mut descent: CGFloat = 0.0;
        let mut leading: CGFloat = 0.0;
        let width = unsafe { CTLineGetTypographicBounds(line, &mut ascent, &mut descent, &mut leading) };
        unsafe { CFRelease(line as CFTypeRef) };

        if !width.is_finite() || width <= 0.0 {
            return Ok(0);
        }
        let w = width.round() as i64;
        Ok(w.clamp(0, i64::from(i32::MAX)) as i32)
    }
}

impl Drop for MacPainter {
    fn drop(&mut self) {
        unsafe {
            if !self.ctx.is_null() {
                CGContextRelease(self.ctx);
            }
        }
        for (_, font) in self.font_cache.borrow_mut().drain() {
            unsafe {
                if !font.is_null() {
                    CFRelease(font as CFTypeRef);
                }
            }
        }
    }
}

fn create_bitmap_context(width_px: i32, height_px: i32) -> Result<(CGContextRef, Vec<u8>), String> {
    let width: usize = width_px
        .try_into()
        .map_err(|_| "Viewport width out of range".to_owned())?;
    let height: usize = height_px
        .try_into()
        .map_err(|_| "Viewport height out of range".to_owned())?;

    let bytes_per_row = width
        .checked_mul(4)
        .ok_or_else(|| "Viewport row stride overflow".to_owned())?;
    let len = bytes_per_row
        .checked_mul(height)
        .ok_or_else(|| "Viewport buffer size overflow".to_owned())?;
    let mut data = vec![0u8; len];

    let color_space = unsafe { CGColorSpaceCreateDeviceRGB() };
    if color_space.is_null() {
        return Err("CGColorSpaceCreateDeviceRGB failed".to_owned());
    }

    let ctx = unsafe {
        CGBitmapContextCreate(
            data.as_mut_ptr().cast::<c_void>(),
            width,
            height,
            8,
            bytes_per_row,
            color_space,
            BITMAP_INFO_BGRA_PREMULTIPLIED,
        )
    };
    unsafe { CGColorSpaceRelease(color_space) };
    if ctx.is_null() {
        return Err("CGBitmapContextCreate failed".to_owned());
    }
    unsafe {
        CGContextSetBlendMode(ctx, BLEND_MODE_NORMAL);
    }
    Ok((ctx, data))
}

fn cf_string(input: &str) -> Option<CFStringRef> {
    const K_CFSTRING_ENCODING_UTF8: u32 = 0x0800_0100;

    let bytes = input.as_bytes();
    let len: CFIndex = bytes.len().try_into().ok()?;
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
        None
    } else {
        Some(cf)
    }
}

impl TextMeasurer for MacPainter {
    fn font_metrics_px(&self, style: TextStyle) -> FontMetricsPx {
        let font = self.font_for(style);
        if font.is_null() {
            return FontMetricsPx {
                ascent_px: 8,
                descent_px: 2,
            };
        }
        let ascent = unsafe { CTFontGetAscent(font) };
        let descent = unsafe { CTFontGetDescent(font) };
        let ascent_px = ascent.ceil().clamp(1.0, 1_000_000.0) as i32;
        let descent_px = descent.ceil().clamp(0.0, 1_000_000.0) as i32;
        FontMetricsPx {
            ascent_px,
            descent_px,
        }
    }

    fn text_width_px(&self, text: &str, style: TextStyle) -> Result<i32, String> {
        if style.letter_spacing_px == 0 {
            return self.text_width_no_spacing(text, style);
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
            total_width += i64::from(self.text_width_no_spacing(ch, style)?);
        }
        Ok(total_width.clamp(0, i64::from(i32::MAX)) as i32)
    }
}

impl Painter for MacPainter {
    fn clear(&mut self) -> Result<(), String> {
        self.fill_rect(0, 0, self.width_px, self.height_px, Color::WHITE)
    }

    fn push_opacity(&mut self, opacity: u8) -> Result<(), String> {
        if opacity >= 255 {
            return Ok(());
        }
        self.opacity_depth = self.opacity_depth.saturating_add(1);
        unsafe {
            CGContextBeginTransparencyLayer(self.ctx, std::ptr::null());
        }
        Ok(())
    }

    fn pop_opacity(&mut self, opacity: u8) -> Result<(), String> {
        if opacity >= 255 {
            return Ok(());
        }
        if self.opacity_depth == 0 {
            return Err("opacity stack underflow".to_owned());
        }
        self.opacity_depth -= 1;
        unsafe {
            CGContextSetAlpha(self.ctx, (opacity as CGFloat) / 255.0);
            CGContextEndTransparencyLayer(self.ctx);
        }
        Ok(())
    }

    fn fill_rect(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        color: Color,
    ) -> Result<(), String> {
        if width_px <= 0 || height_px <= 0 {
            return Ok(());
        }
        let rect = self.rect_to_quartz(x_px, y_px, width_px, height_px);
        unsafe {
            CGContextSetRGBFillColor(
                self.ctx,
                (color.r as CGFloat) / 255.0,
                (color.g as CGFloat) / 255.0,
                (color.b as CGFloat) / 255.0,
                (color.a as CGFloat) / 255.0,
            );
            CGContextFillRect(self.ctx, rect);
        }
        Ok(())
    }

    fn fill_rounded_rect(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        radius_px: i32,
        color: Color,
    ) -> Result<(), String> {
        if width_px <= 0 || height_px <= 0 {
            return Ok(());
        }

        let rect = self.rect_to_quartz(x_px, y_px, width_px, height_px);
        let radius = radius_px.max(0) as CGFloat;
        unsafe {
            CGContextSetRGBFillColor(
                self.ctx,
                (color.r as CGFloat) / 255.0,
                (color.g as CGFloat) / 255.0,
                (color.b as CGFloat) / 255.0,
                (color.a as CGFloat) / 255.0,
            );
            let path = CGPathCreateWithRoundedRect(rect, radius, radius, &IDENTITY_TRANSFORM);
            if !path.is_null() {
                CGContextAddPath(self.ctx, path);
                CGContextDrawPath(self.ctx, 0);
                CGPathRelease(path);
            }
        }
        Ok(())
    }

    fn stroke_rounded_rect(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        radius_px: i32,
        border_width_px: i32,
        color: Color,
    ) -> Result<(), String> {
        if width_px <= 0 || height_px <= 0 {
            return Ok(());
        }
        if border_width_px <= 0 {
            return Ok(());
        }

        let rect = self.rect_to_quartz(x_px, y_px, width_px, height_px);
        let radius = radius_px.max(0) as CGFloat;
        unsafe {
            CGContextSetRGBStrokeColor(
                self.ctx,
                (color.r as CGFloat) / 255.0,
                (color.g as CGFloat) / 255.0,
                (color.b as CGFloat) / 255.0,
                (color.a as CGFloat) / 255.0,
            );
            CGContextSetLineWidth(self.ctx, border_width_px.max(1) as CGFloat);
            CGContextSetLineCap(self.ctx, LINE_CAP_BUTT);
            CGContextSetLineJoin(self.ctx, LINE_JOIN_MITER);
            let path = CGPathCreateWithRoundedRect(rect, radius, radius, &IDENTITY_TRANSFORM);
            if !path.is_null() {
                CGContextAddPath(self.ctx, path);
                CGContextDrawPath(self.ctx, 1);
                CGPathRelease(path);
            }
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
        if text.is_empty() {
            return Ok(());
        }

        unsafe {
            CGContextSetRGBFillColor(
                self.ctx,
                (style.color.r as CGFloat) / 255.0,
                (style.color.g as CGFloat) / 255.0,
                (style.color.b as CGFloat) / 255.0,
                (style.color.a as CGFloat) / 255.0,
            );
        }

        if style.letter_spacing_px == 0 {
            self.draw_text_run(x_px, y_px, text, style)?;
        } else {
            let mut cursor_x = x_px;
            let mut first = true;
            for ch in text.chars() {
                if !first {
                    cursor_x = cursor_x.saturating_add(style.letter_spacing_px);
                }
                first = false;

                let mut buf = [0u8; 4];
                let ch = ch.encode_utf8(&mut buf);
                self.draw_text_run(cursor_x, y_px, ch, style)?;
                cursor_x = cursor_x.saturating_add(self.text_width_no_spacing(ch, style)?);
            }
        }

        if style.underline {
            let width_px = self.text_width_px(text, style)?;
            self.fill_rect(
                x_px,
                y_px.saturating_add(1),
                width_px,
                1,
                style.color,
            )?;
        }

        Ok(())
    }

    fn draw_image(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        image: &Argb32Image,
        opacity: u8,
    ) -> Result<(), String> {
        if width_px <= 0 || height_px <= 0 || opacity == 0 {
            return Ok(());
        }
        if image.width == 0 || image.height == 0 {
            return Ok(());
        }

        let mut data = image.data.clone();
        let color_space = unsafe { CGColorSpaceCreateDeviceRGB() };
        if color_space.is_null() {
            return Err("CGColorSpaceCreateDeviceRGB failed".to_owned());
        }
        let ctx = unsafe {
            CGBitmapContextCreate(
                data.as_mut_ptr().cast::<c_void>(),
                image.width as usize,
                image.height as usize,
                8,
                image.row_stride_bytes(),
                color_space,
                BITMAP_INFO_BGRA_PREMULTIPLIED,
            )
        };
        unsafe { CGColorSpaceRelease(color_space) };
        if ctx.is_null() {
            return Err("CGBitmapContextCreate failed for image".to_owned());
        }

        let cg_image = unsafe { CGBitmapContextCreateImage(ctx) };
        unsafe { CGContextRelease(ctx) };
        if cg_image.is_null() {
            return Err("CGBitmapContextCreateImage failed for image".to_owned());
        }

        let rect = self.rect_to_quartz(x_px, y_px, width_px, height_px);

        unsafe {
            if opacity == 255 {
                CGContextDrawImage(self.ctx, rect, cg_image);
            } else {
                CGContextSaveGState(self.ctx);
                CGContextSetAlpha(self.ctx, (opacity as CGFloat) / 255.0);
                CGContextDrawImage(self.ctx, rect, cg_image);
                CGContextRestoreGState(self.ctx);
            }
            CGImageRelease(cg_image);
        }
        Ok(())
    }

    fn draw_svg(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        svg_xml: &str,
        opacity: u8,
    ) -> Result<(), String> {
        if width_px <= 0 || height_px <= 0 || opacity == 0 {
            return Ok(());
        }

        let svg_xml = svg_xml.trim();
        if svg_xml.is_empty() {
            return Ok(());
        }

        let cg_image = match super::svg::rasterize_svg_to_cgimage(svg_xml, width_px, height_px) {
            Ok(image) => image,
            Err(message) => {
                crate::debug::log(
                    crate::debug::Target::Render,
                    crate::debug::Level::Warn,
                    format_args!(
                        "SVG render failed: {}",
                        crate::debug::shorten(&message, 120)
                    ),
                );
                return Ok(());
            }
        };
        let rect = self.rect_to_quartz(x_px, y_px, width_px, height_px);
        unsafe {
            if opacity == 255 {
                CGContextDrawImage(self.ctx, rect, cg_image);
            } else {
                CGContextSaveGState(self.ctx);
                CGContextSetAlpha(self.ctx, (opacity as CGFloat) / 255.0);
                CGContextDrawImage(self.ctx, rect, cg_image);
                CGContextRestoreGState(self.ctx);
            }
            CGImageRelease(cg_image);
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), String> {
        Ok(())
    }
}
