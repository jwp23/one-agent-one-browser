use crate::render::{Painter, Viewport};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TickResult {
    pub needs_redraw: bool,
    pub ready_for_screenshot: bool,
}

pub trait App {
    fn tick(&mut self) -> Result<TickResult, String>;
    fn render(&mut self, painter: &mut dyn Painter, viewport: Viewport) -> Result<(), String>;
}

