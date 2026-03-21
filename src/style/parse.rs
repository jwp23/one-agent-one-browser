use crate::geom::{Color, Edges};
use crate::style::FontFamily;

use super::builder::FontSize;

pub(super) fn parse_css_color(value: &str) -> Option<Color> {
    let value = value.trim();
    if let Some(color) = Color::from_css_hex(value) {
        return Some(color);
    }
    if let Some(color) = parse_rgb_function(value) {
        return Some(color);
    }
    match value.to_ascii_lowercase().as_str() {
        "black" => Some(Color::BLACK),
        "white" => Some(Color::WHITE),
        _ => None,
    }
}

fn parse_rgb_function(value: &str) -> Option<Color> {
    let value = value.trim();
    let value_lower = value.to_ascii_lowercase();
    let (name, args) = if let Some(args) = value_lower.strip_prefix("rgb(") {
        ("rgb", args)
    } else if let Some(args) = value_lower.strip_prefix("rgba(") {
        ("rgba", args)
    } else {
        return None;
    };

    let args = args.strip_suffix(')')?.trim();
    if args.is_empty() {
        return None;
    }

    let parts: Vec<&str> = args.split(',').map(str::trim).collect();
    let expected = if name == "rgb" { 3 } else { 4 };
    if parts.len() != expected {
        return None;
    }

    fn parse_channel(input: &str) -> Option<u8> {
        let number: f32 = input.trim().parse().ok()?;
        Some(number.round().clamp(0.0, 255.0) as u8)
    }

    let r = parse_channel(parts[0])?;
    let g = parse_channel(parts[1])?;
    let b = parse_channel(parts[2])?;

    let a = if name == "rgba" {
        parse_alpha_channel(parts[3])?
    } else {
        255
    };

    Some(Color { r, g, b, a })
}

fn parse_alpha_channel(input: &str) -> Option<u8> {
    let number: f32 = input.trim().parse().ok()?;
    if number <= 1.0 {
        return Some((number.clamp(0.0, 1.0) * 255.0).round() as u8);
    }
    Some(number.round().clamp(0.0, 255.0) as u8)
}

pub(super) fn parse_css_font_family(value: &str) -> FontFamily {
    let mut saw_monospace = false;
    let mut saw_serif = false;
    let mut saw_sans_serif = false;

    for raw in value.split(',') {
        let token = raw.trim();
        if token.is_empty() {
            continue;
        }
        let token = token
            .trim_matches('"')
            .trim_matches('\'')
            .trim()
            .to_ascii_lowercase();

        match token.as_str() {
            "monospace" => saw_monospace = true,
            "serif" => saw_serif = true,
            "sans-serif" => saw_sans_serif = true,
            _ => {}
        }
    }

    if saw_monospace {
        FontFamily::Monospace
    } else if saw_serif {
        FontFamily::Serif
    } else if saw_sans_serif {
        FontFamily::SansSerif
    } else {
        FontFamily::SansSerif
    }
}

pub(super) fn parse_css_length_px(value: &str) -> Option<i32> {
    parse_css_length_px_with_viewport(value, None, None)
}

pub(super) fn parse_css_font_size(
    value: &str,
    viewport_width_px: Option<i32>,
    viewport_height_px: Option<i32>,
) -> Option<FontSize> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if let Some(args) = parse_css_function_args(value, "calc") {
        return parse_css_calc_font_size(args, viewport_width_px, viewport_height_px);
    }

    let (number, unit) = split_css_number_unit(value)?;
    let unit = unit.trim().to_ascii_lowercase();
    match unit.as_str() {
        "%" => Some(FontSize::ParentFactor(number / 100.0)),
        "em" => Some(FontSize::ParentFactor(number)),
        "rem" => Some(FontSize::RootFactor(number)),
        _ => absolute_css_length_to_px(number, unit.as_str(), viewport_width_px, viewport_height_px)
            .map(|px| FontSize::Px(px.round() as i32)),
    }
}

