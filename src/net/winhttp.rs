use crate::url::{Scheme, Url};
use core::ffi::c_void;

type Bool = i32;
type DWORD = u32;
type HInternet = *mut c_void;
type InternetPort = u16;

const TRUE: Bool = 1;

const MAX_DOWNLOAD_BYTES: usize = 10 * 1024 * 1024;
const MAX_REDIRECTS: usize = 10;

const WINHTTP_ACCESS_TYPE_DEFAULT_PROXY: DWORD = 0;

const WINHTTP_FLAG_SECURE: DWORD = 0x0080_0000;

const WINHTTP_OPTION_REDIRECT_POLICY: DWORD = 88;
const WINHTTP_OPTION_REDIRECT_POLICY_NEVER: DWORD = 0;

const WINHTTP_OPTION_DECOMPRESSION: DWORD = 118;
const WINHTTP_DECOMPRESSION_FLAG_GZIP: DWORD = 0x0000_0001;
const WINHTTP_DECOMPRESSION_FLAG_DEFLATE: DWORD = 0x0000_0002;

const WINHTTP_QUERY_STATUS_CODE: DWORD = 19;
const WINHTTP_QUERY_LOCATION: DWORD = 33;
const WINHTTP_QUERY_FLAG_NUMBER: DWORD = 0x2000_0000;

const ERROR_INSUFFICIENT_BUFFER: DWORD = 122;

#[link(name = "winhttp")]
unsafe extern "system" {
    fn WinHttpOpen(
        user_agent: *const u16,
        access_type: DWORD,
        proxy_name: *const u16,
        proxy_bypass: *const u16,
        flags: DWORD,
    ) -> HInternet;
    fn WinHttpCloseHandle(handle: HInternet) -> Bool;
    fn WinHttpConnect(
        session: HInternet,
        server_name: *const u16,
        server_port: InternetPort,
        reserved: DWORD,
    ) -> HInternet;
    fn WinHttpOpenRequest(
        connect: HInternet,
        verb: *const u16,
        object_name: *const u16,
        version: *const u16,
        referrer: *const u16,
        accept_types: *const *const u16,
        flags: DWORD,
    ) -> HInternet;
    fn WinHttpSendRequest(
        request: HInternet,
        headers: *const u16,
        headers_len: DWORD,
        optional: *mut c_void,
        optional_len: DWORD,
        total_len: DWORD,
        context: usize,
    ) -> Bool;
    fn WinHttpReceiveResponse(request: HInternet, reserved: *mut c_void) -> Bool;
    fn WinHttpQueryHeaders(
        request: HInternet,
        info_level: DWORD,
        name: *const u16,
        buffer: *mut c_void,
        buffer_len: *mut DWORD,
        index: *mut DWORD,
    ) -> Bool;
    fn WinHttpQueryDataAvailable(request: HInternet, available: *mut DWORD) -> Bool;
    fn WinHttpReadData(
        request: HInternet,
        buffer: *mut c_void,
        bytes_to_read: DWORD,
        bytes_read: *mut DWORD,
    ) -> Bool;
    fn WinHttpSetTimeouts(
        handle: HInternet,
        resolve_timeout_ms: i32,
        connect_timeout_ms: i32,
        send_timeout_ms: i32,
        receive_timeout_ms: i32,
    ) -> Bool;
    fn WinHttpSetOption(handle: HInternet, option: DWORD, buffer: *const c_void, size: DWORD)
        -> Bool;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetLastError() -> DWORD;
    fn FormatMessageW(
        flags: DWORD,
        source: *const c_void,
        message_id: DWORD,
        language_id: DWORD,
        buffer: *mut u16,
        size: DWORD,
        arguments: *const c_void,
    ) -> DWORD;
}

