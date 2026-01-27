#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RgbImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl RgbImage {
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Result<Self, String> {
        let expected_len = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(3))
            .ok_or_else(|| "Image size overflow".to_owned())? as usize;

        if data.len() != expected_len {
            return Err(format!(
                "Invalid RGB image buffer length: expected {expected_len} bytes, got {}",
                data.len()
            ));
        }

        Ok(Self {
            width,
            height,
            data,
        })
    }

    pub fn row_stride_bytes(&self) -> usize {
        self.width as usize * 3
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Argb32Image {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl Argb32Image {
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Result<Self, String> {
        let expected_len = width
            .checked_mul(height)
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or_else(|| "Image size overflow".to_owned())? as usize;

        if data.len() != expected_len {
            return Err(format!(
                "Invalid ARGB32 image buffer length: expected {expected_len} bytes, got {}",
                data.len()
            ));
        }

        Ok(Self {
            width,
            height,
            data,
        })
    }

    pub fn row_stride_bytes(&self) -> usize {
        self.width as usize * 4
    }
}

pub fn decode_image(data: &[u8]) -> Result<Argb32Image, String> {
    if looks_like_webp(data) {
        return decode_webp_argb32(data);
    }
    if looks_like_png(data) {
        return decode_png_argb32(data);
    }
    if looks_like_jpeg(data) {
        return decode_jpeg_argb32(data);
    }
    Err("Unsupported image format".to_owned())
}

pub fn looks_like_supported_image(data: &[u8]) -> bool {
    looks_like_webp(data) || looks_like_png(data) || looks_like_jpeg(data)
}

fn looks_like_webp(data: &[u8]) -> bool {
    if data.len() < 12 {
        return false;
    }
    &data[..4] == b"RIFF" && &data[8..12] == b"WEBP"
}

fn looks_like_png(data: &[u8]) -> bool {
    const SIG: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
    data.len() >= SIG.len() && data[..SIG.len()] == SIG
}

#[cfg(target_os = "macos")]
fn decode_png_argb32(data: &[u8]) -> Result<Argb32Image, String> {
    decode_imageio_argb32(data)
}

#[cfg(target_os = "windows")]
fn decode_png_argb32(data: &[u8]) -> Result<Argb32Image, String> {
    crate::win::wic::decode_png_argb32(data)
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn decode_png_argb32(data: &[u8]) -> Result<Argb32Image, String> {
    use core::ffi::{c_int, c_void};

    #[repr(C)]
    struct PngImage {
        opaque: *mut c_void,
        version: u32,
        width: u32,
        height: u32,
        format: u32,
        flags: u32,
        colormap_entries: u32,
        warning_or_error: u32,
        message: [u8; 64],
    }

    const PNG_IMAGE_VERSION: u32 = 1;
    const PNG_FORMAT_RGBA: u32 = 0x03;

    #[link(name = "png16")]
    unsafe extern "C" {
        fn png_image_begin_read_from_memory(
            image: *mut PngImage,
            memory: *const c_void,
            size: usize,
        ) -> c_int;
        fn png_image_finish_read(
            image: *mut PngImage,
            background: *const c_void,
            buffer: *mut c_void,
            row_stride: c_int,
            colormap: *const c_void,
        ) -> c_int;
        fn png_image_free(image: *mut PngImage);
    }

    struct PngImageGuard {
        image: PngImage,
    }

    impl Drop for PngImageGuard {
        fn drop(&mut self) {
            unsafe { png_image_free(&mut self.image) };
        }
    }

    fn error_message(prefix: &str, image: &PngImage) -> String {
        let end = image
            .message
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(image.message.len());
        let message = std::str::from_utf8(&image.message[..end]).unwrap_or("").trim();
        if message.is_empty() {
            prefix.to_owned()
        } else {
            format!("{prefix}: {message}")
        }
    }

    let mut guard = PngImageGuard {
        image: PngImage {
            opaque: std::ptr::null_mut(),
            version: PNG_IMAGE_VERSION,
            width: 0,
            height: 0,
            format: 0,
            flags: 0,
            colormap_entries: 0,
            warning_or_error: 0,
            message: [0u8; 64],
        },
    };

    let ok = unsafe {
        png_image_begin_read_from_memory(
            &mut guard.image,
            data.as_ptr().cast::<c_void>(),
            data.len(),
        )
    };
    if ok == 0 {
        return Err(error_message("png_image_begin_read_from_memory failed", &guard.image));
    }

    let width = guard.image.width;
    let height = guard.image.height;
    if width == 0 || height == 0 {
        return Err("Invalid PNG dimensions".to_owned());
    }

    guard.image.format = PNG_FORMAT_RGBA;

    let len = width
        .checked_mul(height)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "PNG image size overflow".to_owned())? as usize;

    let mut rgba = vec![0u8; len];
    let ok = unsafe {
        png_image_finish_read(
            &mut guard.image,
            std::ptr::null(),
            rgba.as_mut_ptr().cast::<c_void>(),
            0,
            std::ptr::null(),
        )
    };
    if ok == 0 {
        return Err(error_message("png_image_finish_read failed", &guard.image));
    }

    let argb32 = premultiply_rgba_to_bgra(&rgba);
    Argb32Image::new(width, height, argb32)
}

fn looks_like_jpeg(data: &[u8]) -> bool {
    data.len() >= 2 && data[0] == 0xff && data[1] == 0xd8
}

#[cfg(target_os = "macos")]
fn decode_jpeg_argb32(data: &[u8]) -> Result<Argb32Image, String> {
    decode_imageio_argb32(data)
}

#[cfg(target_os = "windows")]
fn decode_jpeg_argb32(data: &[u8]) -> Result<Argb32Image, String> {
    crate::win::wic::decode_jpeg_argb32(data)
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn decode_jpeg_argb32(data: &[u8]) -> Result<Argb32Image, String> {
    use core::ffi::{c_char, c_int, c_ulong, c_void};

    type TjHandle = *mut c_void;

    const TJPF_BGRA: c_int = 8;

    #[link(name = "turbojpeg")]
    unsafe extern "C" {
        fn tjInitDecompress() -> TjHandle;
        fn tjDestroy(handle: TjHandle) -> c_int;
        fn tjGetErrorStr2(handle: TjHandle) -> *const c_char;
        fn tjDecompressHeader3(
            handle: TjHandle,
            jpeg_buf: *const u8,
            jpeg_size: c_ulong,
            width: *mut c_int,
            height: *mut c_int,
            jpeg_subsamp: *mut c_int,
            jpeg_colorspace: *mut c_int,
        ) -> c_int;
        fn tjDecompress2(
            handle: TjHandle,
            jpeg_buf: *const u8,
            jpeg_size: c_ulong,
            dst_buf: *mut u8,
            width: c_int,
            pitch: c_int,
            height: c_int,
            pixel_format: c_int,
            flags: c_int,
        ) -> c_int;
    }

    struct TjGuard(TjHandle);

    impl Drop for TjGuard {
        fn drop(&mut self) {
            if self.0.is_null() {
                return;
            }
            unsafe {
                let _ = tjDestroy(self.0);
            }
        }
    }

    fn tj_error(handle: TjHandle, prefix: &str) -> String {
        let ptr = unsafe { tjGetErrorStr2(handle) };
        if ptr.is_null() {
            return prefix.to_owned();
        }
        let message = unsafe { std::ffi::CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned();
        if message.is_empty() {
            prefix.to_owned()
        } else {
            format!("{prefix}: {message}")
        }
    }

    let handle = unsafe { tjInitDecompress() };
    if handle.is_null() {
        return Err("tjInitDecompress failed".to_owned());
    }
    let _guard = TjGuard(handle);

    let mut width: c_int = 0;
    let mut height: c_int = 0;
    let mut subsamp: c_int = 0;
    let mut colorspace: c_int = 0;
    let rc = unsafe {
        tjDecompressHeader3(
            handle,
            data.as_ptr(),
            data.len() as c_ulong,
            &mut width,
            &mut height,
            &mut subsamp,
            &mut colorspace,
        )
    };
    if rc != 0 {
        return Err(tj_error(handle, "tjDecompressHeader3 failed"));
    }
    if width <= 0 || height <= 0 {
        return Err(format!("Invalid JPEG dimensions: {width}x{height}"));
    }

    let width_u32: u32 = width
        .try_into()
        .map_err(|_| format!("Invalid JPEG width: {width}"))?;
    let height_u32: u32 = height
        .try_into()
        .map_err(|_| format!("Invalid JPEG height: {height}"))?;

    let len = width_u32
        .checked_mul(height_u32)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "JPEG image size overflow".to_owned())? as usize;

    let mut bgra = vec![0u8; len];
    let rc = unsafe {
        tjDecompress2(
            handle,
            data.as_ptr(),
            data.len() as c_ulong,
            bgra.as_mut_ptr(),
            width,
            0,
            height,
            TJPF_BGRA,
            0,
        )
    };
    if rc != 0 {
        return Err(tj_error(handle, "tjDecompress2 failed"));
    }

    Argb32Image::new(width_u32, height_u32, bgra)
}

#[cfg(target_os = "macos")]
fn decode_webp_argb32(data: &[u8]) -> Result<Argb32Image, String> {
    decode_imageio_argb32(data)
}

#[cfg(target_os = "windows")]
fn decode_webp_argb32(data: &[u8]) -> Result<Argb32Image, String> {
    crate::win::wic::decode_webp_argb32(data)
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn decode_webp_argb32(data: &[u8]) -> Result<Argb32Image, String> {
    use core::ffi::{c_int, c_void};

    #[link(name = "webp")]
    unsafe extern "C" {
        fn WebPDecodeRGBA(
            data: *const u8,
            data_size: usize,
            width: *mut c_int,
            height: *mut c_int,
        ) -> *mut u8;
        fn WebPFree(ptr: *mut c_void);
    }

    struct WebPBuffer(*mut u8);

    impl Drop for WebPBuffer {
        fn drop(&mut self) {
            if self.0.is_null() {
                return;
            }
            unsafe { WebPFree(self.0 as *mut c_void) };
        }
    }

    let mut width: c_int = 0;
    let mut height: c_int = 0;
    let ptr = unsafe { WebPDecodeRGBA(data.as_ptr(), data.len(), &mut width, &mut height) };
    if ptr.is_null() {
        return Err("WebPDecodeRGBA failed".to_owned());
    }
    let buf = WebPBuffer(ptr);

    let width_u32: u32 = width
        .try_into()
        .map_err(|_| format!("Invalid WebP width: {width}"))?;
    let height_u32: u32 = height
        .try_into()
        .map_err(|_| format!("Invalid WebP height: {height}"))?;

    let len = width_u32
        .checked_mul(height_u32)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "WebP image size overflow".to_owned())? as usize;

    let rgba = unsafe { std::slice::from_raw_parts(buf.0, len) };
    Argb32Image::new(width_u32, height_u32, premultiply_rgba_to_bgra(rgba))
}

#[cfg(target_os = "macos")]
fn decode_imageio_argb32(data: &[u8]) -> Result<Argb32Image, String> {
    use core::ffi::{c_int, c_uchar, c_uint, c_void};

    type CFIndex = isize;
    type CFAllocatorRef = *const c_void;
    type CFDataRef = *const c_void;
    type CFDictionaryRef = *const c_void;
    type CFTypeRef = *const c_void;
    type CGContextRef = *mut c_void;
    type CGColorSpaceRef = *mut c_void;
    type CGImageRef = *mut c_void;
    type CGImageSourceRef = *mut c_void;

    type CGFloat = f64;
    type SizeT = usize;

    #[repr(C)]
    struct CGPoint {
        x: CGFloat,
        y: CGFloat,
    }

    #[repr(C)]
    struct CGSize {
        width: CGFloat,
        height: CGFloat,
    }

    #[repr(C)]
    struct CGRect {
        origin: CGPoint,
        size: CGSize,
    }

    const K_CGIMAGE_ALPHA_PREMULTIPLIED_FIRST: c_uint = 2;
    const K_CGBITMAP_BYTEORDER32LITTLE: c_uint = 2 << 12;
    const BITMAP_INFO_BGRA_PREMULTIPLIED: c_uint =
        K_CGIMAGE_ALPHA_PREMULTIPLIED_FIRST | K_CGBITMAP_BYTEORDER32LITTLE;

    const BLEND_MODE_COPY: c_int = 0;

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFDataCreate(
            allocator: CFAllocatorRef,
            bytes: *const c_uchar,
            length: CFIndex,
        ) -> CFDataRef;
        fn CFRelease(cf: CFTypeRef);
    }

    #[link(name = "ImageIO", kind = "framework")]
    unsafe extern "C" {
        fn CGImageSourceCreateWithData(
            data: CFDataRef,
            options: CFDictionaryRef,
        ) -> CGImageSourceRef;
        fn CGImageSourceCreateImageAtIndex(
            source: CGImageSourceRef,
            index: SizeT,
            options: CFDictionaryRef,
        ) -> CGImageRef;
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGColorSpaceCreateDeviceRGB() -> CGColorSpaceRef;
        fn CGColorSpaceRelease(space: CGColorSpaceRef);

        fn CGBitmapContextCreate(
            data: *mut c_void,
            width: SizeT,
            height: SizeT,
            bits_per_component: SizeT,
            bytes_per_row: SizeT,
            space: CGColorSpaceRef,
            bitmap_info: c_uint,
        ) -> CGContextRef;
        fn CGContextRelease(c: CGContextRef);

        fn CGContextSetBlendMode(c: CGContextRef, mode: c_int);
        fn CGContextDrawImage(c: CGContextRef, rect: CGRect, image: CGImageRef);

        fn CGImageGetWidth(image: CGImageRef) -> SizeT;
        fn CGImageGetHeight(image: CGImageRef) -> SizeT;
        fn CGImageRelease(image: CGImageRef);
    }

    let len: CFIndex = data
        .len()
        .try_into()
        .map_err(|_| "Image buffer too large".to_owned())?;
    let cf_data = unsafe { CFDataCreate(std::ptr::null(), data.as_ptr(), len) };
    if cf_data.is_null() {
        return Err("CFDataCreate failed".to_owned());
    }

    let source = unsafe { CGImageSourceCreateWithData(cf_data, std::ptr::null()) };
    unsafe { CFRelease(cf_data) };
    if source.is_null() {
        return Err("CGImageSourceCreateWithData failed".to_owned());
    }

    let image = unsafe { CGImageSourceCreateImageAtIndex(source, 0, std::ptr::null()) };
    unsafe { CFRelease(source as CFTypeRef) };
    if image.is_null() {
        return Err("CGImageSourceCreateImageAtIndex failed".to_owned());
    }

    let width: u32 = unsafe { CGImageGetWidth(image) }
        .try_into()
        .map_err(|_| "Invalid decoded image width".to_owned())?;
    let height: u32 = unsafe { CGImageGetHeight(image) }
        .try_into()
        .map_err(|_| "Invalid decoded image height".to_owned())?;
    if width == 0 || height == 0 {
        unsafe { CGImageRelease(image) };
        return Err("Decoded image has invalid dimensions".to_owned());
    }

    let bytes_per_row = width
        .checked_mul(4)
        .ok_or_else(|| "Decoded image row stride overflow".to_owned())? as usize;
    let expected_len = (height as usize)
        .checked_mul(bytes_per_row)
        .ok_or_else(|| "Decoded image buffer overflow".to_owned())?;
    let mut bgra = vec![0u8; expected_len];

    let color_space = unsafe { CGColorSpaceCreateDeviceRGB() };
    if color_space.is_null() {
        unsafe { CGImageRelease(image) };
        return Err("CGColorSpaceCreateDeviceRGB failed".to_owned());
    }

    let ctx = unsafe {
        CGBitmapContextCreate(
            bgra.as_mut_ptr().cast::<c_void>(),
            width as usize,
            height as usize,
            8,
            bytes_per_row,
            color_space,
            BITMAP_INFO_BGRA_PREMULTIPLIED,
        )
    };
    unsafe { CGColorSpaceRelease(color_space) };
    if ctx.is_null() {
        unsafe { CGImageRelease(image) };
        return Err("CGBitmapContextCreate failed".to_owned());
    }

    unsafe {
        CGContextSetBlendMode(ctx, BLEND_MODE_COPY);
        CGContextDrawImage(
            ctx,
            CGRect {
                origin: CGPoint { x: 0.0, y: 0.0 },
                size: CGSize {
                    width: width as CGFloat,
                    height: height as CGFloat,
                },
            },
            image,
        );
        CGContextRelease(ctx);
        CGImageRelease(image);
    }

    Argb32Image::new(width, height, bgra)
}

#[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
fn premultiply_rgba_to_bgra(rgba: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; rgba.len()];
    for (src, dst) in rgba.chunks_exact(4).zip(out.chunks_exact_mut(4)) {
        let r = src[0] as u16;
        let g = src[1] as u16;
        let b = src[2] as u16;
        let a = src[3] as u16;

        let premul = |channel: u16| -> u8 {
            ((channel.saturating_mul(a).saturating_add(127)) / 255)
                .min(255) as u8
        };

        let a8 = a.min(255) as u8;
        dst[0] = premul(b);
        dst[1] = premul(g);
        dst[2] = premul(r);
        dst[3] = a8;
    }
    out
}
