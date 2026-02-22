const SCALE_ONE_1024: u32 = 1024;
const MIN_SCALE_1024: u32 = 256; // 0.25x
const MAX_SCALE_1024: u32 = 8192; // 8.0x

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ScaleFactor {
    scale_1024: u32,
}

impl ScaleFactor {
    pub fn detect() -> Self {
        if let Some(scale) = scale_from_env() {
            return Self::new(scale);
        }
        Self::new(SCALE_ONE_1024)
    }

    pub fn new(scale_1024: u32) -> Self {
        let mut scale_1024 = scale_1024.clamp(MIN_SCALE_1024, MAX_SCALE_1024);
        if scale_1024 == 0 {
            scale_1024 = SCALE_ONE_1024;
        }
        Self { scale_1024 }
    }

    pub fn scale_int(self) -> i32 {
        ((self.scale_1024 + 512) / 1024) as i32
    }

    pub fn css_size_to_device_px(self, css_px: i32) -> i32 {
        let css_px = i64::from(css_px);
        let scaled = mul_div_round_nearest(css_px, i64::from(self.scale_1024), 1024);
        clamp_i64_to_i32(scaled.max(1))
    }

    pub fn css_coord_to_device_px(self, css_px: i32) -> i32 {
        let css_px = i64::from(css_px);
        let scaled = mul_div_round_nearest(css_px, i64::from(self.scale_1024), 1024);
        clamp_i64_to_i32(scaled)
    }

    pub fn css_span_to_device_px(self, start_css_px: i32, span_css_px: i32) -> (i32, i32) {
        if span_css_px <= 0 {
            return (0, 0);
        }
        let start = i64::from(start_css_px);
        let end = start.saturating_add(i64::from(span_css_px));
        let scale = i64::from(self.scale_1024);
        let start_dev = mul_div_round_nearest(start, scale, 1024);
        let end_dev = mul_div_round_nearest(end, scale, 1024);
        let span_dev = end_dev.saturating_sub(start_dev);
        (clamp_i64_to_i32(start_dev), clamp_i64_to_i32(span_dev))
    }

    pub fn device_delta_to_css_px(self, device_px: i32) -> i32 {
        if self.scale_1024 == SCALE_ONE_1024 {
            return device_px;
        }
        let device_px = i64::from(device_px);
        let denom = i64::from(self.scale_1024);
        let css = mul_div_round_nearest(device_px, 1024, denom);
        clamp_i64_to_i32(css)
    }
}

fn scale_from_env() -> Option<u32> {
    let value = std::env::var("OAB_SCALE").ok()?;
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let scale_f32 = if let Some(percent) = value.strip_suffix('%') {
        let n: f32 = percent.trim().parse().ok()?;
        n / 100.0
    } else {
        value.parse().ok()?
    };
    if !scale_f32.is_finite() || scale_f32 <= 0.0 {
        return None;
    }
    let scale_1024 = (scale_f32 * 1024.0).round() as i64;
    if scale_1024 <= 0 {
        return None;
    }
    Some(clamp_i64_to_u32(scale_1024))
}

fn mul_div_round_nearest(value: i64, num: i64, den: i64) -> i64 {
    if den == 0 {
        return 0;
    }
    let product = value.saturating_mul(num);
    if product >= 0 {
        (product + den / 2) / den
    } else {
        (product - den / 2) / den
    }
}

fn clamp_i64_to_u32(value: i64) -> u32 {
    if value <= 0 {
        return 0;
    }
    if value >= i64::from(u32::MAX) {
        return u32::MAX;
    }
    value as u32
}

fn clamp_i64_to_i32(value: i64) -> i32 {
    if value <= i64::from(i32::MIN) {
        return i32::MIN;
    }
    if value >= i64::from(i32::MAX) {
        return i32::MAX;
    }
    value as i32
}
