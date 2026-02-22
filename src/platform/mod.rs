#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
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

    #[cfg(target_os = "macos")]
    return macos::run_window(title, options, app);

    #[cfg(target_os = "windows")]
    return windows::run_window(title, options, app);

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = (title, options, app);
        Err(
            "Unsupported platform: this demo currently only supports Linux/X11, macOS, and Windows"
                .to_owned(),
        )
    }
}
