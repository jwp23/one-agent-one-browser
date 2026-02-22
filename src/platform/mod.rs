#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "linux")]
mod wayland;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod x11;

use crate::app::App;
#[cfg(target_os = "linux")]
use std::ffi::OsStr;
use std::path::PathBuf;

#[derive(Debug, Default, Clone)]
pub struct WindowOptions {
    pub screenshot_path: Option<PathBuf>,
    pub headless: bool,
    pub initial_width_px: Option<i32>,
    pub initial_height_px: Option<i32>,
}

pub fn run_window(title: &str, options: WindowOptions, app: &mut impl App) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    return run_linux_window(title, options, app);

    #[cfg(target_os = "macos")]
    return macos::run_window(title, options, app);

    #[cfg(target_os = "windows")]
    return windows::run_window(title, options, app);

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = (title, options, app);
        Err(
            "Unsupported platform: this demo currently only supports Linux (X11/XWayland), macOS, and Windows"
                .to_owned(),
        )
    }
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LinuxBackend {
    X11,
    Wayland,
}

#[cfg(target_os = "linux")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LinuxBackendPreference {
    Auto,
    X11,
    Wayland,
}

#[cfg(target_os = "linux")]
fn run_linux_window(title: &str, options: WindowOptions, app: &mut impl App) -> Result<(), String> {
    let preference = linux_backend_preference_from_env()?;

    match preference {
        LinuxBackendPreference::X11 => run_linux_backend(LinuxBackend::X11, title, options, app),
        LinuxBackendPreference::Wayland => {
            run_linux_backend(LinuxBackend::Wayland, title, options, app)
        }
        LinuxBackendPreference::Auto => {
            let (primary, secondary) = if is_wayland_session() {
                (LinuxBackend::Wayland, LinuxBackend::X11)
            } else {
                (LinuxBackend::X11, LinuxBackend::Wayland)
            };

            let secondary_options = options.clone();
            match run_linux_backend(primary, title, options, app) {
                Ok(()) => Ok(()),
                Err(primary_error) => {
                    match run_linux_backend(secondary, title, secondary_options, app) {
                        Ok(()) => Ok(()),
                        Err(secondary_error) => Err(format!(
                            "Linux backend auto-selection failed.\nPrimary ({}) error: {}\nFallback ({}) error: {}",
                            backend_name(primary),
                            primary_error,
                            backend_name(secondary),
                            secondary_error,
                        )),
                    }
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn run_linux_backend(
    backend: LinuxBackend,
    title: &str,
    options: WindowOptions,
    app: &mut impl App,
) -> Result<(), String> {
    match backend {
        LinuxBackend::X11 => x11::run_window(title, options, app),
        LinuxBackend::Wayland => wayland::run_window(title, options, app),
    }
}

#[cfg(target_os = "linux")]
fn backend_name(backend: LinuxBackend) -> &'static str {
    match backend {
        LinuxBackend::X11 => "x11",
        LinuxBackend::Wayland => "wayland",
    }
}

#[cfg(target_os = "linux")]
fn linux_backend_preference_from_env() -> Result<LinuxBackendPreference, String> {
    let Some(value) = std::env::var("OAB_LINUX_BACKEND").ok() else {
        return Ok(LinuxBackendPreference::Auto);
    };
    linux_backend_preference_from_str(Some(value.as_str()))
}

#[cfg(target_os = "linux")]
fn linux_backend_preference_from_str(
    value: Option<&str>,
) -> Result<LinuxBackendPreference, String> {
    let Some(value) = value else {
        return Ok(LinuxBackendPreference::Auto);
    };
    let value = value.trim();
    if value.is_empty() {
        return Ok(LinuxBackendPreference::Auto);
    }
    if value.eq_ignore_ascii_case("auto") {
        return Ok(LinuxBackendPreference::Auto);
    }
    if value.eq_ignore_ascii_case("x11") {
        return Ok(LinuxBackendPreference::X11);
    }
    if value.eq_ignore_ascii_case("wayland") {
        return Ok(LinuxBackendPreference::Wayland);
    }
    Err(format!(
        "Invalid OAB_LINUX_BACKEND={value:?}. Expected one of: auto, x11, wayland."
    ))
}

#[cfg(target_os = "linux")]
fn is_wayland_session() -> bool {
    let wayland_display = std::env::var_os("WAYLAND_DISPLAY");
    let xdg_session_type = std::env::var("XDG_SESSION_TYPE").ok();
    is_wayland_session_from_values(wayland_display.as_deref(), xdg_session_type.as_deref())
}

#[cfg(target_os = "linux")]
fn is_wayland_session_from_values(
    wayland_display: Option<&OsStr>,
    xdg_session_type: Option<&str>,
) -> bool {
    wayland_display.is_some_and(|value| !value.is_empty())
        || xdg_session_type.is_some_and(|value| value.eq_ignore_ascii_case("wayland"))
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::{
        LinuxBackendPreference, is_wayland_session_from_values, linux_backend_preference_from_str,
    };
    use std::ffi::OsStr;

    #[test]
    fn linux_backend_preference_parses_expected_values() {
        assert_eq!(
            linux_backend_preference_from_str(None).unwrap(),
            LinuxBackendPreference::Auto
        );
        assert_eq!(
            linux_backend_preference_from_str(Some("")).unwrap(),
            LinuxBackendPreference::Auto
        );
        assert_eq!(
            linux_backend_preference_from_str(Some("auto")).unwrap(),
            LinuxBackendPreference::Auto
        );
        assert_eq!(
            linux_backend_preference_from_str(Some("x11")).unwrap(),
            LinuxBackendPreference::X11
        );
        assert_eq!(
            linux_backend_preference_from_str(Some("WAYLAND")).unwrap(),
            LinuxBackendPreference::Wayland
        );
    }

    #[test]
    fn linux_backend_preference_rejects_invalid_values() {
        let err = linux_backend_preference_from_str(Some("unknown")).unwrap_err();
        assert!(err.contains("OAB_LINUX_BACKEND"));
    }

    #[test]
    fn wayland_session_detection_handles_both_signals() {
        assert!(is_wayland_session_from_values(
            Some(OsStr::new("wayland-0")),
            None
        ));
        assert!(is_wayland_session_from_values(None, Some("wayland")));
        assert!(is_wayland_session_from_values(None, Some("WAYLAND")));
        assert!(!is_wayland_session_from_values(None, Some("x11")));
        assert!(!is_wayland_session_from_values(Some(OsStr::new("")), None));
    }
}
