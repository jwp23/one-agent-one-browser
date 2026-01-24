#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextStyle {
    pub bold: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Viewport {
    pub width_px: i32,
    pub height_px: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawText {
    pub x_px: i32,
    pub y_px: i32,
    pub text: String,
    pub style: TextStyle,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DisplayList {
    pub texts: Vec<DrawText>,
}

pub trait TextMeasurer {
    fn line_height_px(&self) -> i32;
    fn text_width_px(&self, text: &str) -> Result<i32, String>;
}

pub trait Painter: TextMeasurer {
    fn clear(&mut self) -> Result<(), String>;
    fn draw_text(&mut self, x_px: i32, y_px: i32, text: &str, style: TextStyle)
        -> Result<(), String>;
    fn flush(&mut self) -> Result<(), String>;
}

