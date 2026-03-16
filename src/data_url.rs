pub fn decode_bytes(url: &str) -> Result<Vec<u8>, String> {
    let payload = url
        .trim()
        .strip_prefix("data:")
        .ok_or_else(|| "Not a data URL".to_owned())?;
    let (meta, data) = payload
        .split_once(',')
        .ok_or_else(|| "Invalid data URL: missing comma".to_owned())?;

    if meta
        .split(';')
        .any(|part| part.eq_ignore_ascii_case("base64"))
    {
        return decode_base64(data);
    }

    percent_decode(data)
}

fn percent_decode(data: &str) -> Result<Vec<u8>, String> {
    let bytes = data.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut idx = 0usize;

    while idx < bytes.len() {
        match bytes[idx] {
            b'%' => {
                if idx + 2 >= bytes.len() {
                    return Err("Invalid data URL: incomplete percent escape".to_owned());
                }
                let hi = decode_hex(bytes[idx + 1])?;
                let lo = decode_hex(bytes[idx + 2])?;
                out.push((hi << 4) | lo);
                idx += 3;
            }
            byte => {
                out.push(byte);
                idx += 1;
            }
        }
    }

    Ok(out)
}

fn decode_hex(byte: u8) -> Result<u8, String> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err("Invalid data URL: bad hex escape".to_owned()),
    }
}

fn decode_base64(data: &str) -> Result<Vec<u8>, String> {
    let filtered: Vec<u8> = data
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    if filtered.len() % 4 != 0 {
        return Err("Invalid data URL: malformed base64 payload".to_owned());
    }

    let mut out = Vec::with_capacity(filtered.len() / 4 * 3);
    for chunk in filtered.chunks(4) {
        let a = decode_base64_char(chunk[0])?;
        let b = decode_base64_char(chunk[1])?;
        let c = if chunk[2] == b'=' {
            64
        } else {
            decode_base64_char(chunk[2])?
        };
        let d = if chunk[3] == b'=' {
            64
        } else {
            decode_base64_char(chunk[3])?
        };

        out.push((a << 2) | (b >> 4));
        if c != 64 {
            out.push(((b & 0x0f) << 4) | (c >> 2));
        }
        if d != 64 {
            out.push(((c & 0x03) << 6) | d);
        }
    }

    Ok(out)
}

fn decode_base64_char(byte: u8) -> Result<u8, String> {
    match byte {
        b'A'..=b'Z' => Ok(byte - b'A'),
        b'a'..=b'z' => Ok(byte - b'a' + 26),
        b'0'..=b'9' => Ok(byte - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err("Invalid data URL: bad base64 character".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::decode_bytes;

    #[test]
    fn decodes_percent_encoded_data_url() {
        let decoded = decode_bytes("data:image/svg+xml,%3Csvg%3Eok%3C/svg%3E").unwrap();
        assert_eq!(decoded, b"<svg>ok</svg>");
    }

    #[test]
    fn decodes_base64_data_url() {
        let decoded = decode_bytes("data:text/plain;base64,aGVsbG8=").unwrap();
        assert_eq!(decoded, b"hello");
    }
}
