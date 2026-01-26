use core::ffi::{c_int, c_uchar, c_ulong};
use std::ffi::{CStr, CString};

use super::xlib::{self, Atom, Display, Window};

const SCALE_ONE_1024: u32 = 1024;
const CSS_REFERENCE_DPI: u32 = 96;

const MIN_SCALE_1024: u32 = 256; // 0.25x
const MAX_SCALE_1024: u32 = 8192; // 8.0x

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ScaleFactor {
    scale_1024: u32,
}

impl ScaleFactor {
    pub fn detect(display: *mut Display, screen: c_int) -> Self {
        if let Some(scale) = scale_from_env() {
            return Self::new(scale);
        }
        if let Some(scale) = scale_from_xsettings(display, screen) {
            return Self::new(scale);
        }
        if let Some(scale) = scale_from_xresources(display) {
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

fn scale_from_xsettings(display: *mut Display, screen: c_int) -> Option<u32> {
    let settings_bytes = xsettings_blob(display, screen)?;
    let xft_dpi_1024 = xsettings_find_int(&settings_bytes, "Xft/DPI")?;
    if xft_dpi_1024 == 0 {
        return None;
    }

    let scale_1024 = ((u64::from(xft_dpi_1024) + u64::from(CSS_REFERENCE_DPI) / 2)
        / u64::from(CSS_REFERENCE_DPI)) as u32;
    Some(scale_1024)
}

fn xsettings_blob(display: *mut Display, screen: c_int) -> Option<Vec<u8>> {
    let selection_name = CString::new(format!("_XSETTINGS_S{screen}")).ok()?;
    let selection = unsafe { xlib::XInternAtom(display, selection_name.as_ptr(), 0) };
    if selection == 0 {
        return None;
    }

    let owner: Window = unsafe { xlib::XGetSelectionOwner(display, selection) };
    if owner == 0 {
        return None;
    }

    let property_name = CString::new("_XSETTINGS_SETTINGS").ok()?;
    let property: Atom = unsafe { xlib::XInternAtom(display, property_name.as_ptr(), 0) };
    if property == 0 {
        return None;
    }

    let mut actual_type: Atom = 0;
    let mut actual_format: c_int = 0;
    let mut nitems: c_ulong = 0;
    let mut bytes_after: c_ulong = 0;
    let mut prop: *mut c_uchar = std::ptr::null_mut();

    let status = unsafe {
        xlib::XGetWindowProperty(
            display,
            owner,
            property,
            0,
            65536,
            0,
            0,
            &mut actual_type,
            &mut actual_format,
            &mut nitems,
            &mut bytes_after,
            &mut prop,
        )
    };
    if status != 0 || prop.is_null() || actual_format != 8 {
        if !prop.is_null() {
            unsafe {
                xlib::XFree(prop.cast());
            }
        }
        return None;
    }

    let len: usize = nitems.try_into().ok()?;
    let bytes = unsafe { std::slice::from_raw_parts(prop, len) }.to_vec();
    unsafe {
        xlib::XFree(prop.cast());
    }
    Some(bytes)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ByteOrder {
    Little,
    Big,
}

fn xsettings_find_int(blob: &[u8], name: &str) -> Option<u32> {
    if blob.len() < 12 {
        return None;
    }

    let order = match blob[0] {
        b'l' | b'L' => ByteOrder::Little,
        b'B' | b'b' => ByteOrder::Big,
        _ => return None,
    };

    let mut cursor = 4usize;
    let _serial = read_u32(blob, &mut cursor, order)?;
    let n_settings = read_u32(blob, &mut cursor, order)? as usize;

    for _ in 0..n_settings {
        if cursor + 4 > blob.len() {
            return None;
        }

        let setting_type = blob[cursor];
        cursor += 2; // type + pad
        let name_len = read_u16(blob, &mut cursor, order)? as usize;
        if cursor + name_len > blob.len() {
            return None;
        }
        let name_bytes = &blob[cursor..cursor + name_len];
        cursor += name_len;

        while cursor % 4 != 0 {
            cursor += 1;
        }

        let _last_change_serial = read_u32(blob, &mut cursor, order)?;

        match setting_type {
            0 => {
                let value = read_u32(blob, &mut cursor, order)?;
                if name_bytes == name.as_bytes() {
                    return Some(value);
                }
            }
            1 => {
                let len = read_u32(blob, &mut cursor, order)? as usize;
                if cursor + len > blob.len() {
                    return None;
                }
                cursor += len;
                while cursor % 4 != 0 {
                    cursor += 1;
                }
            }
            2 => {
                cursor = cursor.checked_add(8)?;
                if cursor > blob.len() {
                    return None;
                }
            }
            _ => return None,
        }
    }

    None
}

fn read_u16(blob: &[u8], cursor: &mut usize, order: ByteOrder) -> Option<u16> {
    let bytes = blob.get(*cursor..*cursor + 2)?;
    *cursor += 2;
    let value = match order {
        ByteOrder::Little => u16::from_le_bytes(bytes.try_into().ok()?),
        ByteOrder::Big => u16::from_be_bytes(bytes.try_into().ok()?),
    };
    Some(value)
}

fn read_u32(blob: &[u8], cursor: &mut usize, order: ByteOrder) -> Option<u32> {
    let bytes = blob.get(*cursor..*cursor + 4)?;
    *cursor += 4;
    let value = match order {
        ByteOrder::Little => u32::from_le_bytes(bytes.try_into().ok()?),
        ByteOrder::Big => u32::from_be_bytes(bytes.try_into().ok()?),
    };
    Some(value)
}

fn scale_from_xresources(display: *mut Display) -> Option<u32> {
    let ptr = unsafe { xlib::XResourceManagerString(display) };
    if ptr.is_null() {
        return None;
    }
    let s = unsafe { CStr::from_ptr(ptr) }.to_string_lossy();

    for line in s.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('!') {
            continue;
        }

        let (key, value) = match line.split_once(':').or_else(|| line.split_once('=')) {
            Some((k, v)) => (k.trim(), v.trim()),
            None => continue,
        };

        if !key.eq_ignore_ascii_case("Xft.dpi") && !key.ends_with("Xft.dpi") {
            continue;
        }

        let dpi: f32 = value.parse().ok()?;
        if !dpi.is_finite() || dpi <= 0.0 {
            return None;
        }

        let scale_1024 = (dpi * 1024.0 / (CSS_REFERENCE_DPI as f32)).round() as i64;
        if scale_1024 <= 0 {
            return None;
        }
        return Some(clamp_i64_to_u32(scale_1024));
    }

    None
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_sizes_at_125_percent_scale() {
        let scale = ScaleFactor::new(1280);

        assert_eq!(scale.css_size_to_device_px(1024), 1280);
        assert_eq!(scale.device_size_to_css_px(1280), 1024);

        assert_eq!(scale.device_delta_to_css_px(48), 38);
        assert_eq!(scale.device_delta_to_css_px(-48), -38);
    }

    #[test]
    fn maps_device_coords_into_matching_css_intervals() {
        let scale = ScaleFactor::new(1280);

        // CSS pixel 0 maps to device [0,1)
        assert_eq!(scale.css_coord_to_device_px(0), 0);
        assert_eq!(scale.css_coord_to_device_px(1), 1);
        assert_eq!(scale.device_coord_to_css_px(0), 0);

        // CSS pixel 1 maps to device [1,3)
        assert_eq!(scale.css_coord_to_device_px(2), 3);
        assert_eq!(scale.device_coord_to_css_px(1), 1);
        assert_eq!(scale.device_coord_to_css_px(2), 1);

        // CSS pixel 2 starts at device 3
        assert_eq!(scale.device_coord_to_css_px(3), 2);
    }
}
