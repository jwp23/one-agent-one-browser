use crate::geom::Edges;

use super::parse::{
    parse_css_box_edges, parse_css_box_edges_with_auto, parse_css_color, parse_css_flex,
    parse_css_length_px,
};
use super::{
    AutoEdges, CascadePriority, Display, FlexAlignItems, FlexDirection, FlexJustifyContent, FlexWrap,
    FontFamily, Position, StyleBuilder, TextAlign, Visibility,
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
            if value.trim().eq_ignore_ascii_case("auto") {
                builder.apply_top(None, priority);
            } else if let Some(px) = parse_css_length_px(value) {
                builder.apply_top(Some(px), priority);
            }
        }
        "right" => {
            if value.trim().eq_ignore_ascii_case("auto") {
                builder.apply_right(None, priority);
            } else if let Some(px) = parse_css_length_px(value) {
                builder.apply_right(Some(px), priority);
            }
        }
        "bottom" => {
            if value.trim().eq_ignore_ascii_case("auto") {
                builder.apply_bottom(None, priority);
            } else if let Some(px) = parse_css_length_px(value) {
                builder.apply_bottom(Some(px), priority);
            }
        }
        "left" => {
            if value.trim().eq_ignore_ascii_case("auto") {
                builder.apply_left(None, priority);
            } else if let Some(px) = parse_css_length_px(value) {
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
        "font-family" => {
            let family = if value.to_ascii_lowercase().contains("monospace") {
                FontFamily::Monospace
            } else {
                FontFamily::SansSerif
            };
            builder.apply_font_family(family, priority);
        }
        "font-size" => {
            if let Some(px) = parse_css_length_px(value) {
                builder.apply_font_size_px(px, priority);
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
            if let Some(px) = parse_css_length_px(value) {
                builder.apply_padding_component(|e| Edges { left: px, ..e }, priority);
            }
        }
        "padding-right" => {
            if let Some(px) = parse_css_length_px(value) {
                builder.apply_padding_component(|e| Edges { right: px, ..e }, priority);
            }
        }
        "padding-top" => {
            if let Some(px) = parse_css_length_px(value) {
                builder.apply_padding_component(|e| Edges { top: px, ..e }, priority);
            }
        }
        "padding-bottom" => {
            if let Some(px) = parse_css_length_px(value) {
                builder.apply_padding_component(|e| Edges { bottom: px, ..e }, priority);
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
            } else if let Some(px) = parse_css_length_px(value) {
                builder.apply_margin_component(|e| Edges { left: px, ..e }, priority);
                builder.apply_margin_auto_component(|a| AutoEdges { left: false, ..a }, priority);
            }
        }
        "margin-right" => {
            if value.trim().eq_ignore_ascii_case("auto") {
                builder.apply_margin_auto_component(|a| AutoEdges { right: true, ..a }, priority);
            } else if let Some(px) = parse_css_length_px(value) {
                builder.apply_margin_component(|e| Edges { right: px, ..e }, priority);
                builder.apply_margin_auto_component(|a| AutoEdges { right: false, ..a }, priority);
            }
        }
        "margin-top" => {
            if let Some(px) = parse_css_length_px(value) {
                builder.apply_margin_component(|e| Edges { top: px, ..e }, priority);
            }
        }
        "margin-bottom" => {
            if let Some(px) = parse_css_length_px(value) {
                builder.apply_margin_component(|e| Edges { bottom: px, ..e }, priority);
            }
        }
        "width" => {
            if let Some(px) = parse_css_length_px(value) {
                builder.apply_width(Some(px), priority);
            }
        }
        "min-width" => {
            if let Some(px) = parse_css_length_px(value) {
                builder.apply_min_width(Some(px), priority);
            }
        }
        "max-width" => {
            if let Some(px) = parse_css_length_px(value) {
                builder.apply_max_width(Some(px), priority);
            }
        }
        "height" => {
            if let Some(px) = parse_css_length_px(value) {
                builder.apply_height(Some(px), priority);
            }
        }
        "min-height" => {
            if let Some(px) = parse_css_length_px(value) {
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
            if value.trim().eq_ignore_ascii_case("auto") {
                builder.apply_flex_basis(None, priority);
            } else if let Some(px) = parse_css_length_px(value) {
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
            if let Some(px) = parse_css_length_px(first) {
                builder.apply_flex_gap_px(px.max(0), priority);
            }
        }
        _ => {}
    }
}

