use crate::geom::Edges;

use super::parse::{
    parse_css_box_edges, parse_css_box_edges_with_auto, parse_css_color, parse_css_flex,
    parse_css_font_family, parse_css_length_px,
};
use super::{
    AutoEdges, BorderStyle, CascadePriority, Display, FlexAlignItems, FlexDirection,
    FlexJustifyContent, FlexWrap, LetterSpacing, Position, StyleBuilder, TextAlign, Visibility,
};

pub(super) fn apply_declaration(
    builder: &mut StyleBuilder,
    name: &str,
    value: &str,
    priority: CascadePriority,
) {
    match name {
        "display" => {
            if value.eq_ignore_ascii_case("none") {
                builder.apply_display(Display::None, priority);
            } else if value.eq_ignore_ascii_case("block") {
                builder.apply_display(Display::Block, priority);
            } else if value.eq_ignore_ascii_case("inline") {
                builder.apply_display(Display::Inline, priority);
            } else if value.eq_ignore_ascii_case("inline-block") {
                builder.apply_display(Display::InlineBlock, priority);
            } else if value.eq_ignore_ascii_case("flex") {
                builder.apply_display(Display::Flex, priority);
            }
        }
        "visibility" => {
            if value.eq_ignore_ascii_case("hidden") {
                builder.apply_visibility(Visibility::Hidden, priority);
            } else if value.eq_ignore_ascii_case("visible") {
                builder.apply_visibility(Visibility::Visible, priority);
            }
        }
        "position" => {
            let position = match value.trim().to_ascii_lowercase().as_str() {
                "static" => Some(Position::Static),
                "relative" => Some(Position::Relative),
                "absolute" => Some(Position::Absolute),
                "fixed" => Some(Position::Fixed),
                _ => None,
            };
            if let Some(position) = position {
                builder.apply_position(position, priority);
            }
        }
        "top" => {
            let value = value.trim();
            if value.eq_ignore_ascii_case("auto")
                || value.eq_ignore_ascii_case("unset")
                || value.eq_ignore_ascii_case("initial")
            {
                builder.apply_top(None, priority);
            } else if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_top(Some(px), priority);
            }
        }
        "right" => {
            let value = value.trim();
            if value.eq_ignore_ascii_case("auto")
                || value.eq_ignore_ascii_case("unset")
                || value.eq_ignore_ascii_case("initial")
            {
                builder.apply_right(None, priority);
            } else if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_right(Some(px), priority);
            }
        }
        "bottom" => {
            let value = value.trim();
            if value.eq_ignore_ascii_case("auto")
                || value.eq_ignore_ascii_case("unset")
                || value.eq_ignore_ascii_case("initial")
            {
                builder.apply_bottom(None, priority);
            } else if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_bottom(Some(px), priority);
            }
        }
        "left" => {
            let value = value.trim();
            if value.eq_ignore_ascii_case("auto")
                || value.eq_ignore_ascii_case("unset")
                || value.eq_ignore_ascii_case("initial")
            {
                builder.apply_left(None, priority);
            } else if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_left(Some(px), priority);
            }
        }
        "color" => {
            if let Some(color) = parse_css_color(value) {
                builder.apply_color(color, priority);
            }
        }
        "background-color" => {
            if let Some(color) = parse_css_color(value) {
                builder.apply_background_color(Some(color), priority);
            } else if value.eq_ignore_ascii_case("transparent") {
                builder.apply_background_color(None, priority);
            }
        }
        "background" => {
            let value = value.trim();
            if let Some(color) = parse_css_color(value) {
                builder.apply_background_color(Some(color), priority);
            } else if value.eq_ignore_ascii_case("transparent") {
                builder.apply_background_color(None, priority);
            }
        }
        "opacity" => {
            if let Some(opacity) = parse_css_opacity_u8(value) {
                builder.apply_opacity(opacity, priority);
            }
        }
        "font-family" => {
            builder.apply_font_family(parse_css_font_family(value), priority);
        }
        "font-size" => {
            if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_font_size_px(px, priority);
            }
        }
        "letter-spacing" => {
            let value = value.trim();
            if value.eq_ignore_ascii_case("normal") {
                builder.apply_letter_spacing(LetterSpacing::Normal, priority);
            } else if let Some(factor) = parse_em_factor(value) {
                builder.apply_letter_spacing(LetterSpacing::Em(factor), priority);
            } else if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_letter_spacing(LetterSpacing::Px(px), priority);
            }
        }
        "font-weight" => {
            if value.eq_ignore_ascii_case("bold") {
                builder.apply_bold(true, priority);
            } else if value.eq_ignore_ascii_case("normal") {
                builder.apply_bold(false, priority);
            } else if let Ok(weight) = value.trim().parse::<u16>() {
                builder.apply_bold(weight >= 600, priority);
            }
        }
        "text-decoration" => {
            if value.eq_ignore_ascii_case("underline") {
                builder.apply_underline(true, priority);
            } else if value.eq_ignore_ascii_case("none") {
                builder.apply_underline(false, priority);
            }
        }
        "text-align" => {
            let align = match value.trim().to_ascii_lowercase().as_str() {
                "left" => Some(TextAlign::Left),
                "center" => Some(TextAlign::Center),
                "right" => Some(TextAlign::Right),
                _ => None,
            };
            if let Some(align) = align {
                builder.apply_text_align(align, priority);
            }
        }
        "line-height" => {
            if let Some(px) = builder.parse_css_line_height_px(value) {
                builder.apply_line_height_px(px, priority);
            } else if value.eq_ignore_ascii_case("normal") {
                builder.apply_line_height_px(None, priority);
            }
        }
        "padding" => {
            if let Some(edges) = parse_css_box_edges(value) {
                builder.apply_padding(edges, priority);
            }
        }
        "padding-left" => {
            if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_padding_component(|e| Edges { left: px, ..e }, priority);
            }
        }
        "padding-right" => {
            if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_padding_component(|e| Edges { right: px, ..e }, priority);
            }
        }
        "padding-top" => {
            if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_padding_component(|e| Edges { top: px, ..e }, priority);
            }
        }
        "padding-bottom" => {
            if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_padding_component(|e| Edges { bottom: px, ..e }, priority);
            }
        }
        "border" => {
            if let Some(border) = parse_border_shorthand(value) {
                if let Some(width) = border.width_px {
                    builder.apply_border_width(all_edges(width), priority);
                }
                if let Some(style) = border.style {
                    builder.apply_border_style(style, priority);
                }
                if let Some(color) = border.color {
                    builder.apply_border_color(color, priority);
                }
            }
        }
        "border-width" => {
            if let Some(edges) = parse_css_box_edges(value) {
                builder.apply_border_width(edges, priority);
            }
        }
        "border-style" => {
            let style = match value.trim().to_ascii_lowercase().as_str() {
                "none" => Some(BorderStyle::None),
                "solid" => Some(BorderStyle::Solid),
                _ => None,
            };
            if let Some(style) = style {
                builder.apply_border_style(style, priority);
            }
        }
        "border-color" => {
            if let Some(color) = value
                .split_whitespace()
                .find_map(parse_css_color)
            {
                builder.apply_border_color(color, priority);
            }
        }
        "border-bottom" => {
            if let Some(border) = parse_border_shorthand(value) {
                if let Some(width) = border.width_px {
                    builder.apply_border_width_component(|e| Edges { bottom: width, ..e }, priority);
                }
                if let Some(style) = border.style {
                    builder.apply_border_style(style, priority);
                }
                if let Some(color) = border.color {
                    builder.apply_border_color(color, priority);
                }
            }
        }
        "border-radius" => {
            if let Some(px) = parse_css_border_radius_px(value) {
                builder.apply_border_radius_px(px.max(0), priority);
            }
        }
        "margin" => {
            if let Some((edges, auto)) = parse_css_box_edges_with_auto(value) {
                builder.apply_margin(edges, priority);
                builder.apply_margin_auto(auto, priority);
            }
        }
        "margin-left" => {
            if value.trim().eq_ignore_ascii_case("auto") {
                builder.apply_margin_auto_component(|a| AutoEdges { left: true, ..a }, priority);
            } else if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_margin_component(|e| Edges { left: px, ..e }, priority);
                builder.apply_margin_auto_component(|a| AutoEdges { left: false, ..a }, priority);
            }
        }
        "margin-right" => {
            if value.trim().eq_ignore_ascii_case("auto") {
                builder.apply_margin_auto_component(|a| AutoEdges { right: true, ..a }, priority);
            } else if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_margin_component(|e| Edges { right: px, ..e }, priority);
                builder.apply_margin_auto_component(|a| AutoEdges { right: false, ..a }, priority);
            }
        }
        "margin-top" => {
            if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_margin_component(|e| Edges { top: px, ..e }, priority);
            }
        }
        "margin-bottom" => {
            if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_margin_component(|e| Edges { bottom: px, ..e }, priority);
            }
        }
        "width" => {
            let value = value.trim();
            if value.eq_ignore_ascii_case("auto")
                || value.eq_ignore_ascii_case("unset")
                || value.eq_ignore_ascii_case("initial")
            {
                builder.apply_width(None, priority);
            } else if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_width(Some(px), priority);
            }
        }
        "min-width" => {
            let value = value.trim();
            if value.eq_ignore_ascii_case("unset") || value.eq_ignore_ascii_case("initial") {
                builder.apply_min_width(None, priority);
            } else if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_min_width(Some(px), priority);
            }
        }
        "max-width" => {
            let value = value.trim();
            if value.eq_ignore_ascii_case("unset") || value.eq_ignore_ascii_case("initial") {
                builder.apply_max_width(None, priority);
            } else if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_max_width(Some(px), priority);
            }
        }
        "height" => {
            let value = value.trim();
            if value.eq_ignore_ascii_case("auto")
                || value.eq_ignore_ascii_case("unset")
                || value.eq_ignore_ascii_case("initial")
            {
                builder.apply_height(None, priority);
            } else if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_height(Some(px), priority);
            }
        }
        "min-height" => {
            let value = value.trim();
            if value.eq_ignore_ascii_case("unset") || value.eq_ignore_ascii_case("initial") {
                builder.apply_min_height(None, priority);
            } else if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_min_height(Some(px), priority);
            }
        }
        "flex-direction" => {
            let direction = match value.trim().to_ascii_lowercase().as_str() {
                "row" => Some(FlexDirection::Row),
                "column" => Some(FlexDirection::Column),
                _ => None,
            };
            if let Some(direction) = direction {
                builder.apply_flex_direction(direction, priority);
            }
        }
        "flex-wrap" => {
            let wrap = match value.trim().to_ascii_lowercase().as_str() {
                "nowrap" => Some(FlexWrap::NoWrap),
                "wrap" => Some(FlexWrap::Wrap),
                _ => None,
            };
            if let Some(wrap) = wrap {
                builder.apply_flex_wrap(wrap, priority);
            }
        }
        "flex-grow" => {
            if let Ok(grow) = value.trim().parse::<i32>() {
                builder.apply_flex_grow(grow.max(0), priority);
            }
        }
        "flex-shrink" => {
            if let Ok(shrink) = value.trim().parse::<i32>() {
                builder.apply_flex_shrink(shrink.max(0), priority);
            }
        }
        "flex-basis" => {
            let value = value.trim();
            if value.eq_ignore_ascii_case("auto")
                || value.eq_ignore_ascii_case("unset")
                || value.eq_ignore_ascii_case("initial")
            {
                builder.apply_flex_basis(None, priority);
            } else if let Some(px) = builder.parse_css_length_px(value) {
                builder.apply_flex_basis(Some(px.max(0)), priority);
            }
        }
        "flex" => {
            if let Some(flex) = parse_css_flex(value) {
                builder.apply_flex_grow(flex.grow, priority);
                builder.apply_flex_shrink(flex.shrink, priority);
                builder.apply_flex_basis(flex.basis_px, priority);
            }
        }
        "justify-content" => {
            let justify = match value.trim().to_ascii_lowercase().as_str() {
                "space-between" => Some(FlexJustifyContent::SpaceBetween),
                "flex-start" | "start" => Some(FlexJustifyContent::Start),
                "center" => Some(FlexJustifyContent::Center),
                "flex-end" | "end" => Some(FlexJustifyContent::End),
                _ => None,
            };
            if let Some(justify) = justify {
                builder.apply_flex_justify_content(justify, priority);
            }
        }
        "align-items" => {
            let align = match value.trim().to_ascii_lowercase().as_str() {
                "center" => Some(FlexAlignItems::Center),
                "flex-start" | "start" => Some(FlexAlignItems::Start),
                "flex-end" | "end" => Some(FlexAlignItems::End),
                _ => None,
            };
            if let Some(align) = align {
                builder.apply_flex_align_items(align, priority);
            }
        }
        "gap" => {
            let first = value.split_whitespace().next().unwrap_or("");
            if let Some(px) = builder.parse_css_length_px(first) {
                builder.apply_flex_gap_px(px.max(0), priority);
            }
        }
        _ => {}
    }
}

