use std::ffi::OsString;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    let parsed = match parse_args(&args) {
        Ok(parsed) => parsed,
        Err(err) => {
            eprintln!("{err}\n");
            eprintln!("Usage: fetch-resource <url> [--out <path>]");
            eprintln!("Fetches a URL using the browser's libcurl wrapper, prints basic info,");
            eprintln!("and optionally writes the response bytes to disk.");
            return ExitCode::from(2);
        }
    };

    let bytes = match one_agent_one_browser::net::fetch_url_bytes(&parsed.url) {
        Ok(bytes) => bytes,
        Err(err) => {
            eprintln!("Fetch failed: {err}");
            return ExitCode::from(1);
        }
    };

    println!("URL:   {}", parsed.url);
    println!("Bytes: {}", bytes.len());
    println!("Head:  {}", hex_prefix(&bytes, 32));

    match one_agent_one_browser::image::decode_image(&bytes) {
        Ok(image) => {
            println!("Decode: OK ({}x{}, argb32)", image.width, image.height);
        }
        Err(err) => {
            println!("Decode: FAIL ({err})");
        }
    }

    if let Some(out_path) = parsed.out_path {
        if let Err(err) = std::fs::write(&out_path, &bytes) {
            eprintln!("Failed to write {}: {err}", out_path.display());
            return ExitCode::from(1);
        }
        println!("Wrote: {}", out_path.display());
    }

    ExitCode::SUCCESS
}

#[derive(Debug)]
struct Args {
    url: String,
    out_path: Option<PathBuf>,
}

fn parse_args(args: &[OsString]) -> Result<Args, String> {
    let mut url: Option<String> = None;
    let mut out_path: Option<PathBuf> = None;

    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        let Some(s) = arg.to_str() else {
            return Err("Argument is not valid UTF-8".to_owned());
        };

        match s {
            "--out" => {
                let next = iter
                    .next()
                    .ok_or_else(|| "Missing value for --out".to_owned())?;
                if out_path.is_some() {
                    return Err("Duplicate --out flag".to_owned());
                }
                out_path = Some(PathBuf::from(next));
            }
            _ if s.starts_with("--out=") => {
                if out_path.is_some() {
                    return Err("Duplicate --out flag".to_owned());
                }
                let value = s.trim_start_matches("--out=");
                if value.is_empty() {
                    return Err("Invalid --out=... value: empty path".to_owned());
                }
                out_path = Some(PathBuf::from(value));
            }
            _ if s.starts_with('-') => return Err(format!("Unknown flag: {s}")),
            _ => {
                if url.is_some() {
                    return Err("Unexpected extra argument".to_owned());
                }
                url = Some(s.to_owned());
            }
        }
    }

    let url = url.ok_or_else(|| "Missing URL argument".to_owned())?;
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_owned());
    }

    Ok(Args { url, out_path })
}

fn hex_prefix(bytes: &[u8], max_len: usize) -> String {
    let mut out = String::new();
    for (idx, &b) in bytes.iter().take(max_len).enumerate() {
        if idx > 0 {
            out.push(' ');
        }
        out.push_str(&format!("{b:02x}"));
    }
    if bytes.len() > max_len {
        out.push_str(" …");
    }
    out
}
