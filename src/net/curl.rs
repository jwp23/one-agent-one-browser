use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int, c_long};
use std::sync::OnceLock;

type CURLcode = c_int;
type CURLoption = c_int;
type CURLINFO = c_int;

type WriteFn = extern "C" fn(*mut c_char, usize, usize, *mut std::ffi::c_void) -> usize;

#[repr(C)]
struct CURL {
    _private: [u8; 0],
}

const CURLE_OK: CURLcode = 0;

const CURL_GLOBAL_DEFAULT: c_long = 3;

const CURLOPT_URL: CURLoption = 10002;
const CURLOPT_FOLLOWLOCATION: CURLoption = 52;
const CURLOPT_FAILONERROR: CURLoption = 45;
const CURLOPT_WRITEFUNCTION: CURLoption = 20011;
const CURLOPT_WRITEDATA: CURLoption = 10001;
const CURLOPT_USERAGENT: CURLoption = 10018;
const CURLOPT_ACCEPT_ENCODING: CURLoption = 10102;
const CURLOPT_TIMEOUT_MS: CURLoption = 155;
const CURLOPT_CONNECTTIMEOUT_MS: CURLoption = 156;
const CURLOPT_NOSIGNAL: CURLoption = 99;

const CURLINFO_RESPONSE_CODE: CURLINFO = 0x200002;

const MAX_DOWNLOAD_BYTES: usize = 10 * 1024 * 1024;

#[link(name = "curl")]
unsafe extern "C" {
    fn curl_global_init(flags: c_long) -> CURLcode;
    fn curl_easy_init() -> *mut CURL;
    fn curl_easy_cleanup(handle: *mut CURL);
    fn curl_easy_perform(handle: *mut CURL) -> CURLcode;
    fn curl_easy_setopt(handle: *mut CURL, option: CURLoption, ...) -> CURLcode;
    fn curl_easy_getinfo(handle: *mut CURL, info: CURLINFO, ...) -> CURLcode;
    fn curl_easy_strerror(code: CURLcode) -> *const c_char;
}

fn ensure_global_init() -> Result<(), String> {
    static INIT: OnceLock<Result<(), String>> = OnceLock::new();
    INIT.get_or_init(|| {
        let code = unsafe { curl_global_init(CURL_GLOBAL_DEFAULT) };
        if code == CURLE_OK {
            Ok(())
        } else {
            Err(format!("curl_global_init failed: {}", curl_error(code)))
        }
    })
    .clone()
}

pub(super) fn fetch_url_bytes(url: &str) -> Result<Vec<u8>, String> {
    ensure_global_init()?;

    let c_url =
        CString::new(url).map_err(|_| "URL contains an unexpected NUL byte".to_owned())?;

    let handle = unsafe { curl_easy_init() };
    if handle.is_null() {
        return Err("curl_easy_init failed".to_owned());
    }

    let mut buffer: Vec<u8> = Vec::new();
    let mut ctx = WriteContext {
        buffer: &mut buffer,
        max_bytes: MAX_DOWNLOAD_BYTES,
    };

    let user_agent = CString::new("one-agent-one-browser/0.1")
        .map_err(|_| "User-Agent contains an unexpected NUL byte".to_owned())?;
    let accept_encoding =
        CString::new("").map_err(|_| "Accept-Encoding contains an unexpected NUL byte".to_owned())?;

    let _cleanup = CurlHandle(handle);
    setopt_ptr(handle, CURLOPT_URL, c_url.as_ptr())?;
    setopt_long(handle, CURLOPT_FOLLOWLOCATION, 1)?;
    setopt_long(handle, CURLOPT_FAILONERROR, 1)?;
    setopt_long(handle, CURLOPT_TIMEOUT_MS, 15_000)?;
    setopt_long(handle, CURLOPT_CONNECTTIMEOUT_MS, 5_000)?;
    setopt_long(handle, CURLOPT_NOSIGNAL, 1)?;
    setopt_ptr(handle, CURLOPT_USERAGENT, user_agent.as_ptr())?;
    setopt_ptr(handle, CURLOPT_ACCEPT_ENCODING, accept_encoding.as_ptr())?;

    setopt_ptr(
        handle,
        CURLOPT_WRITEDATA,
        (&mut ctx as *mut WriteContext).cast::<std::ffi::c_void>(),
    )?;
    setopt_write_fn(handle, CURLOPT_WRITEFUNCTION, write_callback)?;

    let code = unsafe { curl_easy_perform(handle) };
    if code != CURLE_OK {
        return Err(format!("Failed to fetch {url}: {}", curl_error(code)));
    }

    let response_code = getinfo_long(handle, CURLINFO_RESPONSE_CODE)?;
    if !(200..=399).contains(&response_code) {
        return Err(format!(
            "Unexpected HTTP status {response_code} fetching {url}"
        ));
    }

    Ok(buffer)
}

struct CurlHandle(*mut CURL);

impl Drop for CurlHandle {
    fn drop(&mut self) {
        unsafe { curl_easy_cleanup(self.0) };
    }
}

struct WriteContext<'a> {
    buffer: &'a mut Vec<u8>,
    max_bytes: usize,
}

extern "C" fn write_callback(
    ptr: *mut c_char,
    size: usize,
    nmemb: usize,
    userdata: *mut std::ffi::c_void,
) -> usize {
    let Some(total) = size.checked_mul(nmemb) else {
        return 0;
    };
    if total == 0 {
        return 0;
    }

    let ctx = unsafe { &mut *(userdata.cast::<WriteContext<'_>>()) };
    if ctx.buffer.len().saturating_add(total) > ctx.max_bytes {
        return 0;
    }

    let bytes = unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), total) };
    ctx.buffer.extend_from_slice(bytes);
    total
}

fn setopt_long(handle: *mut CURL, option: CURLoption, value: c_long) -> Result<(), String> {
    let code = unsafe { curl_easy_setopt(handle, option, value) };
    if code == CURLE_OK {
        Ok(())
    } else {
        Err(format!("curl_easy_setopt failed: {}", curl_error(code)))
    }
}

fn setopt_ptr<T>(
    handle: *mut CURL,
    option: CURLoption,
    value: *const T,
) -> Result<(), String> {
    let code = unsafe { curl_easy_setopt(handle, option, value) };
    if code == CURLE_OK {
        Ok(())
    } else {
        Err(format!("curl_easy_setopt failed: {}", curl_error(code)))
    }
}

fn setopt_write_fn(
    handle: *mut CURL,
    option: CURLoption,
    value: WriteFn,
) -> Result<(), String> {
    let code = unsafe { curl_easy_setopt(handle, option, value) };
    if code == CURLE_OK {
        Ok(())
    } else {
        Err(format!("curl_easy_setopt failed: {}", curl_error(code)))
    }
}

fn getinfo_long(handle: *mut CURL, info: CURLINFO) -> Result<i64, String> {
    let mut out: c_long = 0;
    let code = unsafe { curl_easy_getinfo(handle, info, &mut out as *mut c_long) };
    if code == CURLE_OK {
        Ok(out as i64)
    } else {
        Err(format!("curl_easy_getinfo failed: {}", curl_error(code)))
    }
}

fn curl_error(code: CURLcode) -> String {
    let ptr = unsafe { curl_easy_strerror(code) };
    if ptr.is_null() {
        return format!("curl error {code}");
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}