#[derive(Clone, Copy, Debug)]
struct ParsedBorder {
    width_px: Option<i32>,
    style: Option<BorderStyle>,
    color: Option<crate::geom::Color>,
}

fn parse_border_shorthand(value: &str) -> Option<ParsedBorder> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let mut width_px = None;
    let mut style = None;
    let mut color = None;

    for token in value.split_whitespace() {
        if width_px.is_none() {
            if let Some(px) = parse_css_length_px(token) {
                width_px = Some(px.max(0));
                continue;
            }
        }

        if style.is_none() {
            let parsed = match token.to_ascii_lowercase().as_str() {
                "none" => Some(BorderStyle::None),
                "solid" => Some(BorderStyle::Solid),
                _ => None,
            };
            if parsed.is_some() {
                style = parsed;
                continue;
            }
        }

        if color.is_none() {
            if let Some(parsed) = parse_css_color(token) {
                color = Some(parsed);
                continue;
            }
        }
    }

    if width_px.is_none() && style.is_none() && color.is_none() {
        return None;
    }

    Some(ParsedBorder {
        width_px,
        style,
        color,
    })
}

fn all_edges(px: i32) -> Edges {
    let px = px.max(0);
    Edges {
        top: px,
        right: px,
        bottom: px,
        left: px,
    }
}

fn parse_css_border_radius_px(value: &str) -> Option<i32> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    let first = value.split('/').next().unwrap_or(value);
    let first = first.split_whitespace().next().unwrap_or(first);
    parse_css_length_px(first)
}

fn parse_css_opacity_u8(value: &str) -> Option<u8> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let number: f32 = value.parse().ok()?;
    Some((number.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn parse_em_factor(value: &str) -> Option<f32> {
    let value = value.trim();
    let number = value.strip_suffix("em")?;
    number.trim().parse().ok()
}
