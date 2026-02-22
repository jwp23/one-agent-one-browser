use crate::geom::Color;
use crate::image::{Argb32Image, RgbImage};
use crate::render::{FontMetricsPx, Painter, TextMeasurer, TextStyle, Viewport};
use core::ffi::{c_int, c_uint, c_ulong};

use super::cairo::CairoCanvas;
use super::xft::XftRenderer;
use super::xlib::{
    self, ALL_PLANES, Colormap, Display, Drawable, GC, IMAGE_FORMAT_Z_PIXMAP, Pixmap, Visual,
    Window,
};

pub struct X11Painter {
    pub display: *mut Display,
    pub window: Window,
    gc: GC,
    back_buffer: Pixmap,
    back_buffer_width: c_uint,
    back_buffer_height: c_uint,
    back_buffer_depth: c_uint,
    black_pixel: c_ulong,
    white_pixel: c_ulong,
    visual_masks: (c_ulong, c_ulong, c_ulong),
    xft: XftRenderer,
    cairo: CairoCanvas,
    opacity_depth: usize,
}

impl X11Painter {
    pub fn new(
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
        visual: *mut Visual,
        colormap: Colormap,
        screen: c_int,
    ) -> Result<Self, String> {
        let xft = XftRenderer::new(display, visual, colormap, screen, back_buffer)?;
        let cairo = CairoCanvas::new(
            display,
            back_buffer as Drawable,
            visual,
            back_buffer_width as i32,
            back_buffer_height as i32,
        )?;
        Ok(Self {
            display,
            window,
            gc,
            back_buffer,
            back_buffer_width,
            back_buffer_height,
            back_buffer_depth,
            black_pixel,
            white_pixel,
            visual_masks,
            xft,
            cairo,
            opacity_depth: 0,
        })
    }