pub(super) fn fetch_url_bytes(url: &str) -> Result<Vec<u8>, String> {
    let mut current =
        Url::parse(url).map_err(|err| format!("Invalid URL {url:?}: {err}"))?;

    let session = WinHttpHandle::open("one-agent-one-browser/0.1")?;
    session.set_timeouts(5_000, 5_000, 15_000, 15_000)?;

    for redirect in 0..=MAX_REDIRECTS {
        let response = fetch_once(&session, &current)?;

        if is_redirect_status(response.status_code) {
            if redirect == MAX_REDIRECTS {
                return Err(format!("Too many redirects fetching {}", current.as_str()));
            }

            let location = response
                .location
                .ok_or_else(|| format!("Redirect without Location header fetching {}", current.as_str()))?;
            let next = current
                .resolve(location.trim())
                .ok_or_else(|| format!("Failed to resolve redirect {location:?} from {}", current.as_str()))?;
            current = next;
            continue;
        }

        if (200..=399).contains(&response.status_code) {
            return Ok(response.body);
        }

        return Err(format!(
            "Unexpected HTTP status {} fetching {}",
            response.status_code,
            current.as_str()
        ));
    }

    Err(format!("Too many redirects fetching {}", current.as_str()))
}

struct FetchResponse {
    status_code: u32,
    location: Option<String>,
    body: Vec<u8>,
}

fn fetch_once(session: &WinHttpHandle, url: &Url) -> Result<FetchResponse, String> {
    let host = url.host();
    let host_w = wide_null_terminated(host);
    let path_w = wide_null_terminated(url.path_and_query());
    let verb_w = wide_null_terminated("GET");

    let port = url.port().unwrap_or_else(|| match url.scheme() {
        Scheme::Http => 80,
        Scheme::Https => 443,
    });

    let connect = session.connect(&host_w, port)?;
    let mut request_flags: DWORD = 0;
    if url.scheme() == Scheme::Https {
        request_flags |= WINHTTP_FLAG_SECURE;
    }

    let request = connect.open_request(&verb_w, &path_w, request_flags)?;

    request.set_redirect_policy_never()?;

    if !request.enable_decompression()? {
        // Ensure we can still parse text payloads by opting out of compression.
        request.send(Some("Accept-Encoding: identity\r\n"))?;
    } else {
        request.send(None)?;
    }
    request.receive_response()?;

    let status_code = request.query_status_code()?;
    let location = if is_redirect_status(status_code) {
        request.query_header_string(WINHTTP_QUERY_LOCATION)?
    } else {
        None
    };

    let body = if is_redirect_status(status_code) {
        Vec::new()
    } else {
        request.read_to_end(MAX_DOWNLOAD_BYTES)?
    };

    Ok(FetchResponse {
        status_code,
        location,
        body,
    })
}

fn is_redirect_status(status: u32) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

struct WinHttpHandle(HInternet);

impl WinHttpHandle {
    fn open(user_agent: &str) -> Result<Self, String> {
        let ua_w = wide_null_terminated(user_agent);
        let handle = unsafe {
            WinHttpOpen(
                ua_w.as_ptr(),
                WINHTTP_ACCESS_TYPE_DEFAULT_PROXY,
                std::ptr::null(),
                std::ptr::null(),
                0,
            )
        };
        if handle.is_null() {
            return Err(format!(
                "WinHttpOpen failed: {}",
                win32_error_message(last_error())
            ));
        }
        Ok(Self(handle))
    }

    fn set_timeouts(
        &self,
        resolve_timeout_ms: i32,
        connect_timeout_ms: i32,
        send_timeout_ms: i32,
        receive_timeout_ms: i32,
    ) -> Result<(), String> {
        let ok = unsafe {
            WinHttpSetTimeouts(
                self.0,
                resolve_timeout_ms,
                connect_timeout_ms,
                send_timeout_ms,
                receive_timeout_ms,
            )
        };
        if ok == TRUE {
            Ok(())
        } else {
            Err(format!(
                "WinHttpSetTimeouts failed: {}",
                win32_error_message(last_error())
            ))
        }
    }

    fn connect(&self, host: &[u16], port: u16) -> Result<WinHttpConnection, String> {
        let handle = unsafe { WinHttpConnect(self.0, host.as_ptr(), port, 0) };
        if handle.is_null() {
            return Err(format!(
                "WinHttpConnect failed: {}",
                win32_error_message(last_error())
            ));
        }
        Ok(WinHttpConnection(WinHttpHandle(handle)))
    }
}

impl Drop for WinHttpHandle {
    fn drop(&mut self) {
        if self.0.is_null() {
            return;
        }
        unsafe {
            let _ = WinHttpCloseHandle(self.0);
        }
    }
}

