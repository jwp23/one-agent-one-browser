use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct Args {
    pub target: Option<Target>,
    pub screenshot_path: Option<PathBuf>,
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
