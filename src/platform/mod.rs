#[cfg(target_os = "linux")]
mod x11;

use crate::render::{Painter, Viewport};
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct WindowOptions {
    pub screenshot_path: Option<PathBuf>,
}

pub fn run_window<F>(title: &str, options: WindowOptions, render: F) -> Result<(), String>
where
    F: FnMut(&mut dyn Painter, Viewport) -> Result<(), String>,
{
    #[cfg(target_os = "linux")]
    return x11::run_window(title, options, render);

    #[cfg(not(target_os = "linux"))]
    Err("Unsupported platform: this demo currently only supports Linux/X11".to_owned())
}
