use crate::geom::Color;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GradientDirection {
    TopToBottom,
    BottomToTop,
    LeftToRight,
    RightToLeft,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinearGradient {
    pub direction: GradientDirection,
    pub start: Color,
    pub end: Color,
}

pub(super) fn parse_css_linear_gradient(value: &str) -> Option<LinearGradient> {
    let value = value.trim();
    let lower = value.to_ascii_lowercase();
    let args = lower.strip_prefix("linear-gradient(")?;
    let args = args.strip_suffix(')')?.trim();
    if args.is_empty() {
        return None;
    }

    let parts = split_top_level_commas(args);
    if parts.len() < 2 {
        return None;
    }

    let mut direction = GradientDirection::TopToBottom;
    let mut stop_start = 0usize;
    let first = parts[0].trim();
    if let Some(rest) = first.strip_prefix("to ") {
        direction = match rest.trim() {
            "bottom" => GradientDirection::TopToBottom,
            "top" => GradientDirection::BottomToTop,
            "right" => GradientDirection::LeftToRight,
            "left" => GradientDirection::RightToLeft,
            _ => return None,
        };
        stop_start = 1;
    }

    let mut colors = Vec::new();
    for part in &parts[stop_start..] {
        let Some(color) = parse_stop_color(part) else {
            continue;
        };
        colors.push(color);
    }
    if colors.len() < 2 {
        return None;
    }

    Some(LinearGradient {
        direction,
        start: colors[0],
        end: *colors.last().expect("colors len >= 2"),
    })
}

fn parse_stop_color(stop: &str) -> Option<Color> {
    let stop = stop.trim();
    if stop.is_empty() {
        return None;
    }

    if let Some(color) = super::parse::parse_css_color(stop) {
        return Some(color);
    }

    let token = first_component_outside_parens(stop);
    super::parse::parse_css_color(token)
}

fn first_component_outside_parens(input: &str) -> &str {
    let bytes = input.as_bytes();
    let mut depth = 0usize;
    let mut idx = 0usize;
    while idx < bytes.len() {
        match bytes[idx] {
            b'(' => depth = depth.saturating_add(1),
            b')' => depth = depth.saturating_sub(1),
            b' ' | b'\t' | b'\n' | b'\r' if depth == 0 => break,
            _ => {}
        }
        idx += 1;
    }
    input[..idx].trim()
}

fn split_top_level_commas(input: &str) -> Vec<&str> {
    let bytes = input.as_bytes();
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut parts = Vec::new();

    for (idx, &b) in bytes.iter().enumerate() {
        match b {
            b'(' => depth = depth.saturating_add(1),
            b')' => depth = depth.saturating_sub(1),
            b',' if depth == 0 => {
                parts.push(input[start..idx].trim());
                start = idx + 1;
            }
            _ => {}
        }
    }

    parts.push(input[start..].trim());
    parts
}