struct WinHttpConnection(WinHttpHandle);

impl WinHttpConnection {
    fn open_request(
        &self,
        verb: &[u16],
        path: &[u16],
        flags: DWORD,
    ) -> Result<WinHttpRequest, String> {
        let handle = unsafe {
            WinHttpOpenRequest(
                self.0 .0,
                verb.as_ptr(),
                path.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                flags,
            )
        };
        if handle.is_null() {
            return Err(format!(
                "WinHttpOpenRequest failed: {}",
                win32_error_message(last_error())
            ));
        }
        Ok(WinHttpRequest(WinHttpHandle(handle)))
    }
}

struct WinHttpRequest(WinHttpHandle);

impl WinHttpRequest {
    fn set_redirect_policy_never(&self) -> Result<(), String> {
        let policy: DWORD = WINHTTP_OPTION_REDIRECT_POLICY_NEVER;
        let ok = unsafe {
            WinHttpSetOption(
                self.0 .0,
                WINHTTP_OPTION_REDIRECT_POLICY,
                (&policy as *const DWORD).cast::<c_void>(),
                std::mem::size_of::<DWORD>() as DWORD,
            )
        };
        if ok == TRUE {
            Ok(())
        } else {
            Err(format!(
                "WinHttpSetOption(WINHTTP_OPTION_REDIRECT_POLICY) failed: {}",
                win32_error_message(last_error())
            ))
        }
    }

    fn enable_decompression(&self) -> Result<bool, String> {
        let flags: DWORD = WINHTTP_DECOMPRESSION_FLAG_GZIP | WINHTTP_DECOMPRESSION_FLAG_DEFLATE;
        let ok = unsafe {
            WinHttpSetOption(
                self.0 .0,
                WINHTTP_OPTION_DECOMPRESSION,
                (&flags as *const DWORD).cast::<c_void>(),
                std::mem::size_of::<DWORD>() as DWORD,
            )
        };
        if ok == TRUE {
            Ok(true)
        } else {
            // Not supported on older versions / configs. Fall back to requesting identity encoding.
            Ok(false)
        }
    }

    fn send(&self, additional_headers: Option<&str>) -> Result<(), String> {
        let (headers_ptr, headers_len) = if let Some(headers) = additional_headers {
            let headers_w = wide_null_terminated(headers);
            let len_chars: usize = headers_w.len().saturating_sub(1);
            let len: DWORD = len_chars
                .try_into()
                .map_err(|_| "Request headers too long".to_owned())?;
            (Some(headers_w), len)
        } else {
            (None, 0)
        };

        let headers_ptr = headers_ptr
            .as_ref()
            .map(|v| v.as_ptr())
            .unwrap_or_else(std::ptr::null);

        let ok = unsafe {
            WinHttpSendRequest(
                self.0 .0,
                headers_ptr,
                headers_len,
                std::ptr::null_mut(),
                0,
                0,
                0,
            )
        };
        if ok == TRUE {
            Ok(())
        } else {
            Err(format!(
                "WinHttpSendRequest failed: {}",
                win32_error_message(last_error())
            ))
        }
    }

    fn receive_response(&self) -> Result<(), String> {
        let ok = unsafe { WinHttpReceiveResponse(self.0 .0, std::ptr::null_mut()) };
        if ok == TRUE {
            Ok(())
        } else {
            Err(format!(
                "WinHttpReceiveResponse failed: {}",
                win32_error_message(last_error())
            ))
        }
    }

    fn query_status_code(&self) -> Result<u32, String> {
        let mut status: DWORD = 0;
        let mut len: DWORD = std::mem::size_of::<DWORD>() as DWORD;
        let ok = unsafe {
            WinHttpQueryHeaders(
                self.0 .0,
                WINHTTP_QUERY_STATUS_CODE | WINHTTP_QUERY_FLAG_NUMBER,
                std::ptr::null(),
                (&mut status as *mut DWORD).cast::<c_void>(),
                &mut len,
                std::ptr::null_mut(),
            )
        };
        if ok == TRUE {
            Ok(status)
        } else {
            Err(format!(
                "WinHttpQueryHeaders(status_code) failed: {}",
                win32_error_message(last_error())
            ))
        }
    }