pub(crate) fn parse_css_length_px_f32_with_viewport(
    value: &str,
    viewport_width_px: Option<i32>,
    viewport_height_px: Option<i32>,
) -> Option<f32> {
    let value = value.trim();
    if value == "0" {
        return Some(0.0);
    }

    if let Some(args) = parse_css_function_args(value, "calc") {
        return parse_css_calc_length_px_f32(args, viewport_width_px, viewport_height_px);
    }
    if let Some(args) = parse_css_function_args(value, "max") {
        return split_top_level_commas(args)
            .into_iter()
            .filter_map(|part| {
                parse_css_length_px_f32_with_viewport(part, viewport_width_px, viewport_height_px)
            })
            .reduce(f32::max);
    }
    if let Some(args) = parse_css_function_args(value, "min") {
        return split_top_level_commas(args)
            .into_iter()
            .filter_map(|part| {
                parse_css_length_px_f32_with_viewport(part, viewport_width_px, viewport_height_px)
            })
            .reduce(f32::min);
    }

    let (number, unit) = split_css_number_unit(value)?;
    absolute_css_length_to_px(number, unit, viewport_width_px, viewport_height_px)
}

pub(super) fn parse_css_length_px_with_viewport(
    value: &str,
    viewport_width_px: Option<i32>,
    viewport_height_px: Option<i32>,
) -> Option<i32> {
    parse_css_length_px_f32_with_viewport(value, viewport_width_px, viewport_height_px)
        .map(|px| px.round() as i32)
}

fn parse_css_function_args<'a>(value: &'a str, name: &str) -> Option<&'a str> {
    let value = value.trim();
    let open = value.find('(')?;
    if !value[..open].trim().eq_ignore_ascii_case(name) || !value.ends_with(')') {
        return None;
    }

    let mut depth = 0usize;
    for (idx, ch) in value.char_indices() {
        match ch {
            '(' => depth = depth.saturating_add(1),
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 && idx + ch.len_utf8() != value.len() {
                    return None;
                }
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }

    Some(value[open + 1..value.len().saturating_sub(1)].trim())
}

