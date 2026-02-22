use core::ffi::c_void;

type BOOL = i32;
type DWORD = u32;
type HDC = *mut c_void;
type HWND = *mut c_void;
type INT = i32;
type UINT = u32;
type WORD = u16;

const BI_RGB: DWORD = 0;
const DIB_RGB_COLORS: UINT = 0;
const SRCCOPY: DWORD = 0x00CC_0020;

#[repr(C)]
struct RECT {
    left: LONG,
    top: LONG,
    right: LONG,
    bottom: LONG,
}

type LONG = i32;

#[repr(C)]
struct BITMAPINFOHEADER {
    bi_size: DWORD,
    bi_width: LONG,
    bi_height: LONG,
    bi_planes: WORD,
    bi_bit_count: WORD,
    bi_compression: DWORD,
    bi_size_image: DWORD,
    bi_x_pels_per_meter: LONG,
    bi_y_pels_per_meter: LONG,
    bi_clr_used: DWORD,
    bi_clr_important: DWORD,
}

#[repr(C)]
struct RGBQUAD {
    rgb_blue: u8,
    rgb_green: u8,
    rgb_red: u8,
    rgb_reserved: u8,
}

#[repr(C)]
struct BITMAPINFO {
    bmi_header: BITMAPINFOHEADER,
    bmi_colors: [RGBQUAD; 1],
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetClientRect(hwnd: HWND, rect: *mut RECT) -> BOOL;
    fn GetDC(hwnd: HWND) -> HDC;
    fn ReleaseDC(hwnd: HWND, hdc: HDC) -> INT;
}

#[link(name = "gdi32")]
unsafe extern "system" {
    fn StretchDIBits(
        hdc: HDC,
        x_dest: INT,
        y_dest: INT,
        dest_width: INT,
        dest_height: INT,
        x_src: INT,
        y_src: INT,
        src_width: INT,
        src_height: INT,
        bits: *const c_void,
        bits_info: *const BITMAPINFO,
        usage: UINT,
        rop: DWORD,
    ) -> INT;
}

pub(super) fn blit_bgra(
    hwnd: HWND,
    src_width_px: i32,
    src_height_px: i32,
    bgra: &[u8],
) -> Result<(), String> {
    if hwnd.is_null() {
        return Err("blit_bgra called with null HWND".to_owned());
    }
    if src_width_px <= 0 || src_height_px <= 0 {
        return Ok(());
    }

    let expected_len = (src_width_px as usize)
        .checked_mul(src_height_px as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "Back buffer size overflow".to_owned())?;
    if bgra.len() != expected_len {
        return Err(format!(
            "Invalid BGRA buffer length: expected {expected_len} bytes, got {}",
            bgra.len()
        ));
    }

    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let ok = unsafe { GetClientRect(hwnd, &mut rect) };
    if ok == 0 {
        return Err("GetClientRect failed".to_owned());
    }
    let dst_width = rect.right.saturating_sub(rect.left).max(0);
    let dst_height = rect.bottom.saturating_sub(rect.top).max(0);
    if dst_width == 0 || dst_height == 0 {
        return Ok(());
    }

    let hdc = unsafe { GetDC(hwnd) };
    if hdc.is_null() {
        return Err("GetDC returned null".to_owned());
    }
    struct DcGuard {
        hwnd: HWND,
        hdc: HDC,
    }
    impl Drop for DcGuard {
        fn drop(&mut self) {
            if self.hdc.is_null() {
                return;
            }
            unsafe {
                let _ = ReleaseDC(self.hwnd, self.hdc);
            }
            self.hdc = std::ptr::null_mut();
        }
    }
    let dc_guard = DcGuard { hwnd, hdc };

    let header = BITMAPINFOHEADER {
        bi_size: core::mem::size_of::<BITMAPINFOHEADER>() as DWORD,
        bi_width: src_width_px,
        bi_height: -src_height_px, // top-down
        bi_planes: 1,
        bi_bit_count: 32,
        bi_compression: BI_RGB,
        bi_size_image: 0,
        bi_x_pels_per_meter: 0,
        bi_y_pels_per_meter: 0,
        bi_clr_used: 0,
        bi_clr_important: 0,
    };
    let info = BITMAPINFO {
        bmi_header: header,
        bmi_colors: [RGBQUAD {
            rgb_blue: 0,
            rgb_green: 0,
            rgb_red: 0,
            rgb_reserved: 0,
        }],
    };

    let copied = unsafe {
        StretchDIBits(
            dc_guard.hdc,
            0,
            0,
            dst_width,
            dst_height,
            0,
            0,
            src_width_px,
            src_height_px,
            bgra.as_ptr().cast::<c_void>(),
            &info,
            DIB_RGB_COLORS,
            SRCCOPY,
        )
    };
    if copied == 0 {
        return Err("StretchDIBits failed".to_owned());
    }

    Ok(())
}
