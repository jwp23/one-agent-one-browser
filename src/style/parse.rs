use crate::geom::{Color, Edges};

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

pub(super) fn parse_css_length_px(value: &str) -> Option<i32> {
    let value = value.trim();
    if value == "0" {
        return Some(0);
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
    let unit = value[end..].trim().to_ascii_lowercase();
    let px = match unit.as_str() {
        "px" | "" => number,
        "pt" => number * (96.0 / 72.0),
        "rem" | "em" => number * 16.0,
        _ => return None,
    };
    Some(px.round() as i32)
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
