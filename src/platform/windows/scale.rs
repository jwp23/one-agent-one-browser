use core::ffi::c_void;

type UINT = u32;
type HWND = *mut c_void;

const SCALE_ONE_1024: u32 = 1024;
const CSS_REFERENCE_DPI: u32 = 96;

const MIN_SCALE_1024: u32 = 256; // 0.25x
const MAX_SCALE_1024: u32 = 8192; // 8.0x

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ScaleFactor {
    scale_1024: u32,
}

impl ScaleFactor {
    pub fn detect(headless: bool, hwnd: Option<HWND>) -> Self {
        if let Some(scale) = scale_from_env() {
            return Self::new(scale);
        }

        if !headless {
            if let Some(hwnd) = hwnd {
                if let Some(scale) = scale_from_hwnd(hwnd) {
                    return Self::new(scale);
                }
            }
        }

        if let Some(scale) = scale_from_system_dpi() {
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

    pub fn css_size_to_device_px(self, css_px: i32) -> i32 {
        let css_px = i64::from(css_px);
        let scaled = mul_div_round_nearest(css_px, i64::from(self.scale_1024), 1024);
        clamp_i64_to_i32(scaled.max(1))
    }

    pub fn device_size_to_css_px(self, device_px: i32) -> i32 {
        if self.scale_1024 == SCALE_ONE_1024 {
            return device_px.max(1);
        }
        let device_px = i64::from(device_px.max(1));
        let denom = i64::from(self.scale_1024);
        let mut css = mul_div_round_nearest(device_px, 1024, denom).max(1);

        for _ in 0..4 {
            let mapped = mul_div_round_nearest(css, denom, 1024);
            if mapped == device_px {
                break;
            }
            if mapped < device_px {
                css += 1;
            } else {
                css -= 1;
                if css < 1 {
                    css = 1;
                    break;
                }
            }
        }

        clamp_i64_to_i32(css)
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

    pub fn device_coord_to_css_px(self, device_px: i32) -> i32 {
        if self.scale_1024 == SCALE_ONE_1024 {
            return device_px;
        }

        let device_px_i64 = i64::from(device_px);
        let denom = i64::from(self.scale_1024);
        let mut css = mul_div_round_nearest(device_px_i64, 1024, denom);

        for _ in 0..2 {
            let start_dev = mul_div_round_nearest(css, denom, 1024);
            let end_dev = mul_div_round_nearest(css.saturating_add(1), denom, 1024);
            if device_px_i64 < start_dev {
                css -= 1;
                continue;
            }
            if device_px_i64 >= end_dev {
                css += 1;
                continue;
            }
            break;
        }

        clamp_i64_to_i32(css)
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

fn scale_from_system_dpi() -> Option<u32> {
    let dpi = unsafe { GetDpiForSystem() };
    if dpi == 0 {
        return None;
    }
    let scale_1024 = (dpi as u64)
        .checked_mul(1024)
        .and_then(|n| n.checked_div(CSS_REFERENCE_DPI as u64))
        .and_then(|n| u32::try_from(n).ok())?;
    Some(scale_1024)
}

fn scale_from_hwnd(hwnd: HWND) -> Option<u32> {
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 {
        return None;
    }
    let scale_1024 = (dpi as u64)
        .checked_mul(1024)
        .and_then(|n| n.checked_div(CSS_REFERENCE_DPI as u64))
        .and_then(|n| u32::try_from(n).ok())?;
    Some(scale_1024)
}

fn mul_div_round_nearest(value: i64, mul: i64, div: i64) -> i64 {
    if div == 0 {
        return 0;
    }
    let numerator = value.saturating_mul(mul);
    div_round_nearest(numerator, div)
}

fn div_round_nearest(numerator: i64, denom: i64) -> i64 {
    if denom == 0 {
        return 0;
    }
    if numerator >= 0 {
        (numerator + denom / 2) / denom
    } else {
        (numerator - denom / 2) / denom
    }
}

fn clamp_i64_to_i32(value: i64) -> i32 {
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

fn clamp_i64_to_u32(value: i64) -> u32 {
    value.clamp(i64::from(u32::MIN), i64::from(u32::MAX)) as u32
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetDpiForSystem() -> UINT;
    fn GetDpiForWindow(hwnd: HWND) -> UINT;
}
