mod support;

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use support::http::{HttpTestServer, Route};

#[test]
fn screenshot_waits_for_async_images() {
    let red_png = solid_rgb_png_bytes(255, 0, 0);

    let html = br#"<!doctype html>
<meta charset="utf-8">
<style>
  body { margin: 0; background: #ffffff; }
</style>
<img src="/slow.png" width="40" height="40">
"#
    .to_vec();

    let server = HttpTestServer::new(vec![
        Route {
            path: "/index.html".to_owned(),
            status: 200,
            content_type: "text/html; charset=utf-8".to_owned(),
            body: html,
            delay: Duration::ZERO,
        },
        Route {
            path: "/slow.png".to_owned(),
            status: 200,
            content_type: "image/png".to_owned(),
            body: red_png,
            delay: Duration::from_secs(1),
        },
    ]);

    let browser_exe = PathBuf::from(env!("CARGO_BIN_EXE_one-agent-one-browser"));

    let screenshot_path = unique_temp_file_path("screenshot_waits_for_async_images", "png");
    let _ = std::fs::remove_file(&screenshot_path);

    run_browser_screenshot(
        &browser_exe,
        &server.url("/index.html"),
        64,
        64,
        &screenshot_path,
        Duration::from_secs(10),
    );

    assert_eq!(server.requests_for_path("/slow.png"), 1);

    let pixel = read_rgb_png_pixel(&screenshot_path, 20, 20).unwrap();
    assert!(
        pixel[0] > 200 && pixel[1] < 80 && pixel[2] < 80,
        "expected a red pixel from the downloaded image, got rgb={:?}",
        pixel
    );

    let _ = std::fs::remove_file(&screenshot_path);
    server.shutdown();
}

fn run_browser_screenshot(
    browser_exe: &Path,
    url: &str,
    width_px: i32,
    height_px: i32,
    screenshot_path: &Path,
    timeout: Duration,
) {
    let screenshot_arg = format!("--screenshot={}", screenshot_path.display());
    let mut child = Command::new(browser_exe)
        .env("OAB_SCALE", "1")
        .arg("--headless")
        .arg(format!("--width={width_px}"))
        .arg(format!("--height={height_px}"))
        .arg(url)
        .arg(screenshot_arg)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap_or_else(|err| panic!("Failed to start {}: {err}", browser_exe.display()));

    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().expect("Failed to poll browser process") {
            assert!(
                status.success(),
                "Browser process failed (exit={})",
                status.code().unwrap_or(-1)
            );
            assert!(
                screenshot_path.is_file(),
                "Browser process exited successfully but screenshot was not created: {}",
                screenshot_path.display()
            );
            return;
        }

        if started.elapsed() > timeout {
            let _ = child.kill();
            panic!("Browser render timed out after {timeout:?} for {url}");
        }

        std::thread::sleep(Duration::from_millis(20));
    }
}

fn solid_rgb_png_bytes(r: u8, g: u8, b: u8) -> Vec<u8> {
    let temp_path = unique_temp_file_path("solid_rgb_png_bytes", "png");
    let (width, height) = (40u32, 40u32);
    let mut data = Vec::with_capacity((width * height * 3) as usize);
    for _ in 0..width * height {
        data.extend_from_slice(&[r, g, b]);
    }
    let image = one_agent_one_browser::image::RgbImage::new(width, height, data).unwrap();
    one_agent_one_browser::png::write_rgb_png(&temp_path, &image).unwrap();
    let bytes = std::fs::read(&temp_path).unwrap();
    let _ = std::fs::remove_file(&temp_path);
    bytes
}

fn unique_temp_file_path(stem: &str, extension: &str) -> PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO);
    std::env::temp_dir().join(format!(
        "{stem}.{}.{}.{}",
        std::process::id(),
        now.as_secs(),
        extension
    ))
}

fn read_rgb_png_pixel(path: &Path, x: u32, y: u32) -> Result<[u8; 3], String> {
    let image = read_rgb_png(path)?;
    if x >= image.width || y >= image.height {
        return Err(format!(
            "Pixel out of bounds: ({x},{y}) for {}x{} image",
            image.width, image.height
        ));
    }
    let idx = ((y * image.width + x) as usize)
        .checked_mul(3)
        .ok_or_else(|| "Pixel offset overflow".to_owned())?;
    let rgb = image
        .rgb
        .get(idx..idx + 3)
        .ok_or_else(|| "Pixel slice out of bounds".to_owned())?;
    Ok([rgb[0], rgb[1], rgb[2]])
}

struct PngRgbImage {
    width: u32,
    height: u32,
    rgb: Vec<u8>,
}

