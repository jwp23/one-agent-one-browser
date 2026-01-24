use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

const DEFAULT_RENDER_TIMEOUT: Duration = Duration::from_secs(10);

fn main() -> ExitCode {
    let args = match parse_args(std::env::args_os().skip(1)) {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{err}\n");
            eprintln!("Usage: render-test <case.html|case.url> [more ...]");
            eprintln!("Each case must have a baseline PNG next to it: same path with a .png extension.");
            eprintln!("A .url case file contains a single URL (first non-empty, non-comment line).");
            return ExitCode::from(2);
        }
    };

    let browser_exe = match find_browser_exe() {
        Ok(path) => path,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(2);
        }
    };

    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("render-test");
    if let Err(err) = std::fs::create_dir_all(&output_dir) {
        eprintln!("Failed to create {}: {err}", output_dir.display());
        return ExitCode::from(2);
    }

    println!("Browser: {}", browser_exe.display());
    println!("Output:  {}", output_dir.display());

    let mut passed = 0usize;
    let mut failed = 0usize;

    for case_path in &args.case_paths {
        match run_case(&browser_exe, &output_dir, case_path) {
            Ok(()) => passed += 1,
            Err(err) => {
                failed += 1;
                eprintln!("{err}");
            }
        }
    }

    println!("Summary: {passed} passed, {failed} failed");
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

#[derive(Debug)]
struct Args {
    case_paths: Vec<PathBuf>,
}

fn parse_args(args: impl Iterator<Item = OsString>) -> Result<Args, String> {
    let case_paths: Vec<PathBuf> = args.map(PathBuf::from).collect();
    if case_paths.is_empty() {
        return Err("Missing case path(s).".to_owned());
    }
    Ok(Args { case_paths })
}

fn find_browser_exe() -> Result<PathBuf, String> {
    let this_exe =
        std::env::current_exe().map_err(|err| format!("Failed to find current exe: {err}"))?;
    let dir = this_exe
        .parent()
        .ok_or_else(|| format!("Failed to get parent directory of {}", this_exe.display()))?;

    let browser_name = format!("one-agent-one-browser{}", std::env::consts::EXE_SUFFIX);
    let candidate = dir.join(browser_name);
    if candidate.is_file() {
        return Ok(candidate);
    }

    let fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("debug")
        .join(format!("one-agent-one-browser{}", std::env::consts::EXE_SUFFIX));
    if fallback.is_file() {
        return Ok(fallback);
    }

    let fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join(format!("one-agent-one-browser{}", std::env::consts::EXE_SUFFIX));
    if fallback.is_file() {
        return Ok(fallback);
    }

    Err(format!(
        "Failed to locate the browser binary next to {}.\nExpected: {}\nHint: run `cargo build` first.",
        this_exe.display(),
        candidate.display(),
    ))
}

fn run_case(browser_exe: &Path, output_dir: &Path, html_path: &Path) -> Result<(), String> {
    let expected_png = html_path.with_extension("png");
    let case_target = parse_case_target(html_path)?;
    if !expected_png.is_file() {
        return Err(match &case_target {
            CaseTarget::File => format!(
                "FAIL {}\nMissing baseline PNG: {}\nHint: generate it with:\n  {} {} --screenshot={}\n",
                html_path.display(),
                expected_png.display(),
                browser_exe.display(),
                html_path.display(),
                expected_png.display(),
            ),
            CaseTarget::Url(url) => format!(
                "FAIL {}\nMissing baseline PNG: {}\nURL: {url}\nHint: generate it with:\n  {} \"{url}\" --screenshot={}\n",
                html_path.display(),
                expected_png.display(),
                browser_exe.display(),
                expected_png.display(),
            ),
        });
    }

    let actual_png = output_dir.join(actual_filename_for_html(html_path)?);

    println!("Case: {}", html_path.display());
    if let CaseTarget::Url(url) = &case_target {
        println!("URL:  {url}");
    }
    let browser_arg = match &case_target {
        CaseTarget::File => OsString::from(html_path),
        CaseTarget::Url(url) => OsString::from(url),
    };
    render_to_png(browser_exe, &browser_arg, &actual_png, DEFAULT_RENDER_TIMEOUT)?;

    let comparison = compare_files(&expected_png, &actual_png)?;
    if comparison.matches {
        let _ = std::fs::remove_file(&actual_png);
        println!("PASS {}", html_path.display());
        Ok(())
    } else {
        Err(format!(
            "FAIL {}\nExpected: {} (len={}, fnv1a64={})\nActual:   {} (len={}, fnv1a64={})\n{}\nHint: to accept the new output:\n  cp {} {}\n",
            html_path.display(),
            expected_png.display(),
            comparison.expected.len,
            format_u64_hex(comparison.expected.fnv1a64),
            actual_png.display(),
            comparison.actual.len,
            format_u64_hex(comparison.actual.fnv1a64),
            comparison
                .first_difference
                .map(|offset| format!("First differing byte offset: {offset}"))
                .unwrap_or_else(|| "Files differ.".to_owned()),
            actual_png.display(),
            expected_png.display(),
        ))
    }
}

