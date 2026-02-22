use core::ffi::{c_double, c_void};
use std::borrow::Cow;
use std::fs::OpenOptions;
use std::io::Write as _;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

type CGFloat = c_double;
type CFIndex = isize;
type CFAllocatorRef = *const c_void;
type CFTypeRef = *const c_void;
type CFURLRef = *const c_void;
type CGImageRef = *mut c_void;

#[repr(C)]
struct CGSize {
    width: CGFloat,
    height: CGFloat,
}

#[allow(non_upper_case_globals)]
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(cf: CFTypeRef);

    fn CFURLCreateFromFileSystemRepresentation(
        allocator: CFAllocatorRef,
        buffer: *const u8,
        buf_len: CFIndex,
        is_directory: u8,
    ) -> CFURLRef;
}

#[allow(non_upper_case_globals)]
#[link(name = "QuickLook", kind = "framework")]
unsafe extern "C" {
    fn QLThumbnailImageCreate(
        allocator: CFAllocatorRef,
        url: CFURLRef,
        max_thumbnail_size: CGSize,
        options: *const c_void,
    ) -> CGImageRef;
}

pub(super) fn rasterize_svg_to_cgimage(
    svg_xml: &str,
    width_px: i32,
    height_px: i32,
) -> Result<CGImageRef, String> {
    if width_px <= 0 || height_px <= 0 {
        return Err("Invalid SVG raster size".to_owned());
    }

    let svg_xml = svg_xml.trim();
    if svg_xml.is_empty() {
        return Err("SVG XML was empty".to_owned());
    }

    let svg_xml = ensure_xlink_namespace(svg_xml);
    rasterize_svg_via_quicklook(svg_xml.as_ref().as_bytes(), width_px, height_px)
}

fn rasterize_svg_via_quicklook(
    svg_bytes: &[u8],
    width_px: i32,
    height_px: i32,
) -> Result<CGImageRef, String> {
    let temp_file = TempSvgFile::create(svg_bytes)?;
    let url = create_file_url(&temp_file.path)?;

    let max_size = CGSize {
        width: width_px as CGFloat,
        height: height_px as CGFloat,
    };

    let image =
        unsafe { QLThumbnailImageCreate(std::ptr::null(), url, max_size, std::ptr::null()) };
    unsafe { CFRelease(url as CFTypeRef) };

    if image.is_null() {
        return Err("QLThumbnailImageCreate failed".to_owned());
    }

    Ok(image)
}

fn create_file_url(path: &PathBuf) -> Result<CFURLRef, String> {
    use std::os::unix::ffi::OsStrExt as _;

    let bytes = path.as_os_str().as_bytes();
    let len: CFIndex = bytes
        .len()
        .try_into()
        .map_err(|_| "Temp path too long".to_owned())?;
    let url = unsafe {
        CFURLCreateFromFileSystemRepresentation(std::ptr::null(), bytes.as_ptr(), len, 0)
    };
    if url.is_null() {
        return Err("CFURLCreateFromFileSystemRepresentation failed".to_owned());
    }
    Ok(url)
}

struct TempSvgFile {
    path: PathBuf,
}

impl TempSvgFile {
    fn create(svg_bytes: &[u8]) -> Result<Self, String> {
        let temp_dir = std::env::temp_dir();
        let pid = std::process::id();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| "System clock not available".to_owned())?
            .as_nanos();

        for attempt in 0..100u32 {
            let filename = format!("one-agent-one-browser-svg-{pid}-{nanos}-{attempt}.svg");
            let path = temp_dir.join(filename);

            let mut file = match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => file,
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(err) => return Err(format!("Failed to create temp SVG file: {err}")),
            };

            file.write_all(svg_bytes)
                .and_then(|_| file.flush())
                .map_err(|err| format!("Failed to write temp SVG file: {err}"))?;

            return Ok(Self { path });
        }

        Err("Failed to create unique temp SVG file".to_owned())
    }
}

impl Drop for TempSvgFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn ensure_xlink_namespace(svg_xml: &str) -> Cow<'_, str> {
    if !svg_xml.contains("xlink:") || svg_xml.contains("xmlns:xlink") {
        return Cow::Borrowed(svg_xml);
    }

    let Some(svg_start) = svg_xml.find("<svg") else {
        return Cow::Borrowed(svg_xml);
    };

    let Some(svg_end) = find_tag_end(svg_xml, svg_start) else {
        return Cow::Borrowed(svg_xml);
    };

    let insert_at = start_tag_insert_pos(svg_xml, svg_start, svg_end);
    let injection = r#" xmlns:xlink="http://www.w3.org/1999/xlink""#;

    let mut out = String::with_capacity(svg_xml.len() + injection.len());
    out.push_str(&svg_xml[..insert_at]);
    out.push_str(injection);
    out.push_str(&svg_xml[insert_at..]);
    Cow::Owned(out)
}

fn find_tag_end(input: &str, start: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut idx = start;
    let mut quote: Option<u8> = None;

    while idx < bytes.len() {
        let b = bytes[idx];
        if let Some(q) = quote {
            if b == q {
                quote = None;
            }
            idx += 1;
            continue;
        }

        match b {
            b'"' | b'\'' => quote = Some(b),
            b'>' => return Some(idx),
            _ => {}
        }
        idx += 1;
    }

    None
}

fn start_tag_insert_pos(input: &str, start: usize, end: usize) -> usize {
    debug_assert!(start <= end);

    let bytes = input.as_bytes();
    let mut idx = end;
    while idx > start && bytes[idx.saturating_sub(1)].is_ascii_whitespace() {
        idx = idx.saturating_sub(1);
    }

    if idx > start && bytes[idx.saturating_sub(1)] == b'/' {
        idx.saturating_sub(1)
    } else {
        end
    }
}
