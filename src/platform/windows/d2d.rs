use crate::win::com;
use crate::win::com::{ComPtr, GUID, HResultError, HRESULT};
use core::ffi::c_void;

pub(super) enum ID2D1Factory1 {}
pub(super) enum ID2D1Device {}
pub(super) enum ID2D1DeviceContext {}
pub(super) enum ID2D1DeviceContext5 {}
pub(super) enum ID2D1Bitmap1 {}
pub(super) enum ID2D1Layer {}
pub(super) enum ID2D1SolidColorBrush {}
pub(super) enum ID2D1SvgDocument {}

pub(super) const D2D1_DRAW_TEXT_OPTIONS_NONE: u32 = 0;
pub(super) const D2D1_TEXT_ANTIALIAS_MODE_CLEARTYPE: u32 = 1;
pub(super) const D2D1_UNIT_MODE_PIXELS: u32 = 1;

pub(super) const D2D1_BITMAP_OPTIONS_TARGET: u32 = 0x0000_0001;
pub(super) const D2D1_BITMAP_OPTIONS_CANNOT_DRAW: u32 = 0x0000_0002;
pub(super) const D2D1_BITMAP_OPTIONS_CPU_READ: u32 = 0x0000_0004;

pub(super) const D2D1_BITMAP_INTERPOLATION_MODE_LINEAR: u32 = 1;
pub(super) const D2D1_ANTIALIAS_MODE_PER_PRIMITIVE: u32 = 0;
pub(super) const D2D1_LAYER_OPTIONS1_NONE: u32 = 0;

pub(super) const D2D1_MAP_OPTIONS_READ: u32 = 0x1;

const D2D1_FACTORY_TYPE_SINGLE_THREADED: u32 = 0;

const DXGI_FORMAT_B8G8R8A8_UNORM: u32 = 87;
const D2D1_ALPHA_MODE_PREMULTIPLIED: u32 = 1;

const IID_ID2D1_FACTORY1: GUID = GUID {
    data1: 0xbb12_d362,
    data2: 0xdaee,
    data3: 0x4b9a,
    data4: [0xaa, 0x1d, 0x14, 0xba, 0x40, 0x1c, 0xfa, 0x1f],
};