    fn query_header_string(&self, info_level: DWORD) -> Result<Option<String>, String> {
        let mut needed_bytes: DWORD = 0;
        let ok = unsafe {
            WinHttpQueryHeaders(
                self.0 .0,
                info_level,
                std::ptr::null(),
                std::ptr::null_mut(),
                &mut needed_bytes,
                std::ptr::null_mut(),
            )
        };

        if ok == TRUE {
            return Ok(Some(String::new()));
        }

        let err = last_error();
        if err != ERROR_INSUFFICIENT_BUFFER {
            return Ok(None);
        }

        let needed_u16 = needed_bytes
            .checked_add(1)
            .and_then(|n| n.checked_div(2))
            .ok_or_else(|| "Header buffer size overflow".to_owned())?;
        let mut buf: Vec<u16> = vec![0u16; needed_u16 as usize];
        let mut buf_len_bytes: DWORD = (buf.len() * 2)
            .try_into()
            .map_err(|_| "Header buffer too large".to_owned())?;

        let ok = unsafe {
            WinHttpQueryHeaders(
                self.0 .0,
                info_level,
                std::ptr::null(),
                buf.as_mut_ptr().cast::<c_void>(),
                &mut buf_len_bytes,
                std::ptr::null_mut(),
            )
        };
        if ok != TRUE {
            return Err(format!(
                "WinHttpQueryHeaders(header {info_level}) failed: {}",
                win32_error_message(last_error())
            ));
        }

        let len_u16 = (buf_len_bytes as usize) / 2;
        buf.truncate(len_u16);
        while buf.last() == Some(&0) {
            buf.pop();
        }
        Ok(Some(String::from_utf16_lossy(&buf)))
    }

    fn read_to_end(&self, max_bytes: usize) -> Result<Vec<u8>, String> {
        let mut out: Vec<u8> = Vec::new();
        loop {
            let mut available: DWORD = 0;
            let ok = unsafe { WinHttpQueryDataAvailable(self.0 .0, &mut available) };
            if ok != TRUE {
                return Err(format!(
                    "WinHttpQueryDataAvailable failed: {}",
                    win32_error_message(last_error())
                ));
            }
            if available == 0 {
                break;
            }

            let chunk_len: usize = available
                .try_into()
                .map_err(|_| "Response chunk size out of range".to_owned())?;
            if out.len().saturating_add(chunk_len) > max_bytes {
                return Err(format!(
                    "Response exceeds maximum size ({max_bytes} bytes)"
                ));
            }

            let mut chunk = vec![0u8; chunk_len];
            let mut read: DWORD = 0;
            let ok = unsafe {
                WinHttpReadData(
                    self.0 .0,
                    chunk.as_mut_ptr().cast::<c_void>(),
                    available,
                    &mut read,
                )
            };
            if ok != TRUE {
                return Err(format!(
                    "WinHttpReadData failed: {}",
                    win32_error_message(last_error())
                ));
            }
            let read_usize: usize = read
                .try_into()
                .map_err(|_| "Response read size out of range".to_owned())?;
            chunk.truncate(read_usize);
            out.extend_from_slice(&chunk);
        }
        Ok(out)
    }
}

fn wide_null_terminated(input: &str) -> Vec<u16> {
    let mut out: Vec<u16> = input.encode_utf16().collect();
    out.push(0);
    out
}

fn last_error() -> DWORD {
    unsafe { GetLastError() }
}

fn win32_error_message(code: DWORD) -> String {
    const FORMAT_MESSAGE_FROM_SYSTEM: DWORD = 0x0000_1000;
    const FORMAT_MESSAGE_IGNORE_INSERTS: DWORD = 0x0000_0200;

    if code == 0 {
        return "ok".to_owned();
    }

    let mut buf = [0u16; 512];
    let len = unsafe {
        FormatMessageW(
            FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
            std::ptr::null(),
            code,
            0,
            buf.as_mut_ptr(),
            buf.len() as DWORD,
            std::ptr::null(),
        )
    };

    if len == 0 {
        return format!("Windows error {code}");
    }

    let message = String::from_utf16_lossy(&buf[..len as usize]);
    message.trim().to_owned()
}
