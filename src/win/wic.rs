use super::com::{self, ComPtr, GUID, HRESULT, HResultError};
use crate::debug;
use crate::image::Argb32Image;
use core::ffi::c_void;
use super::stream::IStream;

type UINT = u32;

const WINCODEC_ERR_COMPONENTNOTFOUND: HRESULT = 0x8898_2f50u32 as i32;

const WICBITMAPDITHERTYPENONE: u32 = 0;
const WICBITMAPPALETTETYPECUSTOM: u32 = 0;
const WICDECODEOPTIONS_METADATA_CACHE_ON_DEMAND: u32 = 0;

#[repr(C)]
struct IWICImagingFactory {
    vtbl: *const IWICImagingFactoryVtbl,
}

#[repr(C)]
struct IWICBitmapDecoder {
    vtbl: *const IWICBitmapDecoderVtbl,
}

#[repr(C)]
struct IWICBitmapSource {
    vtbl: *const IWICBitmapSourceVtbl,
}

#[repr(C)]
struct IWICFormatConverter {
    vtbl: *const IWICFormatConverterVtbl,
}

#[repr(C)]
struct IWICBitmapFrameDecode {
    vtbl: *const IWICBitmapSourceVtbl,
}

#[repr(C)]
struct IWICImagingFactoryVtbl {
    // IUnknown
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,

    // IWICImagingFactory
    create_decoder_from_filename: *const c_void,
    create_decoder_from_stream: unsafe extern "system" fn(
        *mut c_void,
        *mut IStream,
        *const GUID,
        u32,
        *mut *mut IWICBitmapDecoder,
    ) -> HRESULT,
    create_decoder_from_file_handle: *const c_void,
    create_component_info: *const c_void,
    create_decoder: *const c_void,
    create_encoder: *const c_void,
    create_palette: *const c_void,
    create_format_converter:
        unsafe extern "system" fn(*mut c_void, *mut *mut IWICFormatConverter) -> HRESULT,
}

#[repr(C)]
struct IWICBitmapDecoderVtbl {
    // IUnknown
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,

    // IWICBitmapDecoder
    query_capability: *const c_void,
    initialize: *const c_void,
    get_container_format: *const c_void,
    get_decoder_info: *const c_void,
    copy_palette: *const c_void,
    get_metadata_query_reader: *const c_void,
    get_preview: *const c_void,
    get_color_contexts: *const c_void,
    get_thumbnail: *const c_void,
    get_frame_count: *const c_void,
    get_frame:
        unsafe extern "system" fn(*mut c_void, UINT, *mut *mut IWICBitmapFrameDecode) -> HRESULT,
}

#[repr(C)]
struct IWICBitmapSourceVtbl {
    // IUnknown
    query_interface:
        unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    release: unsafe extern "system" fn(*mut c_void) -> u32,

    // IWICBitmapSource
    get_size: unsafe extern "system" fn(*mut c_void, *mut UINT, *mut UINT) -> HRESULT,
    get_pixel_format: *const c_void,
    get_resolution: *const c_void,
    copy_palette: *const c_void,
    copy_pixels: unsafe extern "system" fn(*mut c_void, *const c_void, UINT, UINT, *mut u8) -> HRESULT,
}

#[repr(C)]
struct IWICFormatConverterVtbl {
    // IWICBitmapSource (includes IUnknown)
    base: IWICBitmapSourceVtbl,

    // IWICFormatConverter
    initialize: unsafe extern "system" fn(
        *mut c_void,
        *mut IWICBitmapSource,
        *const GUID,
        u32,
        *mut c_void,
        f64,
        u32,
    ) -> HRESULT,
    can_convert: *const c_void,
}

const CLSID_WIC_IMAGING_FACTORY: GUID = GUID {
    data1: 0x317d06e8,
    data2: 0x5f24,
    data3: 0x433d,
    data4: [0xbd, 0xf7, 0x79, 0xce, 0x68, 0xd8, 0xab, 0xc2],
};

const IID_IWIC_IMAGING_FACTORY: GUID = GUID {
    data1: 0xec5ec8a9,
    data2: 0xc395,
    data3: 0x4314,
    data4: [0x9c, 0x77, 0x54, 0xd7, 0xa9, 0x35, 0xff, 0x70],
};

const GUID_WIC_PIXEL_FORMAT_32BPP_PBGRA: GUID = GUID {
    data1: 0x6fddc324,
    data2: 0x4e03,
    data3: 0x4bfe,
    data4: [0xb1, 0x85, 0x3d, 0x77, 0x76, 0x8d, 0xc9, 0x10],
};

pub(crate) fn decode_png_argb32(bytes: &[u8]) -> Result<Argb32Image, String> {
    decode_wic_argb32(bytes).map_err(|err| err.message())
}

pub(crate) fn decode_jpeg_argb32(bytes: &[u8]) -> Result<Argb32Image, String> {
    decode_wic_argb32(bytes).map_err(|err| err.message())
}

pub(crate) fn decode_webp_argb32(bytes: &[u8]) -> Result<Argb32Image, String> {
    match decode_wic_argb32(bytes) {
        Ok(image) => Ok(image),
        Err(err) if err.hr == WINCODEC_ERR_COMPONENTNOTFOUND => {
            let message = "WebP decode failed: a WIC WebP codec is not installed. Install \"WebP Image Extensions\" from the Microsoft Store to enable WebP rendering.";
            debug::log(
                debug::Target::Render,
                debug::Level::Warn,
                format_args!("{message}"),
            );
            Err(message.to_owned())
        }
        Err(err) => Err(err.message()),
    }
}