const IID_ID2D1_DEVICE_CONTEXT5: GUID = GUID {
    data1: 0x7836_d248,
    data2: 0x68cc,
    data3: 0x4df6,
    data4: [0xb9, 0xe8, 0xde, 0x99, 0x1b, 0xf6, 0x2e, 0xb7],
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct D2D1_SIZE_U {
    pub(super) width: u32,
    pub(super) height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct D2D1_SIZE_F {
    pub(super) width: f32,
    pub(super) height: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct D2D1_POINT_2F {
    pub(super) x: f32,
    pub(super) y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct D2D1_RECT_F {
    pub(super) left: f32,
    pub(super) top: f32,
    pub(super) right: f32,
    pub(super) bottom: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct D2D1_MATRIX_3X2_F {
    pub(super) m11: f32,
    pub(super) m12: f32,
    pub(super) m21: f32,
    pub(super) m22: f32,
    pub(super) dx: f32,
    pub(super) dy: f32,
}

pub(super) const D2D1_IDENTITY_MATRIX: D2D1_MATRIX_3X2_F = D2D1_MATRIX_3X2_F {
    m11: 1.0,
    m12: 0.0,
    m21: 0.0,
    m22: 1.0,
    dx: 0.0,
    dy: 0.0,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct D2D1_COLOR_F {
    pub(super) r: f32,
    pub(super) g: f32,
    pub(super) b: f32,
    pub(super) a: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct D2D1_PIXEL_FORMAT {
    pub(super) format: u32,
    pub(super) alpha_mode: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct D2D1_BITMAP_PROPERTIES1 {
    pub(super) pixel_format: D2D1_PIXEL_FORMAT,
    pub(super) dpi_x: f32,
    pub(super) dpi_y: f32,
    pub(super) bitmap_options: u32,
    pub(super) color_context: *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct D2D1_ROUNDED_RECT {
    pub(super) rect: D2D1_RECT_F,
    pub(super) radius_x: f32,
    pub(super) radius_y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct D2D1_LAYER_PARAMETERS1 {
    pub(super) content_bounds: D2D1_RECT_F,
    pub(super) geometric_mask: *mut c_void,
    pub(super) mask_antialias_mode: u32,
    pub(super) mask_transform: D2D1_MATRIX_3X2_F,
    pub(super) opacity: f32,
    pub(super) opacity_brush: *mut c_void,
    pub(super) layer_options1: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct D2D1_MAPPED_RECT {
    pub(super) pitch: u32,
    pub(super) bits: *mut u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct D2D1_FACTORY_OPTIONS {
    pub(super) debug_level: u32,
}

#[link(name = "d2d1")]
unsafe extern "system" {
    fn D2D1CreateFactory(
        factory_type: u32,
        iid: *const GUID,
        options: *const D2D1_FACTORY_OPTIONS,
        factory: *mut *mut c_void,
    ) -> HRESULT;
}

pub(super) fn create_factory1() -> Result<ComPtr<ID2D1Factory1>, String> {
    com::ensure_initialized()?;

    let options = D2D1_FACTORY_OPTIONS { debug_level: 0 };
    let mut out: *mut c_void = std::ptr::null_mut();
    let hr = unsafe {
        D2D1CreateFactory(
            D2D1_FACTORY_TYPE_SINGLE_THREADED,
            &IID_ID2D1_FACTORY1,
            &options,
            &mut out,
        )
    };
    if !com::succeeded(hr) {
        return Err(format!("D2D1CreateFactory failed: {}", com::hresult_string(hr)));
    }
    if out.is_null() {
        return Err("D2D1CreateFactory returned null".to_owned());
    }
    Ok(ComPtr::from_raw(out.cast::<ID2D1Factory1>()))
}

pub(super) fn factory_create_device(
    factory: &ComPtr<ID2D1Factory1>,
    dxgi_device: *mut c_void,
) -> Result<ComPtr<ID2D1Device>, HResultError> {
    let mut device: *mut ID2D1Device = std::ptr::null_mut();
    let hr = unsafe {
        let f: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut *mut ID2D1Device) -> HRESULT =
            std::mem::transmute(vtbl_entry(factory.as_ptr().cast::<c_void>(), 17));
        f(
            factory.as_ptr().cast::<c_void>(),
            dxgi_device,
            &mut device,
        )
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "ID2D1Factory1::CreateDevice failed",
        });
    }
    if device.is_null() {
        return Err(HResultError {
            hr: -1,
            context: "ID2D1Factory1::CreateDevice returned null",
        });
    }
    Ok(ComPtr::from_raw(device))
}

pub(super) fn device_create_device_context(
    device: &ComPtr<ID2D1Device>,
) -> Result<ComPtr<ID2D1DeviceContext5>, HResultError> {
    let mut ctx: *mut ID2D1DeviceContext = std::ptr::null_mut();
    let hr = unsafe {
        let f: unsafe extern "system" fn(*mut c_void, u32, *mut *mut ID2D1DeviceContext) -> HRESULT =
            std::mem::transmute(vtbl_entry(device.as_ptr().cast::<c_void>(), 4));
        f(device.as_ptr().cast::<c_void>(), 0, &mut ctx)
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "ID2D1Device::CreateDeviceContext failed",
        });
    }
    if ctx.is_null() {
        return Err(HResultError {
            hr: -1,
            context: "ID2D1Device::CreateDeviceContext returned null",
        });
    }

    let base = ComPtr::from_raw(ctx);
    let ctx5: ComPtr<ID2D1DeviceContext5> = com::query_interface(
        base.as_ptr().cast::<c_void>(),
        &IID_ID2D1_DEVICE_CONTEXT5,
        "ID2D1DeviceContext::QueryInterface(ID2D1DeviceContext5) failed",
    )?;
    Ok(ctx5)
}

pub(super) fn ctx_set_unit_mode(ctx: &ComPtr<ID2D1DeviceContext5>, unit_mode: u32) {
    unsafe {
        let f: unsafe extern "system" fn(*mut c_void, u32) =
            std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 80));
        f(ctx.as_ptr().cast::<c_void>(), unit_mode);
    }
}

pub(super) fn ctx_set_text_antialias_mode(ctx: &ComPtr<ID2D1DeviceContext5>, mode: u32) {
    unsafe {
        let f: unsafe extern "system" fn(*mut c_void, u32) =
            std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 34));
        f(ctx.as_ptr().cast::<c_void>(), mode);
    }
}

pub(super) fn ctx_set_transform(ctx: &ComPtr<ID2D1DeviceContext5>, transform: &D2D1_MATRIX_3X2_F) {
    unsafe {
        let f: unsafe extern "system" fn(*mut c_void, *const D2D1_MATRIX_3X2_F) =
            std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 30));
        f(ctx.as_ptr().cast::<c_void>(), transform);
    }
}

pub(super) fn ctx_begin_draw(ctx: &ComPtr<ID2D1DeviceContext5>) {
    unsafe {
        let f: unsafe extern "system" fn(*mut c_void) =
            std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 48));
        f(ctx.as_ptr().cast::<c_void>());
    }
}

