use super::parse::{parse_css_length_px_f32_with_viewport, parse_css_length_px_with_viewport};

#[derive(Clone, Copy, Debug)]
pub enum CssLength {
    Px(i32),
    Percent(f32),
    Calc { percent: f32, px: f32 },
}

impl CssLength {
    pub fn resolve_px(self, reference_px: i32) -> i32 {
        let reference_px = reference_px.max(0);
        match self {
            CssLength::Px(px) => px,
            CssLength::Percent(percent) => ((reference_px as f32) * (percent / 100.0)).round() as i32,
            CssLength::Calc { percent, px } => {
                ((reference_px as f32) * (percent / 100.0) + px).round() as i32
            }
        }
    }
}

pub(super) fn parse_css_length(
    value: &str,
    viewport_width_px: Option<i32>,
    viewport_height_px: Option<i32>,
) -> Option<CssLength> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if let Some(calc) = parse_css_calc_length(value, viewport_width_px, viewport_height_px) {
        return Some(calc);
    }

    if let Some(percent) = value.strip_suffix('%') {
        let percent = percent.trim();
        if percent.is_empty() {
            return None;
        }
        let number: f32 = percent.parse().ok()?;
        return Some(CssLength::Percent(number));
    }

    parse_css_length_px_with_viewport(value, viewport_width_px, viewport_height_px)
        .map(CssLength::Px)
}

fn parse_css_calc_length(
    value: &str,
    viewport_width_px: Option<i32>,
    viewport_height_px: Option<i32>,
) -> Option<CssLength> {
    let value = value.trim();
    let Some(prefix) = value.get(..4) else {
        return None;
    };
    if !prefix.eq_ignore_ascii_case("calc") {
        return None;
    }

    let mut rest = &value[4..];
    rest = rest.trim_start();
    let rest = rest.strip_prefix('(')?;
    let close = rest.find(')')?;
    let inner = rest[..close].trim();
    if inner.is_empty() {
        return None;
    }
    if !rest[close + 1..].trim().is_empty() {
        return None;
    }

    let (percent, px) = parse_calc_expression(inner, viewport_width_px, viewport_height_px)?;
    if percent == 0.0 {
        return Some(CssLength::Px(px.round() as i32));
    }
    if px == 0.0 {
        return Some(CssLength::Percent(percent));
    }
    Some(CssLength::Calc { percent, px })
}

fn parse_calc_expression(
    input: &str,
    viewport_width_px: Option<i32>,
    viewport_height_px: Option<i32>,
) -> Option<(f32, f32)> {
    let bytes = input.as_bytes();
    let mut cursor = 0usize;
    let mut percent = 0f32;
    let mut px = 0f32;

    while cursor < bytes.len() {
        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            break;
        }

        let mut sign = 1f32;
        if bytes[cursor] == b'+' {
            cursor += 1;
        } else if bytes[cursor] == b'-' {
            sign = -1.0;
            cursor += 1;
        }

        while cursor < bytes.len() && bytes[cursor].is_ascii_whitespace() {
            cursor += 1;
        }
        if cursor >= bytes.len() {
            return None;
        }

        let term_start = cursor;
        while cursor < bytes.len() && bytes[cursor] != b'+' && bytes[cursor] != b'-' {
            cursor += 1;
        }
        let term = input[term_start..cursor].trim();
        if term.is_empty() {
            return None;
        }

        if let Some(raw) = term.strip_suffix('%') {
            let value: f32 = raw.trim().parse().ok()?;
            percent = percent + sign * value;
            continue;
        }

        let value = parse_css_length_px_f32_with_viewport(term, viewport_width_px, viewport_height_px)?;
        px = px + sign * value;
    }

    Some((percent, px))
}
