use crate::dom::{Document, Element, Node};
use crate::geom::{Edges, Rect};
use crate::style::{AutoEdges, ComputedStyle, Display, StyleComputer, TextAlign};

pub(super) fn add_edges(a: Edges, b: Edges) -> Edges {
    Edges {
        top: a.top.saturating_add(b.top),
        right: a.right.saturating_add(b.right),
        bottom: a.bottom.saturating_add(b.bottom),
        left: a.left.saturating_add(b.left),
    }
}

pub(super) fn constrain_flow_content_box(content_box: Rect, flow_area: Rect) -> Rect {
    let left = flow_area.x.max(content_box.x);
    let right = flow_area.right().min(content_box.right());
    Rect {
        x: left,
        y: content_box.y,
        width: right.saturating_sub(left).max(0),
        height: content_box.height,
    }
}

pub(super) fn establishes_block_formatting_context(style: &ComputedStyle) -> bool {
    matches!(style.display, Display::Flex | Display::Table)
}

pub(super) fn required_outer_width_for_float_clearance(
    style: &ComputedStyle,
    available_width_px: i32,
) -> i32 {
    let margin_left = if style.margin_auto.left {
        0
    } else {
        style.margin.left
    };
    let margin_right = if style.margin_auto.right {
        0
    } else {
        style.margin.right
    };
    let border_width = style
        .width_px
        .map(|width| width.resolve_px(available_width_px))
        .unwrap_or(1)
        .max(0);
    margin_left
        .saturating_add(border_width)
        .saturating_add(margin_right)
        .max(1)
}

pub(super) fn resolve_canvas_background(
    document: &Document,
    styles: &StyleComputer,
    root_style: &ComputedStyle,
    body_style: &ComputedStyle,
    viewport_width_px: i32,
    viewport_height_px: i32,
) -> Option<crate::geom::Color> {
    if let Some(html) = document.find_first_element_by_name("html") {
        let html_style = styles.compute_style_in_viewport(
            html,
            root_style,
            &[],
            viewport_width_px,
            viewport_height_px,
        );
        if html_style.background_color.is_some() {
            return html_style.background_color;
        }
    }
    body_style.background_color
}

pub(super) fn is_flow_block(style: &ComputedStyle, element: &Element) -> bool {
    match style.display {
        Display::Block | Display::Flex | Display::Table => true,
        Display::TableRow | Display::TableCell => true,
        Display::Inline | Display::InlineBlock => {
            if matches!(element.name.as_str(), "div" | "p" | "table") {
                return true;
            }

            if element.name != "span" {
                return false;
            }

            element.children.iter().any(|child| {
                let Node::Element(el) = child else {
                    return false;
                };
                matches!(
                    el.name.as_str(),
                    "html"
                        | "body"
                        | "div"
                        | "p"
                        | "center"
                        | "header"
                        | "main"
                        | "footer"
                        | "nav"
                        | "ul"
                        | "ol"
                        | "li"
                        | "h1"
                        | "h2"
                        | "h3"
                        | "blockquote"
                        | "pre"
                        | "table"
                        | "tr"
                        | "td"
                )
            })
        }
        Display::None => false,
    }
}

pub(super) fn apply_block_alignment(
    align: TextAlign,
    containing: Rect,
    default_x: i32,
    width: i32,
    margin: Edges,
) -> i32 {
    if width <= 0 {
        return default_x;
    }
    let available = containing
        .width
        .saturating_sub(margin.left.saturating_add(margin.right));
    if available <= width {
        return default_x;
    }
    match align {
        TextAlign::Center => containing.x.saturating_add((available - width) / 2),
        TextAlign::Right => containing
            .x
            .saturating_add(available.saturating_sub(width))
            .saturating_add(margin.left),
        TextAlign::Left => default_x,
    }
}

pub(super) fn apply_auto_margin_alignment(
    auto: AutoEdges,
    containing: Rect,
    default_x: i32,
    width: i32,
    margin: Edges,
) -> i32 {
    let left_px = if auto.left { 0 } else { margin.left };
    let right_px = if auto.right { 0 } else { margin.right };
    let available = containing
        .width
        .saturating_sub(left_px.saturating_add(right_px))
        .max(0);

    if available <= width {
        return default_x;
    }

    let remaining = available.saturating_sub(width).max(0);
    if auto.left && auto.right {
        containing
            .x
            .saturating_add(left_px)
            .saturating_add(remaining / 2)
    } else if auto.left {
        containing
            .x
            .saturating_add(left_px)
            .saturating_add(remaining)
    } else {
        default_x
    }
}

pub(super) fn parse_percentage(value: &str) -> Option<f32> {
    let value = value.trim();
    let number = value.strip_suffix('%')?;
    let number: f32 = number.trim().parse().ok()?;
    Some(number)
}
