#[cfg(not(target_os = "windows"))]
mod curl;
mod pool;
#[cfg(target_os = "windows")]
mod winhttp;

pub use pool::{FetchEvent, FetchPool, RequestId};

pub fn fetch_url_bytes(url: &str) -> Result<Vec<u8>, String> {
    #[cfg(target_os = "windows")]
    return winhttp::fetch_url_bytes(url);

    #[cfg(not(target_os = "windows"))]
    return curl::fetch_url_bytes(url);
}

pub fn fetch_url_text(url: &str) -> Result<String, String> {
    let bytes = fetch_url_bytes(url)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