pub(super) fn ctx_end_draw(ctx: &ComPtr<ID2D1DeviceContext5>) -> Result<(), HResultError> {
    let hr = unsafe {
        let f: unsafe extern "system" fn(*mut c_void, *mut u64, *mut u64) -> HRESULT =
            std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 49));
        f(ctx.as_ptr().cast::<c_void>(), std::ptr::null_mut(), std::ptr::null_mut())
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "ID2D1RenderTarget::EndDraw failed",
        });
    }
    Ok(())
}

pub(super) fn ctx_clear(ctx: &ComPtr<ID2D1DeviceContext5>, color: &D2D1_COLOR_F) {
    unsafe {
        let f: unsafe extern "system" fn(*mut c_void, *const D2D1_COLOR_F) =
            std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 47));
        f(ctx.as_ptr().cast::<c_void>(), color);
    }
}

pub(super) fn ctx_create_bitmap(
    ctx: &ComPtr<ID2D1DeviceContext5>,
    size: D2D1_SIZE_U,
    data: Option<(*const u8, u32)>,
    bitmap_options: u32,
) -> Result<ComPtr<ID2D1Bitmap1>, HResultError> {
    let props = D2D1_BITMAP_PROPERTIES1 {
        pixel_format: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alpha_mode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpi_x: 96.0,
        dpi_y: 96.0,
        bitmap_options,
        color_context: std::ptr::null_mut(),
    };

    let (data_ptr, pitch) = data
        .map(|(ptr, pitch)| (ptr.cast::<c_void>(), pitch))
        .unwrap_or((std::ptr::null(), 0));

    let mut bitmap: *mut ID2D1Bitmap1 = std::ptr::null_mut();
    let hr = unsafe {
        let f: unsafe extern "system" fn(
            *mut c_void,
            D2D1_SIZE_U,
            *const c_void,
            u32,
            *const D2D1_BITMAP_PROPERTIES1,
            *mut *mut ID2D1Bitmap1,
        ) -> HRESULT = std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 57));
        f(
            ctx.as_ptr().cast::<c_void>(),
            size,
            data_ptr,
            pitch,
            &props,
            &mut bitmap,
        )
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "ID2D1DeviceContext::CreateBitmap failed",
        });
    }
    if bitmap.is_null() {
        return Err(HResultError {
            hr: -1,
            context: "ID2D1DeviceContext::CreateBitmap returned null",
        });
    }
    Ok(ComPtr::from_raw(bitmap))
}

pub(super) fn ctx_set_target(ctx: &ComPtr<ID2D1DeviceContext5>, target: &ComPtr<ID2D1Bitmap1>) {
    unsafe {
        let f: unsafe extern "system" fn(*mut c_void, *mut c_void) =
            std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 74));
        f(
            ctx.as_ptr().cast::<c_void>(),
            target.as_ptr().cast::<c_void>(),
        );
    }
}

pub(super) fn ctx_create_solid_color_brush(
    ctx: &ComPtr<ID2D1DeviceContext5>,
    color: &D2D1_COLOR_F,
) -> Result<ComPtr<ID2D1SolidColorBrush>, HResultError> {
    let mut brush: *mut ID2D1SolidColorBrush = std::ptr::null_mut();
    let hr = unsafe {
        let f: unsafe extern "system" fn(
            *mut c_void,
            *const D2D1_COLOR_F,
            *const c_void,
            *mut *mut ID2D1SolidColorBrush,
        ) -> HRESULT = std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 8));
        f(
            ctx.as_ptr().cast::<c_void>(),
            color,
            std::ptr::null(),
            &mut brush,
        )
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "ID2D1RenderTarget::CreateSolidColorBrush failed",
        });
    }
    if brush.is_null() {
        return Err(HResultError {
            hr: -1,
            context: "CreateSolidColorBrush returned null",
        });
    }
    Ok(ComPtr::from_raw(brush))
}

