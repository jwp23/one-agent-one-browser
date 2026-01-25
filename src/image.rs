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
    Err("Unsupported image format".to_owned())
}

fn looks_like_webp(data: &[u8]) -> bool {
    if data.len() < 12 {
        return false;
    }
    &data[..4] == b"RIFF" && &data[8..12] == b"WEBP"
}

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
    let mut argb32 = vec![0u8; len];

    for (src, dst) in rgba.chunks_exact(4).zip(argb32.chunks_exact_mut(4)) {
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

    Argb32Image::new(width_u32, height_u32, argb32)
}
