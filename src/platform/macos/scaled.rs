use super::painter::MacPainter;
use super::scale::ScaleFactor;
use crate::render::{FontMetricsPx, Painter, TextMeasurer, TextStyle};

pub(super) struct ScaledPainter<'a> {
    inner: &'a mut MacPainter,
    scale: ScaleFactor,
}

impl<'a> ScaledPainter<'a> {
    pub fn new(inner: &'a mut MacPainter, scale: ScaleFactor) -> Self {
        Self { inner, scale }
    }

    fn scale_style(&self, style: TextStyle) -> TextStyle {
        TextStyle {
            font_size_px: self.scale.css_size_to_device_px(style.font_size_px),
            letter_spacing_px: self.scale.css_coord_to_device_px(style.letter_spacing_px),
            ..style
        }
    }
}

impl TextMeasurer for ScaledPainter<'_> {
    fn font_metrics_px(&self, style: TextStyle) -> FontMetricsPx {
        let scaled_style = self.scale_style(style);
        let metrics = self.inner.font_metrics_px(scaled_style);
        FontMetricsPx {
            ascent_px: self.scale.device_delta_to_css_px(metrics.ascent_px).max(1),
            descent_px: self.scale.device_delta_to_css_px(metrics.descent_px).max(0),
        }
    }

    fn text_width_px(&self, text: &str, style: TextStyle) -> Result<i32, String> {
        let scaled_style = self.scale_style(style);
        let width_device_px = self.inner.text_width_px(text, scaled_style)?;
        Ok(self.scale.device_delta_to_css_px(width_device_px).max(0))
    }
}

impl Painter for ScaledPainter<'_> {
    fn clear(&mut self) -> Result<(), String> {
        self.inner.clear()
    }

    fn push_opacity(&mut self, opacity: u8) -> Result<(), String> {
        self.inner.push_opacity(opacity)
    }

    fn pop_opacity(&mut self, opacity: u8) -> Result<(), String> {
        self.inner.pop_opacity(opacity)
    }

    fn fill_rect(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        color: crate::geom::Color,
    ) -> Result<(), String> {
        let (x_device_px, width_device_px) = self.scale.css_span_to_device_px(x_px, width_px);
        let (y_device_px, height_device_px) = self.scale.css_span_to_device_px(y_px, height_px);
        self.inner
            .fill_rect(x_device_px, y_device_px, width_device_px, height_device_px, color)
    }

    fn fill_rounded_rect(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        radius_px: i32,
        color: crate::geom::Color,
    ) -> Result<(), String> {
        let (x_device_px, width_device_px) = self.scale.css_span_to_device_px(x_px, width_px);
        let (y_device_px, height_device_px) = self.scale.css_span_to_device_px(y_px, height_px);
        let radius_device_px = self.scale.css_coord_to_device_px(radius_px).max(0);
        self.inner.fill_rounded_rect(
            x_device_px,
            y_device_px,
            width_device_px,
            height_device_px,
            radius_device_px,
            color,
        )
    }

    fn stroke_rounded_rect(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        radius_px: i32,
        border_width_px: i32,
        color: crate::geom::Color,
    ) -> Result<(), String> {
        let (x_device_px, width_device_px) = self.scale.css_span_to_device_px(x_px, width_px);
        let (y_device_px, height_device_px) = self.scale.css_span_to_device_px(y_px, height_px);
        let radius_device_px = self.scale.css_coord_to_device_px(radius_px).max(0);
        let border_width_device_px = self.scale.css_coord_to_device_px(border_width_px).max(0);
        self.inner.stroke_rounded_rect(
            x_device_px,
            y_device_px,
            width_device_px,
            height_device_px,
            radius_device_px,
            border_width_device_px,
            color,
        )
    }

    fn draw_text(
        &mut self,
        x_px: i32,
        y_px: i32,
        text: &str,
        style: TextStyle,
    ) -> Result<(), String> {
        let x_device_px = self.scale.css_coord_to_device_px(x_px);
        let y_device_px = self.scale.css_coord_to_device_px(y_px);
        let style = self.scale_style(style);
        self.inner.draw_text(x_device_px, y_device_px, text, style)
    }

    fn draw_image(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        image: &crate::image::Argb32Image,
        opacity: u8,
    ) -> Result<(), String> {
        let (x_device_px, width_device_px) = self.scale.css_span_to_device_px(x_px, width_px);
        let (y_device_px, height_device_px) = self.scale.css_span_to_device_px(y_px, height_px);
        self.inner.draw_image(
            x_device_px,
            y_device_px,
            width_device_px,
            height_device_px,
            image,
            opacity,
        )
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
        let (x_device_px, width_device_px) = self.scale.css_span_to_device_px(x_px, width_px);
        let (y_device_px, height_device_px) = self.scale.css_span_to_device_px(y_px, height_px);
        self.inner.draw_svg(
            x_device_px,
            y_device_px,
            width_device_px,
            height_device_px,
            svg_xml,
            opacity,
        )
    }

    fn flush(&mut self) -> Result<(), String> {
        self.inner.flush()
    }
}

