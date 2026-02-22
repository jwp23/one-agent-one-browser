use core::ffi::c_void;
use std::cell::RefCell;

pub(crate) type HRESULT = i32;

pub(crate) const S_OK: HRESULT = 0;
pub(crate) const S_FALSE: HRESULT = 1;
pub(crate) const RPC_E_CHANGED_MODE: HRESULT = 0x8001_0106u32 as i32;

const COINIT_MULTITHREADED: u32 = 0x0;

#[repr(C)]
#[derive(Clone, Copy)]
pub(crate) struct GUID {
    pub data1: u32,
    pub data2: u16,
    pub data3: u16,
    pub data4: [u8; 8],
}

#[repr(C)]
pub(crate) struct IUnknown {
    vtbl: *const IUnknownVtbl,
}

#[repr(C)]
pub(crate) struct IUnknownVtbl {
    pub query_interface:
        unsafe extern "system" fn(*mut c_void, *const GUID, *mut *mut c_void) -> HRESULT,
    pub add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[link(name = "ole32")]
unsafe extern "system" {
    fn CoInitializeEx(reserved: *mut c_void, coinit: u32) -> HRESULT;
    fn CoUninitialize();
    fn CoCreateInstance(
        clsid: *const GUID,
        outer: *mut c_void,
        context: u32,
        iid: *const GUID,
        out: *mut *mut c_void,
    ) -> HRESULT;
}

const CLSCTX_INPROC_SERVER: u32 = 0x1;

struct ComThreadGuard {
    needs_uninit: bool,
}

impl Drop for ComThreadGuard {
    fn drop(&mut self) {
        if !self.needs_uninit {
            return;
        }
        unsafe { CoUninitialize() };
    }
}

thread_local! {
    static COM_GUARD: RefCell<Option<ComThreadGuard>> = const { RefCell::new(None) };
}

pub(crate) fn ensure_initialized() -> Result<(), String> {
    COM_GUARD.with(|guard| {
        if guard.borrow().is_some() {
            return Ok(());
        }

        let hr = unsafe { CoInitializeEx(std::ptr::null_mut(), COINIT_MULTITHREADED) };
        let needs_uninit = match hr {
            S_OK | S_FALSE => true,
            RPC_E_CHANGED_MODE => false,
            other => return Err(format!("CoInitializeEx failed: {}", hresult_string(other))),
        };

        *guard.borrow_mut() = Some(ComThreadGuard { needs_uninit });
        Ok(())
    })
}

pub(crate) fn co_create_instance<T>(
    clsid: &GUID,
    iid: &GUID,
    context: &'static str,
) -> Result<ComPtr<T>, HResultError> {
    let mut out: *mut c_void = std::ptr::null_mut();
    let hr = unsafe {
        CoCreateInstance(
            clsid as *const GUID,
            std::ptr::null_mut(),
            CLSCTX_INPROC_SERVER,
            iid as *const GUID,
            &mut out,
        )
    };
    if !succeeded(hr) {
        return Err(HResultError { hr, context });
    }
    Ok(ComPtr::from_raw(out.cast::<T>()))
}

pub(crate) fn query_interface<T>(
    object: *mut c_void,
    iid: &GUID,
    context: &'static str,
) -> Result<ComPtr<T>, HResultError> {
    if object.is_null() {
        return Err(HResultError { hr: -1, context });
    }

    let mut out: *mut c_void = std::ptr::null_mut();
    let hr = unsafe {
        let unknown = object.cast::<IUnknown>();
        ((*(*unknown).vtbl).query_interface)(object, iid as *const GUID, &mut out)
    };
    if !succeeded(hr) {
        return Err(HResultError { hr, context });
    }
    Ok(ComPtr::from_raw(out.cast::<T>()))
}

pub(crate) fn succeeded(hr: HRESULT) -> bool {
    hr >= 0
}

pub(crate) fn hresult_string(hr: HRESULT) -> String {
    format!("HRESULT 0x{:08x}", hr as u32)
}

#[derive(Clone, Debug)]
pub(crate) struct HResultError {
    pub hr: HRESULT,
    pub context: &'static str,
}

impl HResultError {
    pub(crate) fn message(&self) -> String {
        format!("{}: {}", self.context, hresult_string(self.hr))
    }
}

pub(crate) struct ComPtr<T> {
    ptr: *mut T,
}

impl<T> ComPtr<T> {
    pub(crate) fn from_raw(ptr: *mut T) -> Self {
        Self { ptr }
    }

    pub(crate) fn as_ptr(&self) -> *mut T {
        self.ptr
    }
}

impl<T> Drop for ComPtr<T> {
    fn drop(&mut self) {
        if self.ptr.is_null() {
            return;
        }
        unsafe {
            let unknown = self.ptr.cast::<IUnknown>();
            let vtbl = (*unknown).vtbl;
            let _ = ((*vtbl).release)(self.ptr.cast::<c_void>());
        }
        self.ptr = std::ptr::null_mut();
    }
}

impl<T> std::fmt::Debug for ComPtr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComPtr").field("ptr", &self.ptr).finish()
    }
}
