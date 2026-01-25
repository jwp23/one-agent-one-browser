use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct Args {
    pub target: Option<Target>,
    pub screenshot_path: Option<PathBuf>,
    pub headless: bool,
    pub width_px: Option<i32>,
    pub height_px: Option<i32>,
}

#[derive(Debug)]
pub enum Target {
    File(PathBuf),
    Url(String),
}

pub fn parse_args(mut args: impl Iterator<Item = OsString>) -> Result<Args, String> {
    let mut parsed = Args::default();

    while let Some(arg) = args.next() {
        if let Some(flag) = arg.to_str() {
            if let Some(value) = flag.strip_prefix("--width=") {
                if parsed.width_px.is_some() {
                    return Err("Duplicate --width flag".to_owned());
                }
                parsed.width_px = Some(parse_dimension_px(value, "--width")?);
                continue;
            }

            if flag == "--width" {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --width".to_owned())?;
                let value = value.to_string_lossy();
                if parsed.width_px.is_some() {
                    return Err("Duplicate --width flag".to_owned());
                }
                parsed.width_px = Some(parse_dimension_px(&value, "--width")?);
                continue;
            }

            if let Some(value) = flag.strip_prefix("--height=") {
                if parsed.height_px.is_some() {
                    return Err("Duplicate --height flag".to_owned());
                }
                parsed.height_px = Some(parse_dimension_px(value, "--height")?);
                continue;
            }

            if flag == "--height" {
                let value = args
                    .next()
                    .ok_or_else(|| "Missing value for --height".to_owned())?;
                let value = value.to_string_lossy();
                if parsed.height_px.is_some() {
                    return Err("Duplicate --height flag".to_owned());
                }
                parsed.height_px = Some(parse_dimension_px(&value, "--height")?);
                continue;
            }

            if let Some(path) = flag.strip_prefix("--screenshot=") {
                if path.is_empty() {
                    return Err("Invalid --screenshot=... value: path is empty".to_owned());
                }
                if parsed.screenshot_path.is_some() {
                    return Err("Duplicate --screenshot flag".to_owned());
                }
                parsed.screenshot_path = Some(PathBuf::from(path));
                continue;
            }

            if flag == "--screenshot" {
                let path = args
                    .next()
                    .ok_or_else(|| "Missing value for --screenshot".to_owned())?;
                if parsed.screenshot_path.is_some() {
                    return Err("Duplicate --screenshot flag".to_owned());
                }
                parsed.screenshot_path = Some(PathBuf::from(path));
                continue;
            }

            if flag == "--headless" {
                if parsed.headless {
                    return Err("Duplicate --headless flag".to_owned());
                }
                parsed.headless = true;
                continue;
            }

            if flag.starts_with('-') {
                return Err(format!("Unknown flag: {flag}"));
            }
        }

        if parsed.target.is_some() {
            return Err("Unexpected extra argument (expected a single HTML file path)".to_owned());
        }

        if let Some(s) = arg.to_str() {
            if s.starts_with("http://") || s.starts_with("https://") {
                parsed.target = Some(Target::Url(s.to_owned()));
                continue;
            }
        }
        parsed.target = Some(Target::File(PathBuf::from(arg)));
    }

    Ok(parsed)
}

fn parse_dimension_px(value: &str, flag: &str) -> Result<i32, String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("Invalid {flag} value: empty"));
    }
    let px: i32 = value
        .parse()
        .map_err(|_| format!("Invalid {flag} value: expected an integer, got {value:?}"))?;
    if px <= 0 {
        return Err(format!("Invalid {flag} value: must be > 0, got {px}"));
    }
    Ok(px)
}