fn decode_wic_argb32(bytes: &[u8]) -> Result<Argb32Image, HResultError> {
    if bytes.is_empty() {
        return Err(HResultError {
            hr: -1,
            context: "WIC decode failed (empty buffer)",
        });
    }

    com::ensure_initialized().map_err(|_| HResultError {
        hr: -1,
        context: "CoInitializeEx failed",
    })?;

    let stream = super::stream::create_istream_from_bytes(bytes)?;

    let factory: ComPtr<IWICImagingFactory> = com::co_create_instance(
        &CLSID_WIC_IMAGING_FACTORY,
        &IID_IWIC_IMAGING_FACTORY,
        "CoCreateInstance(CLSID_WICImagingFactory)",
    )?;

    let decoder = create_decoder_from_stream(&factory, &stream)?;
    let frame = decoder_get_frame(&decoder, 0)?;
    let converter = factory_create_format_converter(&factory)?;

    converter_initialize(&converter, &frame, &GUID_WIC_PIXEL_FORMAT_32BPP_PBGRA)?;

    let (width, height) = bitmap_source_get_size(converter.as_ptr().cast::<IWICBitmapSource>())?;
    if width == 0 || height == 0 {
        return Err(HResultError {
            hr: -1,
            context: "Decoded image had invalid dimensions",
        });
    }

    let stride = width
        .checked_mul(4)
        .ok_or(HResultError {
            hr: -1,
            context: "Decoded image row stride overflow",
        })? as usize;
    let len = stride.checked_mul(height as usize).ok_or(HResultError {
        hr: -1,
        context: "Decoded image buffer size overflow",
    })?;
    let mut bgra = vec![0u8; len];

    bitmap_source_copy_pixels(converter.as_ptr().cast::<IWICBitmapSource>(), stride as u32, &mut bgra)?;

    Argb32Image::new(width, height, bgra).map_err(|_| HResultError {
        hr: -1,
        context: "Decoded image buffer validation failed",
    })
}

fn create_decoder_from_stream(
    factory: &ComPtr<IWICImagingFactory>,
    stream: &ComPtr<IStream>,
) -> Result<ComPtr<IWICBitmapDecoder>, HResultError> {
    let mut decoder: *mut IWICBitmapDecoder = std::ptr::null_mut();
    let hr = unsafe {
        ((*(*factory.as_ptr()).vtbl).create_decoder_from_stream)(
            factory.as_ptr().cast::<c_void>(),
            stream.as_ptr(),
            std::ptr::null(),
            WICDECODEOPTIONS_METADATA_CACHE_ON_DEMAND,
            &mut decoder,
        )
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "IWICImagingFactory::CreateDecoderFromStream failed",
        });
    }
    Ok(ComPtr::from_raw(decoder))
}

fn decoder_get_frame(
    decoder: &ComPtr<IWICBitmapDecoder>,
    index: u32,
) -> Result<ComPtr<IWICBitmapFrameDecode>, HResultError> {
    let mut frame: *mut IWICBitmapFrameDecode = std::ptr::null_mut();
    let hr = unsafe {
        ((*(*decoder.as_ptr()).vtbl).get_frame)(decoder.as_ptr().cast::<c_void>(), index, &mut frame)
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "IWICBitmapDecoder::GetFrame failed",
        });
    }
    Ok(ComPtr::from_raw(frame))
}

fn factory_create_format_converter(
    factory: &ComPtr<IWICImagingFactory>,
) -> Result<ComPtr<IWICFormatConverter>, HResultError> {
    let mut converter: *mut IWICFormatConverter = std::ptr::null_mut();
    let hr = unsafe {
        ((*(*factory.as_ptr()).vtbl).create_format_converter)(
            factory.as_ptr().cast::<c_void>(),
            &mut converter,
        )
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "IWICImagingFactory::CreateFormatConverter failed",
        });
    }
    Ok(ComPtr::from_raw(converter))
}

fn converter_initialize(
    converter: &ComPtr<IWICFormatConverter>,
    source: &ComPtr<IWICBitmapFrameDecode>,
    dst_format: &GUID,
) -> Result<(), HResultError> {
    let hr = unsafe {
        ((*(*converter.as_ptr()).vtbl).initialize)(
            converter.as_ptr().cast::<c_void>(),
            source.as_ptr().cast::<IWICBitmapSource>(),
            dst_format as *const GUID,
            WICBITMAPDITHERTYPENONE,
            std::ptr::null_mut(),
            0.0,
            WICBITMAPPALETTETYPECUSTOM,
        )
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "IWICFormatConverter::Initialize failed",
        });
    }
    Ok(())
}

fn bitmap_source_get_size(source: *mut IWICBitmapSource) -> Result<(u32, u32), HResultError> {
    let mut w: UINT = 0;
    let mut h: UINT = 0;
    let hr = unsafe {
        ((*(*source).vtbl).get_size)(
            source.cast::<c_void>(),
            &mut w as *mut UINT,
            &mut h as *mut UINT,
        )
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "IWICBitmapSource::GetSize failed",
        });
    }
    Ok((w, h))
}

fn bitmap_source_copy_pixels(
    source: *mut IWICBitmapSource,
    stride: u32,
    dest: &mut Vec<u8>,
) -> Result<(), HResultError> {
    let buffer_len: u32 = dest.len().try_into().map_err(|_| HResultError {
        hr: -1,
        context: "Decoded image buffer too large",
    })?;

    let hr = unsafe {
        ((*(*source).vtbl).copy_pixels)(
            source.cast::<c_void>(),
            std::ptr::null(),
            stride,
            buffer_len,
            dest.as_mut_ptr(),
        )
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "IWICBitmapSource::CopyPixels failed",
        });
    }
    Ok(())
}