    pub fn ensure_back_buffer(&mut self, viewport: Viewport) -> Result<(), String> {
        let width_i32 = viewport.width_px;
        let height_i32 = viewport.height_px;
        if width_i32 <= 0 || height_i32 <= 0 {
            return Err(format!("Invalid window size: {width_i32}x{height_i32}"));
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

        let new_back_buffer = unsafe {
            xlib::XCreatePixmap(
                self.display,
                self.window,
                width,
                height,
                self.back_buffer_depth,
            )
        };
        if new_back_buffer == 0 {
            return Err("XCreatePixmap failed during resize".to_owned());
        }

        self.xft.recreate_draw(new_back_buffer as Drawable)?;
        self.cairo
            .recreate(new_back_buffer as Drawable, width_i32, height_i32)?;

        unsafe {
            xlib::XFreePixmap(self.display, self.back_buffer);
        }

        self.back_buffer = new_back_buffer;
        self.back_buffer_width = width;
        self.back_buffer_height = height;
        Ok(())
    }

    pub fn destroy_xft_resources(&mut self) {
        self.xft.destroy();
        self.cairo.destroy();
    }

    pub fn back_buffer(&self) -> Pixmap {
        self.back_buffer
    }

    pub fn capture_back_buffer_rgb(&self) -> Result<RgbImage, String> {
        let width_u32: u32 = self
            .back_buffer_width
            .try_into()
            .map_err(|_| "Screenshot width out of range".to_owned())?;
        let height_u32: u32 = self
            .back_buffer_height
            .try_into()
            .map_err(|_| "Screenshot height out of range".to_owned())?;

        let ximage = unsafe {
            xlib::XGetImage(
                self.display,
                self.back_buffer,
                0,
                0,
                self.back_buffer_width,
                self.back_buffer_height,
                ALL_PLANES,
                IMAGE_FORMAT_Z_PIXMAP,
            )
        };
        if ximage.is_null() {
            return Err("XGetImage returned null".to_owned());
        }
        let ximage = xlib::XImageHandle(ximage);

        let (masks, get_pixel) = unsafe {
            let masks = (
                (*ximage.0).red_mask,
                (*ximage.0).green_mask,
                (*ximage.0).blue_mask,
            );
            let masks = if masks.0 == 0 && masks.1 == 0 && masks.2 == 0 {
                self.visual_masks
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

    fn set_foreground(&mut self, color: Color) {
        let pixel = self.pixel_for_color(color);
        unsafe {
            xlib::XSetForeground(self.display, self.gc, pixel);
        }
    }

    fn pixel_for_color(&self, color: Color) -> c_ulong {
        if color == Color::BLACK {
            return self.black_pixel;
        }
        if color == Color::WHITE {
            return self.white_pixel;
        }

        let (rm, gm, bm) = self.visual_masks;
        let r = pack_channel_u8(color.r, rm);
        let g = pack_channel_u8(color.g, gm);
        let b = pack_channel_u8(color.b, bm);
        r | g | b
    }
}

impl TextMeasurer for X11Painter {
    fn font_metrics_px(&self, style: TextStyle) -> FontMetricsPx {
        self.xft.font_metrics_px(style)
    }

    fn text_width_px(&self, text: &str, style: TextStyle) -> Result<i32, String> {
        self.xft.text_width_px(text, style)
    }
}

impl Painter for X11Painter {
    fn clear(&mut self) -> Result<(), String> {
        self.fill_rect(
            0,
            0,
            self.back_buffer_width as i32,
            self.back_buffer_height as i32,
            Color::WHITE,
        )
    }

    fn push_opacity(&mut self, opacity: u8) -> Result<(), String> {
        if opacity >= 255 {
            return Ok(());
        }
        self.opacity_depth = self.opacity_depth.saturating_add(1);
        self.cairo.push_group();
        Ok(())
    }

    fn pop_opacity(&mut self, opacity: u8) -> Result<(), String> {
        if self.opacity_depth == 0 {
            return Err("opacity stack underflow".to_owned());
        }
        self.opacity_depth -= 1;
        self.cairo.pop_group_with_alpha(opacity);
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

        if self.opacity_depth > 0 || color.a != 255 {
            self.cairo.fill_rect(x_px, y_px, width_px, height_px, color);
            return Ok(());
        }

        let width: c_uint = width_px
            .try_into()
            .map_err(|_| "rect width out of range for X11".to_owned())?;
        let height: c_uint = height_px
            .try_into()
            .map_err(|_| "rect height out of range for X11".to_owned())?;

        self.set_foreground(color);
        unsafe {
            xlib::XFillRectangle(
                self.display,
                self.back_buffer,
                self.gc,
                x_px,
                y_px,
                width,
                height,
            );
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
        self.cairo
            .fill_rounded_rect(x_px, y_px, width_px, height_px, radius_px, color);
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
        self.cairo.stroke_rounded_rect(
            x_px,
            y_px,
            width_px,
            height_px,
            radius_px,
            border_width_px,
            color,
        );
        Ok(())
    }

    fn draw_text(
        &mut self,
        x_px: i32,
        y_px: i32,
        text: &str,
        style: TextStyle,
    ) -> Result<(), String> {
        if self.opacity_depth == 0 {
            self.xft.draw_text(x_px, y_px, text, style)?;
        } else {
            self.cairo.draw_text(x_px, y_px, text, style)?;
        }

        if style.underline {
            let width_px = self.text_width_px(text, style)?;
            self.fill_rect(x_px, y_px.saturating_add(1), width_px, 1, style.color)?;
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
        if width_px <= 0 || height_px <= 0 {
            return Ok(());
        }
        if opacity == 0 {
            return Ok(());
        }
        if image.width == 0 || image.height == 0 {
            return Ok(());
        }

        let mut data = image.data.clone();
        let surface = self.cairo.create_argb32_surface_for_data(
            &mut data,
            image.width as i32,
            image.height as i32,
            image.row_stride_bytes() as i32,
        )?;
        self.cairo.draw_image_surface(
            x_px,
            y_px,
            width_px,
            height_px,
            surface,
            image.width as i32,
            image.height as i32,
            opacity,
        );
        self.cairo.destroy_surface(surface);
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
        self.cairo
            .draw_svg(x_px, y_px, width_px, height_px, svg_xml, opacity)
    }

    fn flush(&mut self) -> Result<(), String> {
        unsafe {
            xlib::XCopyArea(
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
            xlib::XFlush(self.display);
        }
        Ok(())
    }
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
    let max = if bits == 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    if max == 0 {
        return 0;
    }

    if max == 255 {
        return value as u8;
    }

    let scaled = (value * 255 + max / 2) / max;
    scaled as u8
}

fn pack_channel_u8(value: u8, mask: c_ulong) -> c_ulong {
    if mask == 0 {
        return 0;
    }
    let mask_u64 = mask as u64;
    let shift = mask_u64.trailing_zeros();
    let bits = mask_u64.count_ones();
    if bits == 0 || shift >= 64 {
        return 0;
    }

    let max = if bits == 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    if max == 0 {
        return 0;
    }

    let scaled = (u64::from(value) * max + 127) / 255;
    ((scaled << shift) & mask_u64) as c_ulong
}
