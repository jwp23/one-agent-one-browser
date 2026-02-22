use crate::geom::Color;
use crate::image::{Argb32Image, RgbImage};
use crate::render::{FontMetricsPx, Painter, TextMeasurer, TextStyle, Viewport};

use super::cairo::CairoCanvas;

pub struct WaylandPainter {
    width_px: i32,
    height_px: i32,
    bgra: Vec<u8>,
    cairo: CairoCanvas,
    opacity_depth: usize,
}

impl WaylandPainter {
    pub fn new(viewport: Viewport) -> Result<Self, String> {
        let (width_px, height_px) = validate_viewport(viewport)?;
        let mut bgra = vec![0u8; image_len(width_px, height_px)?];
        let cairo = CairoCanvas::new_image(width_px, height_px, &mut bgra)?;
        Ok(Self {
            width_px,
            height_px,
            bgra,
            cairo,
            opacity_depth: 0,
        })
    }

    pub fn ensure_back_buffer(&mut self, viewport: Viewport) -> Result<(), String> {
        let (width_px, height_px) = validate_viewport(viewport)?;
        if width_px == self.width_px && height_px == self.height_px {
            return Ok(());
        }

        self.width_px = width_px;
        self.height_px = height_px;
        self.bgra = vec![0u8; image_len(width_px, height_px)?];
        self.cairo
            .recreate_image(self.width_px, self.height_px, &mut self.bgra)?;
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

        for chunk in self.bgra.chunks_exact(4) {
            rgb.push(chunk[2]);
            rgb.push(chunk[1]);
            rgb.push(chunk[0]);
        }

        RgbImage::new(width_u32, height_u32, rgb)
    }

    pub fn bgra(&self) -> &[u8] {
        &self.bgra
    }
}

impl TextMeasurer for WaylandPainter {
    fn font_metrics_px(&self, style: TextStyle) -> FontMetricsPx {
        self.cairo.font_metrics_px(style)
    }

    fn text_width_px(&self, text: &str, style: TextStyle) -> Result<i32, String> {
        self.cairo.text_width_px(text, style)
    }
}

impl Painter for WaylandPainter {
    fn clear(&mut self) -> Result<(), String> {
        self.fill_rect(0, 0, self.width_px, self.height_px, Color::WHITE)
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
        self.cairo.fill_rect(x_px, y_px, width_px, height_px, color);
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
        self.cairo.draw_text(x_px, y_px, text, style)?;
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
        Ok(())
    }
}

fn validate_viewport(viewport: Viewport) -> Result<(i32, i32), String> {
    if viewport.width_px <= 0 || viewport.height_px <= 0 {
        return Err(format!(
            "Invalid Wayland viewport: {}x{}",
            viewport.width_px, viewport.height_px
        ));
    }
    Ok((viewport.width_px, viewport.height_px))
}

fn image_len(width_px: i32, height_px: i32) -> Result<usize, String> {
    let width: usize = width_px
        .try_into()
        .map_err(|_| format!("Width out of range: {width_px}"))?;
    let height: usize = height_px
        .try_into()
        .map_err(|_| format!("Height out of range: {height_px}"))?;
    width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "Wayland image size overflow".to_owned())
}
