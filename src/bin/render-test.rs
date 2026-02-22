use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

const DEFAULT_RENDER_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_MIN_SIMILARITY: f64 = 0.95;
const MIN_SIMILARITY_ENV: &str = "OAB_RENDER_TEST_MIN_SIMILARITY";

fn main() -> ExitCode {
    let args = match parse_args(std::env::args_os().skip(1)) {
        Ok(args) => args,
        Err(err) => {
            eprintln!("{err}\n");
            eprintln!("Usage: render-test <case.html> [more ...]");
            eprintln!(
                "Each case must have a baseline PNG next to it: <stem>-<platform>.png (e.g. hello-strong-macos.png)."
            );
            eprintln!(
                "{MIN_SIMILARITY_ENV} controls how similar the PNGs must be to pass (default: {DEFAULT_MIN_SIMILARITY})."
            );
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
    println!("Baseline platform: {}", baseline_platform_label());

    let min_similarity = match read_min_similarity() {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{err}");
            return ExitCode::from(2);
        }
    };
    println!("Min similarity: {:.3}", min_similarity);

    let mut passed = 0usize;
    let mut failed = 0usize;

    for case_path in &args.case_paths {
        match run_case(&browser_exe, &output_dir, case_path, min_similarity) {
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
        .join(format!(
            "one-agent-one-browser{}",
            std::env::consts::EXE_SUFFIX
        ));
    if fallback.is_file() {
        return Ok(fallback);
    }

    let fallback = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("release")
        .join(format!(
            "one-agent-one-browser{}",
            std::env::consts::EXE_SUFFIX
        ));
    if fallback.is_file() {
        return Ok(fallback);
    }

    Err(format!(
        "Failed to locate the browser binary next to {}.\nExpected: {}\nHint: run `cargo build` first.",
        this_exe.display(),
        candidate.display(),
    ))
}

fn run_case(
    browser_exe: &Path,
    output_dir: &Path,
    html_path: &Path,
    min_similarity: f64,
) -> Result<(), String> {
    let expected_png = expected_baseline_png(html_path)?;
    if !expected_png.is_file() {
        return Err(format!(
            "FAIL {}\nMissing baseline PNG: {}\nHint: generate it with:\n  {} {} --screenshot={}\n",
            html_path.display(),
            expected_png.display(),
            browser_exe.display(),
            html_path.display(),
            expected_png.display(),
        ));
    }

    let actual_png = output_dir.join(actual_filename_for_html(html_path)?);

    println!("Case: {}", html_path.display());
    let browser_arg = html_path.as_os_str().to_owned();
    render_to_png(
        browser_exe,
        &browser_arg,
        &actual_png,
        DEFAULT_RENDER_TIMEOUT,
    )?;

    let comparison = compare_files(&expected_png, &actual_png, min_similarity)?;
    if comparison.matches {
        if let Some(png_diff) = &comparison.png_diff {
            if let Some(diff_png) = &png_diff.diff_png {
                let _ = std::fs::remove_file(diff_png);
            }
        }
        let _ = std::fs::remove_file(&actual_png);
        if let Some(png_diff) = &comparison.png_diff {
            if png_diff.diff_pixels > 0 {
                println!(
                    "PASS {} (similarity={:.4} >= {:.4})",
                    html_path.display(),
                    similarity_ratio(png_diff),
                    min_similarity
                );
                return Ok(());
            }
        }
        println!("PASS {}", html_path.display());
        Ok(())
    } else {
        let note_details = comparison
            .note
            .as_ref()
            .map(|note| format!("\n{note}\n"))
            .unwrap_or_default();
        let diff_details = if let Some(png_diff) = &comparison.png_diff {
            format!(
                "\nPixels:     {} / {}\nSimilarity: {:.4} (min {:.4})\nBBox:       {}\nDiff PNG:   {}\n",
                png_diff.diff_pixels,
                png_diff.total_pixels,
                similarity_ratio(png_diff),
                min_similarity,
                png_diff
                    .bbox
                    .map(|bbox| format!(
                        "x={}..{} y={}..{} ({}x{})",
                        bbox.min_x,
                        bbox.max_x,
                        bbox.min_y,
                        bbox.max_y,
                        bbox.width(),
                        bbox.height()
                    ))
                    .unwrap_or_else(|| "none".to_owned()),
                png_diff
                    .diff_png
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "none".to_owned()),
            )
        } else {
            String::new()
        };
        Err(format!(
            "FAIL {}\nExpected: {} (len={}, fnv1a64={})\nActual:   {} (len={}, fnv1a64={})\n{}\n{}{}Hint: to accept the new output:\n  cp {} {}\n",
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
            note_details,
            diff_details,
            actual_png.display(),
            expected_png.display(),
        ))
    }
}

fn read_min_similarity() -> Result<f64, String> {
    let value_os = match std::env::var_os(MIN_SIMILARITY_ENV) {
        Some(value) => value,
        None => return Ok(DEFAULT_MIN_SIMILARITY),
    };

    let value = value_os.to_str().ok_or_else(|| {
        format!("{MIN_SIMILARITY_ENV} must be valid UTF-8 and a float in [0.0, 1.0].")
    })?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!(
            "{MIN_SIMILARITY_ENV} is set but empty. Expected a float in [0.0, 1.0]."
        ));
    }

    let parsed: f64 = trimmed.parse().map_err(|err| {
        format!("Invalid {MIN_SIMILARITY_ENV}={value:?}: {err}. Expected a float in [0.0, 1.0].")
    })?;
    if !parsed.is_finite() || !(0.0..=1.0).contains(&parsed) {
        return Err(format!(
            "Invalid {MIN_SIMILARITY_ENV}={value:?}: expected a finite float in [0.0, 1.0]."
        ));
    }

    Ok(parsed)
}

