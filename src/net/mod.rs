mod curl;

pub fn fetch_url_bytes(url: &str) -> Result<Vec<u8>, String> {
    curl::fetch_url_bytes(url)
}

pub fn fetch_url_text(url: &str) -> Result<String, String> {
    let bytes = fetch_url_bytes(url)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

