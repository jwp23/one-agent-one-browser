use crate::render::{Painter, Viewport};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TickResult {
    pub needs_redraw: bool,
    pub ready_for_screenshot: bool,
    pub pending_resources: usize,
}

pub trait App {
    fn tick(&mut self) -> Result<TickResult, String>;
    fn render(&mut self, painter: &mut dyn Painter, viewport: Viewport) -> Result<(), String>;

    fn navigate_back(&mut self) -> Result<TickResult, String> {
        Ok(TickResult::default())
    }

    fn mouse_down(
        &mut self,
        _x_px: i32,
        _y_px: i32,
        _viewport: Viewport,
    ) -> Result<TickResult, String> {
        Ok(TickResult::default())
    }

    fn mouse_wheel(&mut self, _delta_y_px: i32, _viewport: Viewport) -> Result<TickResult, String> {
        Ok(TickResult::default())
    }
}