pub(super) fn ctx_create_layer(ctx: &ComPtr<ID2D1DeviceContext5>) -> Result<ComPtr<ID2D1Layer>, HResultError> {
    let mut layer: *mut ID2D1Layer = std::ptr::null_mut();
    let hr = unsafe {
        let f: unsafe extern "system" fn(*mut c_void, *const D2D1_SIZE_F, *mut *mut ID2D1Layer) -> HRESULT =
            std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 13));
        f(ctx.as_ptr().cast::<c_void>(), std::ptr::null(), &mut layer)
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "ID2D1RenderTarget::CreateLayer failed",
        });
    }
    if layer.is_null() {
        return Err(HResultError {
            hr: -1,
            context: "CreateLayer returned null",
        });
    }
    Ok(ComPtr::from_raw(layer))
}

pub(super) fn ctx_push_layer(
    ctx: &ComPtr<ID2D1DeviceContext5>,
    params: &D2D1_LAYER_PARAMETERS1,
    layer: &ComPtr<ID2D1Layer>,
) {
    unsafe {
        let f: unsafe extern "system" fn(*mut c_void, *const D2D1_LAYER_PARAMETERS1, *mut c_void) =
            std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 40));
        f(
            ctx.as_ptr().cast::<c_void>(),
            params,
            layer.as_ptr().cast::<c_void>(),
        );
    }
}

pub(super) fn ctx_pop_layer(ctx: &ComPtr<ID2D1DeviceContext5>) {
    unsafe {
        let f: unsafe extern "system" fn(*mut c_void) =
            std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 41));
        f(ctx.as_ptr().cast::<c_void>());
    }
}

pub(super) fn ctx_fill_rectangle(
    ctx: &ComPtr<ID2D1DeviceContext5>,
    rect: &D2D1_RECT_F,
    brush: *mut ID2D1SolidColorBrush,
) {
    unsafe {
        let f: unsafe extern "system" fn(*mut c_void, *const D2D1_RECT_F, *mut c_void) =
            std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 17));
        f(
            ctx.as_ptr().cast::<c_void>(),
            rect,
            brush.cast::<c_void>(),
        );
    }
}

pub(super) fn ctx_fill_rounded_rectangle(
    ctx: &ComPtr<ID2D1DeviceContext5>,
    rect: &D2D1_ROUNDED_RECT,
    brush: *mut ID2D1SolidColorBrush,
) {
    unsafe {
        let f: unsafe extern "system" fn(*mut c_void, *const D2D1_ROUNDED_RECT, *mut c_void) =
            std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 19));
        f(
            ctx.as_ptr().cast::<c_void>(),
            rect,
            brush.cast::<c_void>(),
        );
    }
}

pub(super) fn ctx_draw_rounded_rectangle(
    ctx: &ComPtr<ID2D1DeviceContext5>,
    rect: &D2D1_ROUNDED_RECT,
    brush: *mut ID2D1SolidColorBrush,
    stroke_width: f32,
) {
    unsafe {
        let f: unsafe extern "system" fn(*mut c_void, *const D2D1_ROUNDED_RECT, *mut c_void, f32, *mut c_void) =
            std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 18));
        f(
            ctx.as_ptr().cast::<c_void>(),
            rect,
            brush.cast::<c_void>(),
            stroke_width,
            std::ptr::null_mut(),
        );
    }
}

pub(super) fn ctx_draw_bitmap(
    ctx: &ComPtr<ID2D1DeviceContext5>,
    bitmap: &ComPtr<ID2D1Bitmap1>,
    dest_rect: &D2D1_RECT_F,
    opacity: f32,
) {
    unsafe {
        let f: unsafe extern "system" fn(
            *mut c_void,
            *mut c_void,
            *const D2D1_RECT_F,
            f32,
            u32,
            *const D2D1_RECT_F,
        ) = std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 26));
        f(
            ctx.as_ptr().cast::<c_void>(),
            bitmap.as_ptr().cast::<c_void>(),
            dest_rect,
            opacity,
            D2D1_BITMAP_INTERPOLATION_MODE_LINEAR,
            std::ptr::null(),
        );
    }
}