fn read_rgb_png(path: &Path) -> Result<PngRgbImage, String> {
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

    let mut offset = SIGNATURE.len();
    let mut width: Option<u32> = None;
    let mut height: Option<u32> = None;
    let mut idat = Vec::<u8>::new();

    while offset + 8 <= bytes.len() {
        let len = u32::from_be_bytes(
            bytes
                .get(offset..offset + 4)
                .ok_or_else(|| "PNG chunk length out of bounds".to_owned())?
                .try_into()
                .map_err(|_| "PNG chunk length parse failed".to_owned())?,
        ) as usize;
        offset += 4;

        let chunk_type: [u8; 4] = bytes
            .get(offset..offset + 4)
            .ok_or_else(|| "PNG chunk type out of bounds".to_owned())?
            .try_into()
            .map_err(|_| "PNG chunk type parse failed".to_owned())?;
        offset += 4;

        let data = bytes
            .get(offset..offset + len)
            .ok_or_else(|| "PNG chunk data out of bounds".to_owned())?;
        offset += len;

        let _crc = bytes
            .get(offset..offset + 4)
            .ok_or_else(|| "PNG chunk CRC out of bounds".to_owned())?;
        offset += 4;

        match &chunk_type {
            b"IHDR" => {
                if len != 13 {
                    return Err("Invalid IHDR chunk length".to_owned());
                }
                width = Some(u32::from_be_bytes(
                    data.get(0..4)
                        .ok_or_else(|| "IHDR width missing".to_owned())?
                        .try_into()
                        .map_err(|_| "IHDR width parse failed".to_owned())?,
                ));
                height = Some(u32::from_be_bytes(
                    data.get(4..8)
                        .ok_or_else(|| "IHDR height missing".to_owned())?
                        .try_into()
                        .map_err(|_| "IHDR height parse failed".to_owned())?,
                ));
            }
            b"IDAT" => idat.extend_from_slice(data),
            b"IEND" => break,
            _ => {}
        }
    }

    let width = width.ok_or_else(|| "PNG missing IHDR width".to_owned())?;
    let height = height.ok_or_else(|| "PNG missing IHDR height".to_owned())?;
    if width == 0 || height == 0 {
        return Err("Invalid PNG dimensions".to_owned());
    }

    let scanlines = zlib_inflate_stored(&idat)?;

    let row_len = (width as usize)
        .checked_mul(3)
        .and_then(|len| len.checked_add(1))
        .ok_or_else(|| "PNG row size overflow".to_owned())?;
    let expected_scanlines_len = (height as usize)
        .checked_mul(row_len)
        .ok_or_else(|| "PNG image size overflow".to_owned())?;
    if scanlines.len() != expected_scanlines_len {
        return Err(format!(
            "Unexpected PNG scanline length: expected {expected_scanlines_len}, got {}",
            scanlines.len()
        ));
    }

    let mut rgb = Vec::with_capacity((width as usize) * (height as usize) * 3);
    for row in 0..height as usize {
        let start = row * row_len;
        let filter = scanlines[start];
        if filter != 0 {
            return Err(format!("Unsupported PNG filter type: {filter}"));
        }
        rgb.extend_from_slice(&scanlines[start + 1..start + row_len]);
    }

    Ok(PngRgbImage { width, height, rgb })
}

fn zlib_inflate_stored(compressed: &[u8]) -> Result<Vec<u8>, String> {
    if compressed.len() < 2 {
        return Err("Zlib stream too small".to_owned());
    }
    if compressed[0] != 0x78 || compressed[1] != 0x01 {
        return Err("Unexpected zlib header (expected 0x78 0x01)".to_owned());
    }

    let mut out = Vec::new();
    let mut offset = 2usize;
    loop {
        if offset >= compressed.len() {
            return Err("Unexpected end of zlib stream".to_owned());
        }

        let header = compressed[offset];
        offset += 1;
        let is_final = (header & 1) != 0;
        let block_type = (header >> 1) & 0b11;
        if block_type != 0 {
            return Err(format!("Unsupported zlib block type: {block_type}"));
        }

        if offset + 4 > compressed.len() {
            return Err("Truncated zlib stored block header".to_owned());
        }

        let len = u16::from_le_bytes(
            compressed[offset..offset + 2]
                .try_into()
                .map_err(|_| "LEN parse failed".to_owned())?,
        );
        let nlen = u16::from_le_bytes(
            compressed[offset + 2..offset + 4]
                .try_into()
                .map_err(|_| "NLEN parse failed".to_owned())?,
        );
        offset += 4;

        if len ^ nlen != 0xFFFF {
            return Err("Invalid zlib stored block LEN/NLEN".to_owned());
        }

        let len_usize = len as usize;
        if offset + len_usize > compressed.len() {
            return Err("Truncated zlib stored block data".to_owned());
        }
        out.extend_from_slice(&compressed[offset..offset + len_usize]);
        offset += len_usize;

        if is_final {
            break;
        }
    }

    if compressed.len().saturating_sub(offset) < 4 {
        return Err("Missing zlib Adler32 checksum".to_owned());
    }

    Ok(out)
}