fn parse_css_calc_length_px_f32(
    value: &str,
    viewport_width_px: Option<i32>,
    viewport_height_px: Option<i32>,
) -> Option<f32> {
    let mut total = 0.0f32;
    let mut start = 0usize;
    let mut depth = 0usize;
    let mut op = '+';

    for (idx, ch) in value.char_indices() {
        match ch {
            '(' => depth = depth.saturating_add(1),
            ')' => depth = depth.saturating_sub(1),
            '+' | '-' if depth == 0 && idx > start => {
                let term = value[start..idx].trim();
                let amount = parse_css_length_px_f32_with_viewport(
                    term,
                    viewport_width_px,
                    viewport_height_px,
                )?;
                if op == '-' {
                    total -= amount;
                } else {
                    total += amount;
                }
                op = ch;
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    let term = value[start..].trim();
    if term.is_empty() {
        return None;
    }
    let amount =
        parse_css_length_px_f32_with_viewport(term, viewport_width_px, viewport_height_px)?;
    if op == '-' {
        total -= amount;
    } else {
        total += amount;
    }
    Some(total)
}

fn parse_css_calc_font_size(
    value: &str,
    viewport_width_px: Option<i32>,
    viewport_height_px: Option<i32>,
) -> Option<FontSize> {
    let mut parent_factor = 0.0f32;
    let mut root_factor = 0.0f32;
    let mut px = 0.0f32;
    let mut start = 0usize;
    let mut depth = 0usize;
    let mut op = '+';

    for (idx, ch) in value.char_indices() {
        match ch {
            '(' => depth = depth.saturating_add(1),
            ')' => depth = depth.saturating_sub(1),
            '+' | '-' if depth == 0 && idx > start => {
                let term = value[start..idx].trim();
                apply_font_size_term(
                    term,
                    op,
                    &mut parent_factor,
                    &mut root_factor,
                    &mut px,
                    viewport_width_px,
                    viewport_height_px,
                )?;
                op = ch;
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    let term = value[start..].trim();
    if term.is_empty() {
        return None;
    }
    apply_font_size_term(
        term,
        op,
        &mut parent_factor,
        &mut root_factor,
        &mut px,
        viewport_width_px,
        viewport_height_px,
    )?;

    if parent_factor == 0.0 && root_factor == 0.0 {
        return Some(FontSize::Px(px.round() as i32));
    }
    if root_factor == 0.0 && px == 0.0 {
        return Some(FontSize::ParentFactor(parent_factor));
    }
    if parent_factor == 0.0 && px == 0.0 {
        return Some(FontSize::RootFactor(root_factor));
    }
    Some(FontSize::Calc {
        parent_factor,
        root_factor,
        px,
    })
}

fn apply_font_size_term(
    term: &str,
    op: char,
    parent_factor: &mut f32,
    root_factor: &mut f32,
    px: &mut f32,
    viewport_width_px: Option<i32>,
    viewport_height_px: Option<i32>,
) -> Option<()> {
    let sign = if op == '-' { -1.0 } else { 1.0 };
    let (number, unit) = split_css_number_unit(term)?;
    let unit = unit.trim().to_ascii_lowercase();
    match unit.as_str() {
        "%" => *parent_factor += sign * (number / 100.0),
        "em" => *parent_factor += sign * number,
        "rem" => *root_factor += sign * number,
        _ => {
            let amount =
                absolute_css_length_to_px(number, unit.as_str(), viewport_width_px, viewport_height_px)?;
            *px += sign * amount;
        }
    }
    Some(())
}

pub(super) fn split_css_number_unit(value: &str) -> Option<(f32, &str)> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let mut end = 0usize;
    for (idx, ch) in value.char_indices() {
        if !(ch.is_ascii_digit() || ch == '.' || ch == '-') {
            break;
        }
        end = idx + ch.len_utf8();
    }
    if end == 0 {
        return None;
    }

    let number: f32 = value[..end].parse().ok()?;
    Some((number, value[end..].trim()))
}

pub(super) fn absolute_css_length_to_px(
    number: f32,
    unit: &str,
    viewport_width_px: Option<i32>,
    viewport_height_px: Option<i32>,
) -> Option<f32> {
    let unit = unit.trim().to_ascii_lowercase();
    match unit.as_str() {
        "px" | "" => Some(number),
        "pt" => Some(number * (96.0 / 72.0)),
        "em" | "rem" => Some(number * 16.0),
        "vw" => {
            let width_px = viewport_width_px?;
            Some(number * (width_px as f32) / 100.0)
        }
        "vh" => {
            let height_px = viewport_height_px?;
            Some(number * (height_px as f32) / 100.0)
        }
        _ => None,
    }
}

fn split_top_level_commas(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;

    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => depth = depth.saturating_add(1),
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(input[start..idx].trim());
                start = idx + ch.len_utf8();
            }
            _ => {}
        }
    }

    parts.push(input[start..].trim());
    parts
}

#[derive(Clone, Copy, Debug)]
pub(super) struct ParsedFlex {
    pub(super) grow: i32,
    pub(super) shrink: i32,
    pub(super) basis_px: Option<i32>,
}

pub(super) fn parse_css_flex(value: &str) -> Option<ParsedFlex> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if value.eq_ignore_ascii_case("none") {
        return Some(ParsedFlex {
            grow: 0,
            shrink: 0,
            basis_px: None,
        });
    }

    if value.eq_ignore_ascii_case("auto") {
        return Some(ParsedFlex {
            grow: 1,
            shrink: 1,
            basis_px: None,
        });
    }

    let parts: Vec<&str> = value.split_whitespace().collect();
    match parts.as_slice() {
        [grow] => {
            if let Ok(grow) = grow.parse::<f32>() {
                return Some(ParsedFlex {
                    grow: grow.round().max(0.0) as i32,
                    shrink: 1,
                    basis_px: Some(0),
                });
            }
            if grow.eq_ignore_ascii_case("auto") {
                return Some(ParsedFlex {
                    grow: 1,
                    shrink: 1,
                    basis_px: None,
                });
            }
            if grow.eq_ignore_ascii_case("none") {
                return Some(ParsedFlex {
                    grow: 0,
                    shrink: 0,
                    basis_px: None,
                });
            }
            None
        }
        [grow, second] => {
            let grow = grow.parse::<f32>().ok()?.round().max(0.0) as i32;
            if let Ok(shrink) = second.parse::<f32>() {
                return Some(ParsedFlex {
                    grow,
                    shrink: shrink.round().max(0.0) as i32,
                    basis_px: None,
                });
            }
            if second.eq_ignore_ascii_case("auto") {
                return Some(ParsedFlex {
                    grow,
                    shrink: 1,
                    basis_px: None,
                });
            }
            parse_css_length_px(second).map(|px| ParsedFlex {
                grow,
                shrink: 1,
                basis_px: Some(px.max(0)),
            })
        }
        [grow, shrink, basis] => {
            let grow = grow.parse::<f32>().ok()?.round().max(0.0) as i32;
            let shrink = shrink.parse::<f32>().ok()?.round().max(0.0) as i32;
            let basis_px = if basis.eq_ignore_ascii_case("auto") {
                None
            } else {
                Some(parse_css_length_px(basis)?.max(0))
            };
            Some(ParsedFlex {
                grow,
                shrink,
                basis_px,
            })
        }
        _ => None,
    }
}

pub(super) fn parse_css_box_edges(value: &str) -> Option<Edges> {
    let lengths: Vec<i32> = value
        .split_whitespace()
        .filter_map(parse_css_length_px)
        .collect();

    match lengths.as_slice() {
        [] => None,
        [all] => Some(Edges {
            top: *all,
            right: *all,
            bottom: *all,
            left: *all,
        }),
        [vertical, horizontal] => Some(Edges {
            top: *vertical,
            right: *horizontal,
            bottom: *vertical,
            left: *horizontal,
        }),
        [top, horizontal, bottom] => Some(Edges {
            top: *top,
            right: *horizontal,
            bottom: *bottom,
            left: *horizontal,
        }),
        [top, right, bottom, left] => Some(Edges {
            top: *top,
            right: *right,
            bottom: *bottom,
            left: *left,
        }),
        _ => None,
    }
}

pub(super) fn parse_css_box_edges_with_auto(value: &str) -> Option<(Edges, super::AutoEdges)> {
    #[derive(Clone, Copy, Debug)]
    enum Token {
        Px(i32),
        Auto,
    }

    let tokens: Vec<Token> = value
        .split_whitespace()
        .filter_map(|part| {
            if part.eq_ignore_ascii_case("auto") {
                return Some(Token::Auto);
            }
            parse_css_length_px(part).map(Token::Px)
        })
        .collect();

    fn to_px(token: Token) -> i32 {
        match token {
            Token::Px(px) => px,
            Token::Auto => 0,
        }
    }

    fn to_auto(token: Token) -> bool {
        matches!(token, Token::Auto)
    }

    let (top, right, bottom, left) = match tokens.as_slice() {
        [] => return None,
        [all] => (*all, *all, *all, *all),
        [vertical, horizontal] => (*vertical, *horizontal, *vertical, *horizontal),
        [top, horizontal, bottom] => (*top, *horizontal, *bottom, *horizontal),
        [top, right, bottom, left] => (*top, *right, *bottom, *left),
        _ => return None,
    };

    let edges = Edges {
        top: to_px(top),
        right: to_px(right),
        bottom: to_px(bottom),
        left: to_px(left),
    };
    let auto = super::AutoEdges {
        top: to_auto(top),
        right: to_auto(right),
        bottom: to_auto(bottom),
        left: to_auto(left),
    };

    Some((edges, auto))
}

pub(super) fn parse_html_length_px(value: &str) -> Option<i32> {
    let value = value.trim();
    if value.ends_with('%') {
        return None;
    }

    parse_css_length_px(value).or_else(|| value.parse::<i32>().ok())
}

#[cfg(test)]
mod tests {
    use super::parse_css_length_px;

    #[test]
    fn parses_calc_lengths() {
        assert_eq!(parse_css_length_px("calc(1rem + 4px)"), Some(20));
        assert_eq!(parse_css_length_px("calc(20px - 3px)"), Some(17));
    }

    #[test]
    fn parses_min_and_max_lengths() {
        assert_eq!(parse_css_length_px("max(12px, 1rem)"), Some(16));
        assert_eq!(parse_css_length_px("min(12px, 1rem)"), Some(12));
        assert_eq!(parse_css_length_px("max(calc(1rem + 4px), 10px)"), Some(20));
    }
}
