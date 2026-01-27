use crate::win::com;
use crate::win::com::{ComPtr, GUID, HResultError, HRESULT};
use core::ffi::c_void;

type HMODULE = *mut c_void;
type UINT = u32;

pub(super) enum ID3D11Device {}
pub(super) enum ID3D11DeviceContext {}
pub(super) enum IDXGIDevice {}

const D3D11_SDK_VERSION: UINT = 7;
const D3D11_CREATE_DEVICE_BGRA_SUPPORT: UINT = 0x20;

const D3D_DRIVER_TYPE_HARDWARE: UINT = 1;
const D3D_DRIVER_TYPE_WARP: UINT = 5;

const IID_IDXGI_DEVICE: GUID = GUID {
    data1: 0x54ec_77fa,
    data2: 0x1377,
    data3: 0x44e6,
    data4: [0x8c, 0x32, 0x88, 0xfd, 0x5f, 0x44, 0xc8, 0x4c],
};

#[link(name = "d3d11")]
unsafe extern "system" {
    fn D3D11CreateDevice(
        adapter: *mut c_void,
        driver_type: UINT,
        software: HMODULE,
        flags: UINT,
        feature_levels: *const UINT,
        feature_levels_count: UINT,
        sdk_version: UINT,
        device: *mut *mut ID3D11Device,
        feature_level: *mut UINT,
        immediate_context: *mut *mut ID3D11DeviceContext,
    ) -> HRESULT;
}

pub(super) struct D3DDevices {
    pub(super) dxgi_device: ComPtr<IDXGIDevice>,
}

pub(super) fn create_d3d_devices() -> Result<D3DDevices, String> {
    com::ensure_initialized()?;

    let (d3d_device, _d3d_context) = match try_create_device(D3D_DRIVER_TYPE_HARDWARE) {
        Ok(dev) => dev,
        Err(_) => try_create_device(D3D_DRIVER_TYPE_WARP)
            .map_err(|err| format!("D3D11CreateDevice failed (hardware+WARP): {}", err.message()))?,
    };

    let dxgi_device: ComPtr<IDXGIDevice> = com::query_interface(
        d3d_device.as_ptr().cast::<c_void>(),
        &IID_IDXGI_DEVICE,
        "ID3D11Device::QueryInterface(IDXGIDevice) failed",
    )
    .map_err(|err| err.message())?;

    Ok(D3DDevices { dxgi_device })
}

fn try_create_device(driver_type: UINT) -> Result<(ComPtr<ID3D11Device>, ComPtr<ID3D11DeviceContext>), HResultError> {
    let mut device: *mut ID3D11Device = std::ptr::null_mut();
    let mut context: *mut ID3D11DeviceContext = std::ptr::null_mut();
    let mut feature_level: UINT = 0;

    let hr = unsafe {
        D3D11CreateDevice(
            std::ptr::null_mut(),
            driver_type,
            std::ptr::null_mut(),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            std::ptr::null(),
            0,
            D3D11_SDK_VERSION,
            &mut device,
            &mut feature_level,
            &mut context,
        )
    };
    if !com::succeeded(hr) {
        return Err(HResultError {
            hr,
            context: "D3D11CreateDevice failed",
        });
    }
    if device.is_null() || context.is_null() {
        return Err(HResultError {
            hr: -1,
            context: "D3D11CreateDevice returned null pointers",
        });
    }

    Ok((ComPtr::from_raw(device), ComPtr::from_raw(context)))
}
