use super::parse::{
    absolute_css_length_to_px, parse_css_length_px_f32_with_viewport, parse_css_length_px_with_viewport,
    split_css_number_unit,
};

#[derive(Clone, Copy, Debug)]
pub enum CssLength {
    Px(i32),
    Percent(f32),
    Em(f32),
    Rem(f32),
    Calc {
        percent: f32,
        px: f32,
        em: f32,
        rem: f32,
    },
}

impl CssLength {
    pub fn resolve_px(self, reference_px: i32) -> i32 {
        let reference_px = reference_px.max(0);
        match self {
            CssLength::Px(px) => px,
            CssLength::Percent(percent) => {
                ((reference_px as f32) * (percent / 100.0)).round() as i32
            }
            CssLength::Em(em) => (em * 16.0).round() as i32,
            CssLength::Rem(rem) => (rem * 16.0).round() as i32,
            CssLength::Calc {
                percent,
                px,
                em,
                rem,
            } => {
                ((reference_px as f32) * (percent / 100.0) + px + (em * 16.0) + (rem * 16.0))
                    .round() as i32
            }
        }
    }

    pub fn definite_px(self) -> Option<i32> {
        match self {
            CssLength::Px(px) => Some(px),
            CssLength::Percent(_) => None,
            CssLength::Em(em) => Some((em * 16.0).round() as i32),
            CssLength::Rem(rem) => Some((rem * 16.0).round() as i32),
            CssLength::Calc { percent, .. } if percent != 0.0 => None,
            CssLength::Calc { px, em, rem, .. } => {
                Some((px + (em * 16.0) + (rem * 16.0)).round() as i32)
            }
        }
    }

    pub fn resolve_font_relative(self, font_size_px: i32, root_font_size_px: i32) -> CssLength {
        let font_size_px = font_size_px.max(0) as f32;
        let root_font_size_px = root_font_size_px.max(0) as f32;
        let (percent, px) = match self {
            CssLength::Px(px) => (0.0, px as f32),
            CssLength::Percent(percent) => (percent, 0.0),
            CssLength::Em(em) => (0.0, em * font_size_px),
            CssLength::Rem(rem) => (0.0, rem * root_font_size_px),
            CssLength::Calc {
                percent,
                px,
                em,
                rem,
            } => (percent, px + (em * font_size_px) + (rem * root_font_size_px)),
        };
        normalize_css_length(percent, px)
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

    if let Some((number, unit)) = split_css_number_unit(value) {
        let unit = unit.to_ascii_lowercase();
        return match unit.as_str() {
            "%" => Some(CssLength::Percent(number)),
            "em" => Some(CssLength::Em(number)),
            "rem" => Some(CssLength::Rem(number)),
            _ => absolute_css_length_to_px(number, unit.as_str(), viewport_width_px, viewport_height_px)
                .map(|px| CssLength::Px(px.round() as i32)),
        };
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

    let (percent, px, em, rem) =
        parse_calc_expression(inner, viewport_width_px, viewport_height_px)?;
    Some(normalize_css_length_components(percent, px, em, rem))
}

fn parse_calc_expression(
    input: &str,
    viewport_width_px: Option<i32>,
    viewport_height_px: Option<i32>,
) -> Option<(f32, f32, f32, f32)> {
    let bytes = input.as_bytes();
    let mut cursor = 0usize;
    let mut percent = 0f32;
    let mut px = 0f32;
    let mut em = 0f32;
    let mut rem = 0f32;

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

        let (number, unit) = split_css_number_unit(term)?;
        let unit = unit.to_ascii_lowercase();
        match unit.as_str() {
            "%" => percent += sign * number,
            "em" => em += sign * number,
            "rem" => rem += sign * number,
            _ => {
                let value = absolute_css_length_to_px(
                    number,
                    unit.as_str(),
                    viewport_width_px,
                    viewport_height_px,
                )
                .or_else(|| {
                    parse_css_length_px_f32_with_viewport(
                        term,
                        viewport_width_px,
                        viewport_height_px,
                    )
                })?;
                px += sign * value;
            }
        }
    }

    Some((percent, px, em, rem))
}

fn normalize_css_length(percent: f32, px: f32) -> CssLength {
    normalize_css_length_components(percent, px, 0.0, 0.0)
}

fn normalize_css_length_components(percent: f32, px: f32, em: f32, rem: f32) -> CssLength {
    if percent == 0.0 && em == 0.0 && rem == 0.0 {
        return CssLength::Px(px.round() as i32);
    }
    if px == 0.0 && em == 0.0 && rem == 0.0 {
        return CssLength::Percent(percent);
    }
    if percent == 0.0 && px == 0.0 && rem == 0.0 {
        return CssLength::Em(em);
    }
    if percent == 0.0 && px == 0.0 && em == 0.0 {
        return CssLength::Rem(rem);
    }
    CssLength::Calc {
        percent,
        px,
        em,
        rem,
    }
}
