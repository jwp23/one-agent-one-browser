#[cfg(target_os = "linux")]
mod x11;

use crate::app::App;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct WindowOptions {
    pub screenshot_path: Option<PathBuf>,
    pub headless: bool,
    pub initial_width_px: Option<i32>,
    pub initial_height_px: Option<i32>,
}

pub fn run_window(title: &str, options: WindowOptions, app: &mut impl App) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return x11::run_window(title, options, app);

    #[cfg(not(target_os = "linux"))]
    Err("Unsupported platform: this demo currently only supports Linux/X11".to_owned())
}
