use crate::geom::Color;
use crate::style::FontFamily;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextStyle {
    pub color: Color,
    pub bold: bool,
    pub underline: bool,
    pub font_family: FontFamily,
    pub font_size_px: i32,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            bold: false,
            underline: false,
            font_family: FontFamily::SansSerif,
            font_size_px: 16,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FontMetricsPx {
    pub ascent_px: i32,
    pub descent_px: i32,
}

impl FontMetricsPx {
    pub fn line_height_px(self) -> i32 {
        self.ascent_px.saturating_add(self.descent_px).max(1)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Viewport {
    pub width_px: i32,
    pub height_px: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawRect {
    pub x_px: i32,
    pub y_px: i32,
    pub width_px: i32,
    pub height_px: i32,
    pub color: Color,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawText {
    pub x_px: i32,
    pub y_px: i32,
    pub text: String,
    pub style: TextStyle,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DisplayCommand {
    Rect(DrawRect),
    Text(DrawText),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DisplayList {
    pub commands: Vec<DisplayCommand>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkHitRegion {
    pub href: Rc<str>,
    pub x_px: i32,
    pub y_px: i32,
    pub width_px: i32,
    pub height_px: i32,
}

impl LinkHitRegion {
    pub fn contains_point(&self, x_px: i32, y_px: i32) -> bool {
        if self.width_px <= 0 || self.height_px <= 0 {
            return false;
        }
        let within_x = x_px >= self.x_px && x_px < self.x_px.saturating_add(self.width_px);
        let within_y = y_px >= self.y_px && y_px < self.y_px.saturating_add(self.height_px);
        within_x && within_y
    }
}

pub trait TextMeasurer {
    fn font_metrics_px(&self, style: TextStyle) -> FontMetricsPx;
    fn text_width_px(&self, text: &str, style: TextStyle) -> Result<i32, String>;
}

pub trait Painter: TextMeasurer {
    fn clear(&mut self) -> Result<(), String>;
    fn fill_rect(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        color: Color,
    ) -> Result<(), String>;
    fn draw_text(&mut self, x_px: i32, y_px: i32, text: &str, style: TextStyle)
        -> Result<(), String>;
    fn flush(&mut self) -> Result<(), String>;
}
