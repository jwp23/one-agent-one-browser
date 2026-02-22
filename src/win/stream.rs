use super::com;
use super::com::{ComPtr, HResultError};
use core::ffi::c_void;

pub(crate) enum IStream {}

type HGLOBAL = *mut c_void;

const GMEM_MOVEABLE: u32 = 0x0002;
const TRUE: i32 = 1;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GlobalAlloc(flags: u32, bytes: usize) -> HGLOBAL;
    fn GlobalLock(mem: HGLOBAL) -> *mut c_void;
    fn GlobalUnlock(mem: HGLOBAL) -> i32;
    fn GlobalFree(mem: HGLOBAL) -> HGLOBAL;
}

#[link(name = "ole32")]
unsafe extern "system" {
    fn CreateStreamOnHGlobal(
        mem: HGLOBAL,
        delete_on_release: i32,
        stream: *mut *mut IStream,
    ) -> com::HRESULT;
}

pub(crate) fn create_istream_from_bytes(bytes: &[u8]) -> Result<ComPtr<IStream>, HResultError> {
    let mem = unsafe { GlobalAlloc(GMEM_MOVEABLE, bytes.len()) };
    if mem.is_null() {
        return Err(HResultError {
            hr: -1,
            context: "GlobalAlloc failed",
        });
    }

    let ptr = unsafe { GlobalLock(mem) };
    if ptr.is_null() {
        unsafe {
            let _ = GlobalFree(mem);
        }
        return Err(HResultError {
            hr: -1,
            context: "GlobalLock failed",
        });
    }
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.cast::<u8>(), bytes.len());
        let _ = GlobalUnlock(mem);
    }

    let mut stream: *mut IStream = std::ptr::null_mut();
    let hr = unsafe { CreateStreamOnHGlobal(mem, TRUE, &mut stream) };
    if !com::succeeded(hr) {
        unsafe {
            let _ = GlobalFree(mem);
        }
        return Err(HResultError {
            hr,
            context: "CreateStreamOnHGlobal failed",
        });
    }
    if stream.is_null() {
        return Err(HResultError {
            hr: -1,
            context: "CreateStreamOnHGlobal returned null",
        });
    }

    Ok(ComPtr::from_raw(stream))
}
