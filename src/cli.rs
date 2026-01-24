use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct Args {
    pub html_path: Option<PathBuf>,
    pub screenshot_path: Option<PathBuf>,
}

pub fn parse_args(mut args: impl Iterator<Item = OsString>) -> Result<Args, String> {
    let mut parsed = Args::default();

    while let Some(arg) = args.next() {
        if let Some(flag) = arg.to_str() {
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

            if flag.starts_with('-') {
                return Err(format!("Unknown flag: {flag}"));
            }
        }

        if parsed.html_path.is_some() {
            return Err("Unexpected extra argument (expected a single HTML file path)".to_owned());
        }
        parsed.html_path = Some(PathBuf::from(arg));
    }

    Ok(parsed)
}