fn similarity_ratio(diff: &PngDiff) -> f64 {
    if diff.total_pixels == 0 {
        return 0.0;
    }
    (diff.total_pixels.saturating_sub(diff.diff_pixels)) as f64 / diff.total_pixels as f64
}

fn expected_baseline_png(case_path: &Path) -> Result<PathBuf, String> {
    let parent = case_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = case_path
        .file_stem()
        .ok_or_else(|| format!("Invalid case path: {}", case_path.display()))?
        .to_string_lossy();
    let candidates: Vec<PathBuf> = baseline_platform_tags()
        .into_iter()
        .map(|tag| parent.join(format!("{stem}-{tag}.png")))
        .collect();
    if let Some(existing) = candidates.iter().find(|path| path.is_file()) {
        return Ok(existing.to_path_buf());
    }
    candidates
        .into_iter()
        .next()
        .ok_or_else(|| "Internal error: baseline_platform_tags returned no candidates".to_owned())
}

fn baseline_platform_label() -> String {
    baseline_platform_tags().join(" -> ")
}

fn baseline_platform_tags() -> Vec<&'static str> {
    #[cfg(target_os = "linux")]
    {
        return linux_baseline_platform_tags(
            std::env::var_os("WAYLAND_DISPLAY").as_deref(),
            std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
        );
    }
    #[cfg(target_os = "macos")]
    {
        return vec!["macos"];
    }
    #[cfg(target_os = "windows")]
    {
        return vec!["windows"];
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        return vec!["unknown"];
    }
}

#[cfg(target_os = "linux")]
fn linux_baseline_platform_tags(
    wayland_display: Option<&OsStr>,
    xdg_session_type: Option<&str>,
) -> Vec<&'static str> {
    if is_wayland_session(wayland_display, xdg_session_type) {
        vec!["linux-wayland", "linux-x11"]
    } else {
        vec!["linux-x11", "linux-wayland"]
    }
}

