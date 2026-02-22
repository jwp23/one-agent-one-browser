use crate::win::com;
use crate::win::com::{ComPtr, GUID, HRESULT, HResultError};
use core::ffi::c_void;

type BOOL = i32;
type FLOAT = f32;
type UINT32 = u32;

pub(super) enum IDWriteFactory {}
pub(super) enum IDWriteTextFormat {}
pub(super) enum IDWriteTextLayout {}

const DWRITE_FACTORY_TYPE_SHARED: u32 = 0;

pub(super) const DWRITE_FONT_STYLE_NORMAL: u32 = 0;
pub(super) const DWRITE_FONT_STRETCH_NORMAL: u32 = 5;

pub(super) const DWRITE_FONT_WEIGHT_NORMAL: u32 = 400;
pub(super) const DWRITE_FONT_WEIGHT_BOLD: u32 = 700;

pub(super) const DWRITE_MEASURING_MODE_NATURAL: u32 = 0;

const IID_IDWRITE_FACTORY: GUID = GUID {
    data1: 0xb859_ee5a,
    data2: 0xd838,
    data3: 0x4b5b,
    data4: [0xa2, 0xe8, 0x1a, 0xdc, 0x7d, 0x93, 0xdb, 0x48],
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct DWRITE_LINE_METRICS {
    pub(super) length: UINT32,
    pub(super) trailing_whitespace_length: UINT32,
    pub(super) newline_length: UINT32,
    pub(super) height: FLOAT,
    pub(super) baseline: FLOAT,
    pub(super) is_trimmed: BOOL,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct DWRITE_TEXT_METRICS {
    pub(super) left: FLOAT,
    pub(super) top: FLOAT,
    pub(super) width: FLOAT,
    pub(super) width_including_trailing_whitespace: FLOAT,
    pub(super) height: FLOAT,
    pub(super) layout_width: FLOAT,
    pub(super) layout_height: FLOAT,
    pub(super) max_bidi_reordering_depth: UINT32,
    pub(super) line_count: UINT32,
}

#[link(name = "dwrite")]
unsafe extern "system" {
    fn DWriteCreateFactory(
        factory_type: u32,
        iid: *const GUID,
        factory: *mut *mut c_void,
    ) -> HRESULT;
}

pub(super) fn create_factory() -> Result<ComPtr<IDWriteFactory>, String> {
    com::ensure_initialized()?;

    let mut out: *mut c_void = std::ptr::null_mut();
    let hr =
        unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED, &IID_IDWRITE_FACTORY, &mut out) };
    if !com::succeeded(hr) {
        return Err(format!(
            "DWriteCreateFactory failed: {}",
            com::hresult_string(hr)
        ));
    }
    if out.is_null() {
        return Err("DWriteCreateFactory returned null".to_owned());
    }
    Ok(ComPtr::from_raw(out.cast::<IDWriteFactory>()))
}

pub(super) fn create_text_format(
    factory: &ComPtr<IDWriteFactory>,
    family_name: *const u16,
    locale_name: *const u16,
    font_weight: u32,
    font_style: u32,
    font_stretch: u32,
    font_size: f32,
) -> Result<ComPtr<IDWriteTextFormat>, HResultError> {
    let mut format: *mut IDWriteTextFormat = std::ptr::null_mut();
    let hr = unsafe {
        let f: unsafe extern "system" fn(
            *mut c_void,
            *const u16,
            *mut c_void,
            u32,
            u32,
            u32,
            f32,
            *const u16,
            *mut *mut IDWriteTextFormat,
        ) -> HRESULT = std::mem::transmute(vtbl_entry(factory.as_ptr().cast::<c_void>(), 15));
        f(
            factory.as_ptr().cast::<c_void>(),
            family_name,
            std::ptr::null_mut(),
            font_weight,
            font_style,
            font_stretch,
            font_size,
            locale_name,
            &mut format,
        )
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "IDWriteFactory::CreateTextFormat failed",
        });
    }
    if format.is_null() {
        return Err(HResultError {
            hr: -1,
            context: "IDWriteFactory::CreateTextFormat returned null",
        });
    }
    Ok(ComPtr::from_raw(format))
}