fn actual_filename_for_html(html_path: &Path) -> Result<String, String> {
    let stem = html_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("case");
    let hash = fnv1a64(html_path.as_os_str().to_string_lossy().as_bytes());
    Ok(format!("{stem}.{:016x}.actual.png", hash))
}

fn render_to_png(
    browser_exe: &Path,
    browser_arg: &OsString,
    png_path: &Path,
    timeout: Duration,
) -> Result<(), String> {
    let _ = std::fs::remove_file(png_path);

    let screenshot_arg = format!("--screenshot={}", png_path.display());
    let mut child = Command::new(browser_exe)
        .arg(browser_arg)
        .arg(screenshot_arg)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .map_err(|err| format!("Failed to start {}: {err}", browser_exe.display()))?;

    let start = Instant::now();
    loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|err| format!("Failed to poll browser process: {err}"))?
        {
            if !status.success() {
                return Err(format!(
                    "Browser process failed (exit={})",
                    status.code().unwrap_or(-1)
                ));
            }

            if !png_path.is_file() {
                return Err(format!(
                    "Browser process exited successfully but screenshot was not created: {}",
                    png_path.display()
                ));
            }

            return Ok(());
        }

        if start.elapsed() > timeout {
            let _ = child.kill();
            return Err(format!(
                "Browser render timed out after {:?} for {}",
                timeout,
                browser_arg.to_string_lossy()
            ));
        }

        std::thread::sleep(Duration::from_millis(50));
    }
}

#[derive(Debug)]
enum CaseTarget {
    File,
    Url(String),
}

fn parse_case_target(case_path: &Path) -> Result<CaseTarget, String> {
    if case_path.extension().and_then(|s| s.to_str()) != Some("url") {
        return Ok(CaseTarget::File);
    }

    let raw = std::fs::read_to_string(case_path)
        .map_err(|err| format!("Failed to read {}: {err}", case_path.display()))?;
    let url = raw
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .ok_or_else(|| format!("{} does not contain a URL", case_path.display()))?;
    Ok(CaseTarget::Url(url.to_owned()))
}

#[derive(Clone, Copy, Debug)]
struct FileDigest {
    len: u64,
    fnv1a64: u64,
}

#[derive(Clone, Copy, Debug)]
struct FileComparison {
    matches: bool,
    expected: FileDigest,
    actual: FileDigest,
    first_difference: Option<u64>,
}

fn compare_files(expected: &Path, actual: &Path) -> Result<FileComparison, String> {
    let expected_bytes = std::fs::read(expected)
        .map_err(|err| format!("Failed to read {}: {err}", expected.display()))?;
    let actual_bytes = std::fs::read(actual)
        .map_err(|err| format!("Failed to read {}: {err}", actual.display()))?;

    let expected_digest = FileDigest {
        len: expected_bytes.len() as u64,
        fnv1a64: fnv1a64(&expected_bytes),
    };
    let actual_digest = FileDigest {
        len: actual_bytes.len() as u64,
        fnv1a64: fnv1a64(&actual_bytes),
    };

    if expected_bytes == actual_bytes {
        return Ok(FileComparison {
            matches: true,
            expected: expected_digest,
            actual: actual_digest,
            first_difference: None,
        });
    }

    let first_difference = expected_bytes
        .iter()
        .zip(&actual_bytes)
        .position(|(a, b)| a != b)
        .map(|offset| offset as u64);

    Ok(FileComparison {
        matches: false,
        expected: expected_digest,
        actual: actual_digest,
        first_difference,
    })
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn format_u64_hex(value: u64) -> String {
    format!("0x{value:016x}")
}