#[cfg(target_os = "linux")]
fn is_wayland_session(wayland_display: Option<&OsStr>, xdg_session_type: Option<&str>) -> bool {
    wayland_display.is_some_and(|value| !value.is_empty())
        || xdg_session_type.is_some_and(|value| value.eq_ignore_ascii_case("wayland"))
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
    let mut cmd = Command::new(browser_exe);
    if std::env::var_os("OAB_SCALE").is_none() {
        cmd.env("OAB_SCALE", "1");
    }
    let mut child = cmd
        .arg("--headless")
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

#[derive(Clone, Copy, Debug)]
struct FileDigest {
    len: u64,
    fnv1a64: u64,
}

#[derive(Debug)]
struct FileComparison {
    matches: bool,
    expected: FileDigest,
    actual: FileDigest,
    first_difference: Option<u64>,
    note: Option<String>,
    png_diff: Option<PngDiff>,
}

fn compare_files(
    expected: &Path,
    actual: &Path,
    min_similarity: f64,
) -> Result<FileComparison, String> {
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
            note: None,
            png_diff: None,
        });
    }

    let first_difference = expected_bytes
        .iter()
        .zip(&actual_bytes)
        .position(|(a, b)| a != b)
        .map(|offset| offset as u64);

    let mut note = None;
    let mut png_diff = None;
    match compare_png_pixels(expected, actual) {
        Ok(diff) => {
            if diff.diff_pixels == 0 {
                return Ok(FileComparison {
                    matches: true,
                    expected: expected_digest,
                    actual: actual_digest,
                    first_difference,
                    note: None,
                    png_diff: None,
                });
            }

            let similarity = similarity_ratio(&diff);
            if similarity >= min_similarity {
                return Ok(FileComparison {
                    matches: true,
                    expected: expected_digest,
                    actual: actual_digest,
                    first_difference,
                    note: None,
                    png_diff: Some(diff),
                });
            }

            png_diff = Some(diff);
        }
        Err(err) => note = Some(format!("PNG diff unavailable: {err}")),
    }

    Ok(FileComparison {
        matches: false,
        expected: expected_digest,
        actual: actual_digest,
        first_difference,
        note,
        png_diff,
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

#[derive(Clone, Copy, Debug)]
struct PixelBbox {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

impl PixelBbox {
    fn width(self) -> u32 {
        self.max_x.saturating_sub(self.min_x).saturating_add(1)
    }

    fn height(self) -> u32 {
        self.max_y.saturating_sub(self.min_y).saturating_add(1)
    }
}

#[derive(Debug)]
struct PngDiff {
    diff_pixels: u64,
    total_pixels: u64,
    bbox: Option<PixelBbox>,
    diff_png: Option<PathBuf>,
}

fn compare_png_pixels(expected: &Path, actual: &Path) -> Result<PngDiff, String> {
    let expected_img = read_png_rgb(expected)?;
    let actual_img = read_png_rgb(actual)?;

    if expected_img.width != actual_img.width || expected_img.height != actual_img.height {
        return Err(format!(
            "PNG dimensions differ: expected {}x{}, got {}x{}",
            expected_img.width, expected_img.height, actual_img.width, actual_img.height
        ));
    }

    let total_pixels = (expected_img.width as u64) * (expected_img.height as u64);
    let mut diff_pixels: u64 = 0;
    let mut bbox: Option<PixelBbox> = None;

    let mut diff_rgb = Vec::with_capacity(expected_img.rgb.len());

    for (idx, (e, a)) in expected_img
        .rgb
        .chunks_exact(3)
        .zip(actual_img.rgb.chunks_exact(3))
        .enumerate()
    {
        if e == a {
            diff_rgb.extend_from_slice(&fade_pixel(e));
            continue;
        }

        diff_pixels += 1;
        let x = (idx as u32) % expected_img.width;
        let y = (idx as u32) / expected_img.width;
        bbox = Some(expand_bbox(bbox, x, y));
        diff_rgb.extend_from_slice(&[255, 0, 255]);
    }

    let diff_png = if diff_pixels == 0 {
        None
    } else {
        let diff_png = diff_path_for_actual(actual)?;
        let diff_image = one_agent_one_browser::image::RgbImage::new(
            expected_img.width,
            expected_img.height,
            diff_rgb,
        )?;
        one_agent_one_browser::png::write_rgb_png(&diff_png, &diff_image)?;
        Some(diff_png)
    };

    Ok(PngDiff {
        diff_pixels,
        total_pixels,
        bbox,
        diff_png,
    })
}

fn diff_path_for_actual(actual_png: &Path) -> Result<PathBuf, String> {
    let file_name = actual_png
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("Invalid PNG path: {}", actual_png.display()))?;
    let diff_name = file_name
        .strip_suffix(".actual.png")
        .map(|stem| format!("{stem}.diff.png"))
        .unwrap_or_else(|| format!("{file_name}.diff.png"));
    Ok(actual_png.with_file_name(diff_name))
}

fn fade_pixel(pixel: &[u8]) -> [u8; 3] {
    let [r, g, b] = pixel else {
        return [255, 255, 255];
    };
    [fade_channel(*r), fade_channel(*g), fade_channel(*b)]
}

fn fade_channel(value: u8) -> u8 {
    (((value as u16) * 3 + 255) / 4) as u8
}

fn expand_bbox(current: Option<PixelBbox>, x: u32, y: u32) -> PixelBbox {
    match current {
        None => PixelBbox {
            min_x: x,
            max_x: x,
            min_y: y,
            max_y: y,
        },
        Some(bbox) => PixelBbox {
            min_x: bbox.min_x.min(x),
            max_x: bbox.max_x.max(x),
            min_y: bbox.min_y.min(y),
            max_y: bbox.max_y.max(y),
        },
    }
}

struct PngImage {
    width: u32,
    height: u32,
    rgb: Vec<u8>,
}

fn read_png_rgb(path: &Path) -> Result<PngImage, String> {
    const SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    let bytes =
        std::fs::read(path).map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    let signature = bytes
        .get(0..8)
        .ok_or_else(|| format!("{} is too small to be a PNG", path.display()))?;
    if signature != SIGNATURE {
        return Err(format!(
            "{} is not a PNG (invalid signature)",
            path.display()
        ));
    }

    let mut offset = 8usize;
    let mut width: Option<u32> = None;
    let mut height: Option<u32> = None;
    let mut idat = Vec::<u8>::new();

    while offset + 12 <= bytes.len() {
        let chunk_len = read_u32_be(&bytes[offset..offset + 4])? as usize;
        let chunk_type = bytes[offset + 4..offset + 8].to_owned();
        offset += 8;

        let data_end = offset
            .checked_add(chunk_len)
            .ok_or_else(|| "PNG chunk length overflow".to_owned())?;
        let crc_end = data_end
            .checked_add(4)
            .ok_or_else(|| "PNG chunk length overflow".to_owned())?;
        if crc_end > bytes.len() {
            return Err(format!("{} has a truncated PNG chunk", path.display()));
        }

        let chunk_data = &bytes[offset..data_end];
        offset = crc_end;

        match chunk_type.as_slice() {
            b"IHDR" => {
                if chunk_len != 13 {
                    return Err(format!("{} has an invalid IHDR length", path.display()));
                }
                width = Some(read_u32_be(&chunk_data[0..4])?);
                height = Some(read_u32_be(&chunk_data[4..8])?);
                let bit_depth = chunk_data[8];
                let color_type = chunk_data[9];
                let compression = chunk_data[10];
                let filter = chunk_data[11];
                let interlace = chunk_data[12];

                if bit_depth != 8 || color_type != 2 {
                    return Err(format!(
                        "{} uses an unsupported PNG format (bit_depth={bit_depth}, color_type={color_type})",
                        path.display()
                    ));
                }
                if compression != 0 || filter != 0 || interlace != 0 {
                    return Err(format!(
                        "{} uses unsupported PNG parameters (compression={compression}, filter={filter}, interlace={interlace})",
                        path.display()
                    ));
                }
            }
            b"IDAT" => idat.extend_from_slice(chunk_data),
            b"IEND" => break,
            _ => {}
        }
    }

    let width = width.ok_or_else(|| format!("{} is missing IHDR", path.display()))?;
    let height = height.ok_or_else(|| format!("{} is missing IHDR", path.display()))?;
    if idat.is_empty() {
        return Err(format!("{} is missing IDAT", path.display()));
    }

    let decoded = zlib_decompress_stored(&idat)?;
    let row_bytes = width
        .checked_mul(3)
        .ok_or_else(|| "PNG row size overflow".to_owned())? as usize;
    let row_total = row_bytes
        .checked_add(1)
        .ok_or_else(|| "PNG row size overflow".to_owned())?;
    let expected_len = (height as usize)
        .checked_mul(row_total)
        .ok_or_else(|| "PNG image size overflow".to_owned())?;
    if decoded.len() != expected_len {
        return Err(format!(
            "{} decoded to an unexpected size: got {} bytes, expected {expected_len}",
            path.display(),
            decoded.len()
        ));
    }

    let mut rgb = Vec::with_capacity(row_bytes * height as usize);
    for row in 0..height as usize {
        let start = row * row_total;
        let filter_type = decoded[start];
        if filter_type != 0 {
            return Err(format!(
                "{} uses unsupported PNG filtering (filter_type={filter_type})",
                path.display()
            ));
        }
        rgb.extend_from_slice(&decoded[start + 1..start + 1 + row_bytes]);
    }

    Ok(PngImage { width, height, rgb })
}

fn read_u32_be(bytes: &[u8]) -> Result<u32, String> {
    let [a, b, c, d] = bytes else {
        return Err("Invalid u32 slice".to_owned());
    };
    Ok(u32::from_be_bytes([*a, *b, *c, *d]))
}

fn zlib_decompress_stored(zlib: &[u8]) -> Result<Vec<u8>, String> {
    if zlib.len() < 2 + 5 + 4 {
        return Err("Zlib stream is too small".to_owned());
    }

    let mut offset = 2usize;
    let mut out = Vec::new();

    loop {
        let header = *zlib
            .get(offset)
            .ok_or_else(|| "Truncated DEFLATE header".to_owned())?;
        offset += 1;

        let is_final = header & 1 == 1;
        let btype = (header >> 1) & 0b11;
        if btype != 0 {
            return Err("Unsupported DEFLATE block type (expected stored blocks)".to_owned());
        }

        let len = read_u16_le(zlib.get(offset..offset + 2))? as usize;
        let nlen = read_u16_le(zlib.get(offset + 2..offset + 4))?;
        offset += 4;

        if nlen != (!len as u16) {
            return Err("Invalid stored DEFLATE block (LEN/NLEN mismatch)".to_owned());
        }

        let end = offset
            .checked_add(len)
            .ok_or_else(|| "DEFLATE block length overflow".to_owned())?;
        let chunk = zlib
            .get(offset..end)
            .ok_or_else(|| "Truncated DEFLATE block".to_owned())?;
        out.extend_from_slice(chunk);
        offset = end;

        if is_final {
            break;
        }
    }

    Ok(out)
}

fn read_u16_le(bytes: Option<&[u8]>) -> Result<u16, String> {
    let bytes = bytes.ok_or_else(|| "Truncated u16".to_owned())?;
    let [a, b] = bytes else {
        return Err("Invalid u16 slice".to_owned());
    };
    Ok(u16::from_le_bytes([*a, *b]))
}

#[cfg(test)]
mod tests {
    #[cfg(target_os = "linux")]
    use super::linux_baseline_platform_tags;
    #[cfg(target_os = "linux")]
    use std::ffi::OsStr;

    #[cfg(target_os = "linux")]
    #[test]
    fn linux_wayland_baselines_are_preferred_when_wayland_is_detected() {
        assert_eq!(
            linux_baseline_platform_tags(Some(OsStr::new("wayland-0")), None),
            vec!["linux-wayland", "linux-x11"]
        );
        assert_eq!(
            linux_baseline_platform_tags(None, Some("wayland")),
            vec!["linux-wayland", "linux-x11"]
        );
        assert_eq!(
            linux_baseline_platform_tags(None, Some("x11")),
            vec!["linux-x11", "linux-wayland"]
        );
    }
}