pub(super) fn create_text_layout(
    factory: &ComPtr<IDWriteFactory>,
    text: *const u16,
    text_len: u32,
    format: *mut IDWriteTextFormat,
    max_width: f32,
    max_height: f32,
) -> Result<ComPtr<IDWriteTextLayout>, HResultError> {
    let mut layout: *mut IDWriteTextLayout = std::ptr::null_mut();
    let hr = unsafe {
        let f: unsafe extern "system" fn(
            *mut c_void,
            *const u16,
            u32,
            *mut c_void,
            f32,
            f32,
            *mut *mut IDWriteTextLayout,
        ) -> HRESULT = std::mem::transmute(vtbl_entry(factory.as_ptr().cast::<c_void>(), 18));
        f(
            factory.as_ptr().cast::<c_void>(),
            text,
            text_len,
            format.cast::<c_void>(),
            max_width,
            max_height,
            &mut layout,
        )
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "IDWriteFactory::CreateTextLayout failed",
        });
    }
    if layout.is_null() {
        return Err(HResultError {
            hr: -1,
            context: "IDWriteFactory::CreateTextLayout returned null",
        });
    }
    Ok(ComPtr::from_raw(layout))
}

pub(super) fn text_layout_get_metrics(
    layout: &ComPtr<IDWriteTextLayout>,
) -> Result<DWRITE_TEXT_METRICS, HResultError> {
    let mut metrics = DWRITE_TEXT_METRICS::default();
    let hr = unsafe {
        let f: unsafe extern "system" fn(*mut c_void, *mut DWRITE_TEXT_METRICS) -> HRESULT =
            std::mem::transmute(vtbl_entry(layout.as_ptr().cast::<c_void>(), 60));
        f(layout.as_ptr().cast::<c_void>(), &mut metrics)
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "IDWriteTextLayout::GetMetrics failed",
        });
    }
    Ok(metrics)
}

pub(super) fn text_layout_get_line_metrics(
    layout: &ComPtr<IDWriteTextLayout>,
) -> Result<Vec<DWRITE_LINE_METRICS>, HResultError> {
    let mut needed: u32 = 0;
    let hr = unsafe {
        let f: unsafe extern "system" fn(
            *mut c_void,
            *mut DWRITE_LINE_METRICS,
            u32,
            *mut u32,
        ) -> HRESULT = std::mem::transmute(vtbl_entry(layout.as_ptr().cast::<c_void>(), 59));
        f(
            layout.as_ptr().cast::<c_void>(),
            std::ptr::null_mut(),
            0,
            &mut needed,
        )
    };

    if com::succeeded(hr) && needed == 0 {
        return Ok(Vec::new());
    }

    if needed == 0 {
        return Err(HResultError {
            hr,
            context: "IDWriteTextLayout::GetLineMetrics failed (count)",
        });
    }

    let mut out = vec![DWRITE_LINE_METRICS::default(); needed as usize];
    let mut actual: u32 = 0;
    let hr2 = unsafe {
        let f: unsafe extern "system" fn(
            *mut c_void,
            *mut DWRITE_LINE_METRICS,
            u32,
            *mut u32,
        ) -> HRESULT = std::mem::transmute(vtbl_entry(layout.as_ptr().cast::<c_void>(), 59));
        f(
            layout.as_ptr().cast::<c_void>(),
            out.as_mut_ptr(),
            needed,
            &mut actual,
        )
    };
    if !com::succeeded(hr2) {
        return Err(HResultError {
            hr: hr2,
            context: "IDWriteTextLayout::GetLineMetrics failed (fill)",
        });
    }
    out.truncate(actual as usize);
    Ok(out)
}

unsafe fn vtbl_entry(this: *mut c_void, index: usize) -> *const c_void {
    debug_assert!(!this.is_null());
    unsafe {
        let vtbl = *(this as *mut *const *const c_void);
        *vtbl.add(index)
    }
}
