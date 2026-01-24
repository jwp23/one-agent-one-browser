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