pub(super) fn ctx_draw_text_layout(
    ctx: &ComPtr<ID2D1DeviceContext5>,
    origin: D2D1_POINT_2F,
    layout: *mut c_void,
    brush: *mut ID2D1SolidColorBrush,
    options: u32,
    measuring_mode: u32,
) {
    unsafe {
        let f: unsafe extern "system" fn(
            *mut c_void,
            D2D1_POINT_2F,
            *mut c_void,
            *mut c_void,
            u32,
            u32,
        ) = std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 28));
        f(
            ctx.as_ptr().cast::<c_void>(),
            origin,
            layout,
            brush.cast::<c_void>(),
            options,
            measuring_mode,
        );
    }
}

pub(super) fn ctx_create_svg_document(
    ctx: &ComPtr<ID2D1DeviceContext5>,
    svg_stream: *mut c_void,
    viewport_size: D2D1_SIZE_F,
) -> Result<ComPtr<ID2D1SvgDocument>, HResultError> {
    let mut doc: *mut ID2D1SvgDocument = std::ptr::null_mut();
    let hr = unsafe {
        let f: unsafe extern "system" fn(*mut c_void, *mut c_void, D2D1_SIZE_F, *mut *mut ID2D1SvgDocument) -> HRESULT =
            std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 115));
        f(ctx.as_ptr().cast::<c_void>(), svg_stream, viewport_size, &mut doc)
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "ID2D1DeviceContext5::CreateSvgDocument failed",
        });
    }
    if doc.is_null() {
        return Err(HResultError {
            hr: -1,
            context: "CreateSvgDocument returned null",
        });
    }
    Ok(ComPtr::from_raw(doc))
}

pub(super) fn ctx_draw_svg_document(ctx: &ComPtr<ID2D1DeviceContext5>, doc: &ComPtr<ID2D1SvgDocument>) {
    unsafe {
        let f: unsafe extern "system" fn(*mut c_void, *mut c_void) =
            std::mem::transmute(vtbl_entry(ctx.as_ptr().cast::<c_void>(), 116));
        f(ctx.as_ptr().cast::<c_void>(), doc.as_ptr().cast::<c_void>());
    }
}

pub(super) fn bitmap_copy_from_bitmap(dst: &ComPtr<ID2D1Bitmap1>, src: &ComPtr<ID2D1Bitmap1>) -> Result<(), HResultError> {
    let hr = unsafe {
        let f: unsafe extern "system" fn(*mut c_void, *const c_void, *mut c_void, *const c_void) -> HRESULT =
            std::mem::transmute(vtbl_entry(dst.as_ptr().cast::<c_void>(), 8));
        f(
            dst.as_ptr().cast::<c_void>(),
            std::ptr::null(),
            src.as_ptr().cast::<c_void>(),
            std::ptr::null(),
        )
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "ID2D1Bitmap::CopyFromBitmap failed",
        });
    }
    Ok(())
}

pub(super) fn bitmap_map(
    bitmap: &ComPtr<ID2D1Bitmap1>,
    options: u32,
) -> Result<D2D1_MAPPED_RECT, HResultError> {
    let mut mapped = D2D1_MAPPED_RECT::default();
    let hr = unsafe {
        let f: unsafe extern "system" fn(*mut c_void, u32, *mut D2D1_MAPPED_RECT) -> HRESULT =
            std::mem::transmute(vtbl_entry(bitmap.as_ptr().cast::<c_void>(), 14));
        f(bitmap.as_ptr().cast::<c_void>(), options, &mut mapped)
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "ID2D1Bitmap1::Map failed",
        });
    }
    Ok(mapped)
}

pub(super) fn bitmap_unmap(bitmap: &ComPtr<ID2D1Bitmap1>) -> Result<(), HResultError> {
    let hr = unsafe {
        let f: unsafe extern "system" fn(*mut c_void) -> HRESULT =
            std::mem::transmute(vtbl_entry(bitmap.as_ptr().cast::<c_void>(), 15));
        f(bitmap.as_ptr().cast::<c_void>())
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "ID2D1Bitmap1::Unmap failed",
        });
    }
    Ok(())
}

unsafe fn vtbl_entry(this: *mut c_void, index: usize) -> *const c_void {
    debug_assert!(!this.is_null());
    unsafe {
        let vtbl = *(this as *mut *const *const c_void);
        *vtbl.add(index)
    }
}
