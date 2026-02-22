use std::borrow::Cow;

pub(super) fn ensure_xlink_namespace(svg_xml: &str) -> Cow<'_, str> {
    if !svg_xml.contains("xlink:") || svg_xml.contains("xmlns:xlink") {
        return Cow::Borrowed(svg_xml);
    }

    let Some(svg_start) = svg_xml.find("<svg") else {
        return Cow::Borrowed(svg_xml);
    };

    let Some(svg_end) = find_tag_end(svg_xml, svg_start) else {
        return Cow::Borrowed(svg_xml);
    };

    let insert_at = start_tag_insert_pos(svg_xml, svg_start, svg_end);
    let injection = r#" xmlns:xlink="http://www.w3.org/1999/xlink""#;

    let mut out = String::with_capacity(svg_xml.len() + injection.len());
    out.push_str(&svg_xml[..insert_at]);
    out.push_str(injection);
    out.push_str(&svg_xml[insert_at..]);
    Cow::Owned(out)
}

fn find_tag_end(input: &str, start: usize) -> Option<usize> {
    let bytes = input.as_bytes();
    let mut idx = start;
    let mut quote: Option<u8> = None;

    while idx < bytes.len() {
        let b = bytes[idx];
        if let Some(q) = quote {
            if b == q {
                quote = None;
            }
            idx += 1;
            continue;
        }

        match b {
            b'"' | b'\'' => quote = Some(b),
            b'>' => return Some(idx),
            _ => {}
        }
        idx += 1;
    }

    None
}

fn start_tag_insert_pos(input: &str, start: usize, end: usize) -> usize {
    debug_assert!(start <= end);

    let bytes = input.as_bytes();
    let mut idx = end;
    while idx > start && bytes[idx.saturating_sub(1)].is_ascii_whitespace() {
        idx = idx.saturating_sub(1);
    }

    if idx > start && bytes[idx.saturating_sub(1)] == b'/' {
        idx.saturating_sub(1)
    } else {
        end
    }
}
