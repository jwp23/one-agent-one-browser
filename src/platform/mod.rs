#[cfg(target_os = "linux")]
mod x11;

pub fn run_hello_world_window() -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return x11::run_hello_world_window();

    #[cfg(not(target_os = "linux"))]
    Err("Unsupported platform: this demo currently only supports Linux/X11".to_owned())
}

