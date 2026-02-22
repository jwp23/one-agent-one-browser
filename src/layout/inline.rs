use crate::dom::{Element, Node};
use crate::geom::{Rect, Size};
use crate::render::{DisplayCommand, DrawText, FontMetricsPx, LinkHitRegion, TextStyle};
use crate::style::{ComputedStyle, Display, TextAlign, Visibility};
use std::rc::Rc;

use super::LayoutEngine;

#[derive(Clone, Debug)]
enum InlineToken<'doc> {
    Word(String, TextStyle, bool, Option<Rc<str>>),
    Space(TextStyle, bool, Option<Rc<str>>),
    Newline,
    Spacer(Size),
    ElementBox(InlineElementBox<'doc>),
}

#[derive(Clone, Debug)]
struct InlineElementBox<'doc> {
    element: &'doc Element,
    style: ComputedStyle,
    size: Size,
    visible: bool,
    link_href: Option<Rc<str>>,
}

pub(super) fn layout_inline_nodes<'doc>(
    engine: &mut LayoutEngine<'_>,
    nodes: &[&'doc Node],
    parent_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    content_box: Rect,
    start_y: i32,
    paint: bool,
) -> Result<i32, String> {
    layout_inline_nodes_with_link(
        engine,
        nodes,
        parent_style,
        ancestors,
        content_box,
        start_y,
        paint,
        None,
    )
}

pub(super) fn layout_inline_nodes_with_link<'doc>(
    engine: &mut LayoutEngine<'_>,
    nodes: &[&'doc Node],
    parent_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    content_box: Rect,
    start_y: i32,
    paint: bool,
    link_href: Option<Rc<str>>,
) -> Result<i32, String> {
    let mut tokens = Vec::new();
    let mut cursor = InlineCursor::default();

    for &node in nodes {
        collect_tokens(
            engine,
            node,
            parent_style,
            ancestors,
            paint,
            link_href.clone(),
            &mut cursor,
            &mut tokens,
            content_box.width,
        )?;
    }

    layout_tokens(
        engine,
        &tokens,
        parent_style,
        ancestors,
        content_box,
        start_y,
        paint,
    )
}

pub(super) fn measure_inline_nodes<'doc>(
    engine: &LayoutEngine<'_>,
    nodes: &[&'doc Node],
    parent_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    max_width: i32,
) -> Result<Size, String> {
    let mut tokens = Vec::new();
    let mut cursor = InlineCursor::default();

    for &node in nodes {
        collect_tokens(
            engine,
            node,
            parent_style,
            ancestors,
            false,
            None,
            &mut cursor,
            &mut tokens,
            max_width,
        )?;
    }

    measure_tokens(engine, &tokens, parent_style, max_width)
}

#[derive(Default)]
struct InlineCursor {
    pending_space: Option<PendingSpace>,
}

#[derive(Clone, Debug)]
struct PendingSpace {
    style: TextStyle,
    visible: bool,
    link_href: Option<Rc<str>>,
}

impl InlineCursor {
    fn mark_pending_space(&mut self, style: TextStyle, visible: bool, link_href: Option<Rc<str>>) {
        self.pending_space = Some(PendingSpace {
            style,
            visible,
            link_href,
        });
    }

    fn clear_pending_space(&mut self) {
        self.pending_space = None;
    }

    fn flush_pending_space<'doc>(&mut self, out: &mut Vec<InlineToken<'doc>>) {
        let Some(space) = self.pending_space.take() else {
            return;
        };
        if matches!(out.last(), Some(InlineToken::Newline) | None) {
            return;
        }
        out.push(InlineToken::Space(
            space.style,
            space.visible,
            space.link_href,
        ));
    }
}

fn collect_tokens<'doc>(
    engine: &LayoutEngine<'_>,
    node: &'doc Node,
    parent_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    paint: bool,
    link_href: Option<Rc<str>>,
    cursor: &mut InlineCursor,
    out: &mut Vec<InlineToken<'doc>>,
    max_width: i32,
) -> Result<(), String> {
    match node {
        Node::Text(text) => {
            let visible = paint && parent_style.visibility == Visibility::Visible;
            let transformed = parent_style.text_transform.apply(text);
            push_text(
                transformed.as_ref(),
                engine.text_style_for(parent_style),
                visible,
                link_href,
                cursor,
                out,
            );
            Ok(())
        }
        Node::Element(el) => {
            let style = engine.styles.compute_style_in_viewport(
                el,
                parent_style,
                ancestors,
                engine.viewport.width_px,
                engine.viewport.height_px,
            );
            if style.display == Display::None {
                return Ok(());
            }

            if el.name == "br" {
                out.push(InlineToken::Newline);
                cursor.clear_pending_space();
                return Ok(());
            }

            let link_href = anchor_href(el).or(link_href);
            let paint = paint && style.visibility == Visibility::Visible;
            if is_replaced_element(el) {
                cursor.flush_pending_space(out);
                let size = measure_replaced_element_outer_size(el, &style, max_width)?;
                out.push(InlineToken::ElementBox(InlineElementBox {
                    element: el,
                    style,
                    size,
                    visible: paint,
                    link_href,
                }));
                return Ok(());
            }
            let display = style.display;
            ancestors.push(el);
            match display {
                Display::Inline => {
                    let padding = style.padding.resolve_px(max_width);
                    push_inline_spacing(out, style.margin.left.saturating_add(padding.left));
                    for child in &el.children {
                        collect_tokens(
                            engine,
                            child,
                            &style,
                            ancestors,
                            paint,
                            link_href.clone(),
                            cursor,
                            out,
                            max_width,
                        )?;
                    }
                    push_inline_spacing(out, style.margin.right.saturating_add(padding.right));
                }
                _ => {
                    cursor.flush_pending_space(out);
                    let size = measure_inline_element_outer_size(
                        engine, el, &style, ancestors, max_width,
                    )?;
                    out.push(InlineToken::ElementBox(InlineElementBox {
                        element: el,
                        style,
                        size,
                        visible: paint,
                        link_href,
                    }));
                }
            }
            ancestors.pop();
            Ok(())
        }
    }
}

fn anchor_href(element: &Element) -> Option<Rc<str>> {
    if element.name != "a" {
        return None;
    }
    let href = element.attributes.get("href")?.trim();
    if href.is_empty() {
        return None;
    }
    Some(Rc::from(href))
}

pub(super) fn is_replaced_element(element: &Element) -> bool {
    matches!(element.name.as_str(), "img" | "input" | "svg")
}

fn push_inline_spacing<'doc>(out: &mut Vec<InlineToken<'doc>>, width: i32) {
    let width = width.max(0);
    if width == 0 {
        return;
    }
    out.push(InlineToken::Spacer(Size { width, height: 0 }));
}

fn measure_inline_element_outer_size<'doc>(
    engine: &LayoutEngine<'_>,
    element: &'doc Element,
    style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    max_width: i32,
) -> Result<Size, String> {
    let max_width = max_width.max(0);
    let margin = style.margin;
    let available_border_width = max_width
        .saturating_sub(margin.left.saturating_add(margin.right))
        .max(0);

    let padding = style.padding.resolve_px(max_width);
    let inset = super::add_edges(style.border_width, padding);
    let horizontal_inset = inset.left.saturating_add(inset.right);
    let vertical_inset = inset.top.saturating_add(inset.bottom);

    let mut border_width = if let Some(width) = style.width_px {
        width.resolve_px(max_width).max(0)
    } else {
        let available_content_width = available_border_width
            .saturating_sub(horizontal_inset)
            .max(0);
        let nodes: Vec<&Node> = element.children.iter().collect();
        let content_size =
            measure_inline_nodes(engine, &nodes, style, ancestors, available_content_width)?;
        content_size.width.saturating_add(horizontal_inset)
    };

    if let Some(min_width) = style.min_width_px {
        border_width = border_width.max(min_width.resolve_px(max_width).max(0));
    }
    if let Some(max_width_value) = style.max_width_px {
        border_width = border_width.min(max_width_value.resolve_px(max_width).max(0));
    }
    border_width = border_width.min(available_border_width).max(0);

    let mut border_height = if let Some(height) = style.height_px {
        height.max(0)
    } else {
        let available_content_width = border_width.saturating_sub(horizontal_inset).max(0);
        let nodes: Vec<&Node> = element.children.iter().collect();
        let content_size =
            measure_inline_nodes(engine, &nodes, style, ancestors, available_content_width)?;
        content_size.height.saturating_add(vertical_inset)
    };

    if let Some(min_height) = style.min_height_px {
        border_height = border_height.max(min_height.max(0));
    }

    Ok(Size {
        width: margin
            .left
            .saturating_add(border_width)
            .saturating_add(margin.right),
        height: margin
            .top
            .saturating_add(border_height)
            .saturating_add(margin.bottom),
    })
}

pub(super) fn measure_replaced_element_outer_size(
    element: &Element,
    style: &ComputedStyle,
    max_width: i32,
) -> Result<Size, String> {
    let max_width = max_width.max(0);
    let margin = style.margin;
    let available_border_width = max_width
        .saturating_sub(margin.left.saturating_add(margin.right))
        .max(0);

    let padding = style.padding.resolve_px(max_width);
    let inset = super::add_edges(style.border_width, padding);
    let horizontal_inset = inset.left.saturating_add(inset.right);
    let vertical_inset = inset.top.saturating_add(inset.bottom);

    let mut content_width = style.width_px.map(|width| {
        width
            .resolve_px(max_width)
            .max(0)
            .saturating_sub(horizontal_inset)
            .max(0)
    });
    let mut content_height = style
        .height_px
        .map(|height| height.max(0).saturating_sub(vertical_inset).max(0));

    let (intrinsic_width, intrinsic_height) = intrinsic_dimensions(element, style);
    let ratio = intrinsic_aspect_ratio(element, intrinsic_width, intrinsic_height);

    match (content_width, content_height) {
        (Some(_), Some(_)) => {}
        (Some(width), None) => {
            if let Some(ratio) = ratio {
                content_height = Some(((width as f32) / ratio).round() as i32);
            } else {
                content_height = intrinsic_height;
            }
        }
        (None, Some(height)) => {
            if let Some(ratio) = ratio {
                content_width = Some(((height as f32) * ratio).round() as i32);
            } else {
                content_width = intrinsic_width;
            }
        }
        (None, None) => {
            content_width = intrinsic_width;
            content_height = intrinsic_height;
        }
    }

    let mut border_width = content_width
        .unwrap_or(0)
        .max(0)
        .saturating_add(horizontal_inset);
    let mut border_height = content_height
        .unwrap_or(0)
        .max(0)
        .saturating_add(vertical_inset);

    if let Some(min_width) = style.min_width_px {
        border_width = border_width.max(min_width.resolve_px(max_width).max(0));
    }
    if let Some(max_width_value) = style.max_width_px {
        border_width = border_width.min(max_width_value.resolve_px(max_width).max(0));
    }
    border_width = border_width.min(available_border_width).max(0);

    if let Some(min_height) = style.min_height_px {
        border_height = border_height.max(min_height.max(0));
    }

    Ok(Size {
        width: margin
            .left
            .saturating_add(border_width)
            .saturating_add(margin.right),
        height: margin
            .top
            .saturating_add(border_height)
            .saturating_add(margin.bottom),
    })
}

fn intrinsic_dimensions(element: &Element, style: &ComputedStyle) -> (Option<i32>, Option<i32>) {
    let mut width = element
        .attributes
        .get("width")
        .and_then(|value| parse_dimension(value))
        .and_then(|value| i32::try_from(value.round() as i64).ok())
        .filter(|value| *value > 0);
    let mut height = element
        .attributes
        .get("height")
        .and_then(|value| parse_dimension(value))
        .and_then(|value| i32::try_from(value.round() as i64).ok())
        .filter(|value| *value > 0);

    if element.name == "svg" {
        if let Some((w, h)) = parse_svg_viewbox_dimensions(element.attributes.get("viewbox")) {
            if width.is_none() {
                width = Some(w.round() as i32);
            }
            if height.is_none() {
                height = Some(h.round() as i32);
            }
        }
    }

    if element.name == "input" {
        let (default_width, default_height) = intrinsic_input_content_dimensions(element, style);
        if width.is_none() {
            width = default_width;
        }
        if height.is_none() {
            height = default_height;
        }
    }

    (width, height)
}

fn intrinsic_input_content_dimensions(
    element: &Element,
    style: &ComputedStyle,
) -> (Option<i32>, Option<i32>) {
    let font_size_px = style.font_size_px.max(0);
    let line_height_px = style
        .line_height
        .resolve_px(font_size_px)
        .unwrap_or(font_size_px)
        .max(0)
        .max(1);

    let input_type = element
        .attributes
        .get("type")
        .unwrap_or("text")
        .trim()
        .to_ascii_lowercase();

    let width = match input_type.as_str() {
        "submit" | "button" | "reset" => {
            let mut label = element.attributes.get("value").unwrap_or("").trim();
            if label.is_empty() {
                label = match input_type.as_str() {
                    "reset" => "Reset",
                    _ => "Submit",
                };
            }
            let chars = label.chars().count() as i32;
            let approximate_char_width_px = ((font_size_px as f32) * 0.6).round() as i32;
            let letter_spacing_px = style.letter_spacing_px.max(0);
            let spacing_px = letter_spacing_px
                .saturating_mul(chars.saturating_sub(1))
                .max(0);
            let text_px = approximate_char_width_px
                .saturating_mul(chars)
                .saturating_add(spacing_px);
            Some(
                text_px
                    .saturating_add(font_size_px / 2)
                    .max(font_size_px * 2)
                    .max(1),
            )
        }
        _ => Some(font_size_px.saturating_mul(10).max(80).max(1)),
    };

    (width, Some(line_height_px))
}

fn intrinsic_aspect_ratio(
    element: &Element,
    intrinsic_width: Option<i32>,
    intrinsic_height: Option<i32>,
) -> Option<f32> {
    if element.name == "input" {
        return None;
    }
    if element.name == "svg" {
        if let Some((w, h)) = parse_svg_viewbox_dimensions(element.attributes.get("viewbox")) {
            if h > 0.0 {
                return Some(w / h);
            }
        }
    }

    let w = intrinsic_width? as f32;
    let h = intrinsic_height? as f32;
    if h <= 0.0 {
        return None;
    }
    Some(w / h)
}

fn parse_number(value: &str) -> Option<f32> {
    value.trim().parse::<f32>().ok()
}

fn parse_dimension(value: &str) -> Option<f32> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.ends_with('%') {
        return None;
    }

    let number = trimmed
        .strip_suffix("px")
        .or_else(|| trimmed.strip_suffix("PX"))
        .unwrap_or(trimmed);
    parse_number(number)
}

fn parse_svg_viewbox_dimensions(view_box: Option<&str>) -> Option<(f32, f32)> {
    let view_box = view_box?.trim();
    if view_box.is_empty() {
        return None;
    }

    let parts: Vec<&str> = view_box
        .split(|ch: char| ch.is_ascii_whitespace() || ch == ',')
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() != 4 {
        return None;
    }

    let width = parse_number(parts[2])?;
    let height = parse_number(parts[3])?;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    Some((width, height))
}

pub(super) fn serialize_element_xml(element: &Element) -> String {
    super::svg_xml::serialize_element_xml(element)
}

fn push_text<'doc>(
    text: &str,
    style: TextStyle,
    visible: bool,
    link_href: Option<Rc<str>>,
    cursor: &mut InlineCursor,
    out: &mut Vec<InlineToken<'doc>>,
) {
    let mut iter = text.chars().peekable();
    while let Some(ch) = iter.next() {
        if ch.is_whitespace() {
            cursor.mark_pending_space(style, visible, link_href.clone());
            continue;
        }

        cursor.flush_pending_space(out);

        let mut word = String::new();
        word.push(ch);
        while let Some(&next) = iter.peek() {
            if next.is_whitespace() {
                break;
            }
            word.push(next);
            iter.next();
        }
        out.push(InlineToken::Word(word, style, visible, link_href.clone()));
    }
}

fn layout_tokens<'doc>(
    engine: &mut LayoutEngine<'_>,
    tokens: &[InlineToken<'doc>],
    parent_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    content_box: Rect,
    start_y: i32,
    paint: bool,
) -> Result<i32, String> {
    let mut lines: Vec<Line<'doc>> = Vec::new();
    let base_style = engine.text_style_for(parent_style);
    let base_metrics = engine.measurer.font_metrics_px(base_style);
    let explicit_line_height_px = parent_style
        .line_height
        .resolve_px(parent_style.font_size_px)
        .map(|value| value.max(1));
    let mut line = Line::new(explicit_line_height_px, base_metrics);
    let mut x_px = 0i32;

    for token in tokens {
        match token {
            InlineToken::Newline => {
                lines.push(std::mem::replace(
                    &mut line,
                    Line::new(explicit_line_height_px, base_metrics),
                ));
                x_px = 0;
            }
            InlineToken::Space(style, visible, link_href) => {
                if x_px == 0 {
                    continue;
                }
                let space_width_px = engine.measurer.text_width_px(" ", *style)?;
                if x_px.saturating_add(space_width_px) > content_box.width {
                    continue;
                }
                let metrics = engine.measurer.font_metrics_px(*style);
                line.push(Fragment::Text(
                    " ".to_owned(),
                    *style,
                    space_width_px,
                    metrics,
                    *visible,
                    link_href.clone(),
                ));
                x_px = x_px.saturating_add(space_width_px);
            }
            InlineToken::Word(text, style, visible, link_href) => {
                if text.is_empty() {
                    continue;
                }
                let word_width_px = engine.measurer.text_width_px(text, *style)?;
                if x_px != 0 && x_px.saturating_add(word_width_px) > content_box.width {
                    lines.push(std::mem::replace(
                        &mut line,
                        Line::new(explicit_line_height_px, base_metrics),
                    ));
                    x_px = 0;
                }

                let metrics = engine.measurer.font_metrics_px(*style);
                line.push(Fragment::Text(
                    text.clone(),
                    *style,
                    word_width_px,
                    metrics,
                    *visible,
                    link_href.clone(),
                ));
                x_px = x_px.saturating_add(word_width_px);
            }
            InlineToken::Spacer(size) => {
                line.push(Fragment::Spacer(*size));
                x_px = x_px.saturating_add(size.width);
            }
            InlineToken::ElementBox(b) => {
                if x_px != 0 && x_px.saturating_add(b.size.width) > content_box.width {
                    lines.push(std::mem::replace(
                        &mut line,
                        Line::new(explicit_line_height_px, base_metrics),
                    ));
                    x_px = 0;
                }
                line.push(Fragment::ElementBox(b.clone()));
                x_px = x_px.saturating_add(b.size.width);
            }
        }
    }

    if !line.fragments.is_empty() {
        lines.push(line);
    }

    let mut y_px = start_y;
    for line in lines {
        let line_width = line.width_px;
        let align = parent_style.text_align;
        let x_offset = match align {
            TextAlign::Left => 0,
            TextAlign::Center => ((content_box.width - line_width) / 2).max(0),
            TextAlign::Right => (content_box.width - line_width).max(0),
        };

        let baseline_y = y_px.saturating_add(line.baseline_offset_px());
        let mut x_px = content_box.x.saturating_add(x_offset);
        for frag in line.fragments {
            match frag {
                Fragment::Text(text, style, width, _metrics, visible, link_href) => {
                    if paint && visible {
                        engine.list.commands.push(DisplayCommand::Text(DrawText {
                            x_px,
                            y_px: baseline_y,
                            text,
                            style,
                        }));
                        if let Some(href) = link_href {
                            engine.link_regions.push(LinkHitRegion {
                                href,
                                x_px,
                                y_px,
                                width_px: width,
                                height_px: line.height_px,
                                is_fixed: engine.fixed_depth > 0,
                            });
                        }
                    }
                    x_px = x_px.saturating_add(width);
                }
                Fragment::Spacer(size) => {
                    x_px = x_px.saturating_add(size.width);
                }
                Fragment::ElementBox(element_box) => {
                    let border_width = element_box
                        .size
                        .width
                        .saturating_sub(
                            element_box
                                .style
                                .margin
                                .left
                                .saturating_add(element_box.style.margin.right),
                        )
                        .max(0);
                    let border_height = element_box
                        .size
                        .height
                        .saturating_sub(
                            element_box
                                .style
                                .margin
                                .top
                                .saturating_add(element_box.style.margin.bottom),
                        )
                        .max(0);
                    let border_box = Rect {
                        x: x_px.saturating_add(element_box.style.margin.left),
                        y: y_px.saturating_add(element_box.style.margin.top),
                        width: border_width,
                        height: border_height,
                    };

                    let mut element_paint = paint && element_box.visible;
                    if element_paint && element_box.style.opacity == 0 {
                        element_paint = false;
                    }
                    let opacity = element_box.style.opacity;
                    let needs_opacity_group = element_paint && opacity < 255;
                    if needs_opacity_group {
                        engine
                            .list
                            .commands
                            .push(DisplayCommand::PushOpacity(opacity));
                    }

                    if element_paint {
                        let _ = engine.push_background(
                            border_box,
                            &element_box.style,
                            border_box.height,
                        );

                        engine.paint_border(border_box, &element_box.style);

                        if is_replaced_element(element_box.element) {
                            let padding = element_box.style.padding.resolve_px(content_box.width);
                            let content_box = border_box
                                .inset(super::add_edges(element_box.style.border_width, padding));
                            engine.paint_replaced_content(
                                element_box.element,
                                &element_box.style,
                                content_box,
                            )?;
                        }

                        if let Some(href) = element_box.link_href.clone() {
                            engine.link_regions.push(LinkHitRegion {
                                href,
                                x_px: border_box.x,
                                y_px: border_box.y,
                                width_px: border_box.width,
                                height_px: border_box.height,
                                is_fixed: engine.fixed_depth > 0,
                            });
                        }
                    }

                    if !is_replaced_element(element_box.element) {
                        let padding = element_box.style.padding.resolve_px(content_box.width);
                        let content_box = border_box
                            .inset(super::add_edges(element_box.style.border_width, padding));
                        ancestors.push(element_box.element);
                        engine.layout_flow_children(
                            &element_box.element.children,
                            &element_box.style,
                            ancestors,
                            content_box,
                            element_paint,
                        )?;
                        ancestors.pop();
                    }

                    if needs_opacity_group {
                        engine
                            .list
                            .commands
                            .push(DisplayCommand::PopOpacity(opacity));
                    }

                    x_px = x_px.saturating_add(element_box.size.width);
                }
            }
        }

        y_px = y_px.saturating_add(line.height_px);
    }

    Ok(y_px.saturating_sub(start_y).max(0))
}

fn measure_tokens<'doc>(
    engine: &LayoutEngine<'_>,
    tokens: &[InlineToken<'doc>],
    parent_style: &ComputedStyle,
    max_width: i32,
) -> Result<Size, String> {
    let max_width = max_width.max(0);
    let mut lines: Vec<Line<'doc>> = Vec::new();
    let base_style = engine.text_style_for(parent_style);
    let base_metrics = engine.measurer.font_metrics_px(base_style);
    let explicit_line_height_px = parent_style
        .line_height
        .resolve_px(parent_style.font_size_px)
        .map(|value| value.max(1));
    let mut line = Line::new(explicit_line_height_px, base_metrics);
    let mut x_px = 0i32;

    for token in tokens {
        match token {
            InlineToken::Newline => {
                lines.push(std::mem::replace(
                    &mut line,
                    Line::new(explicit_line_height_px, base_metrics),
                ));
                x_px = 0;
            }
            InlineToken::Space(style, _visible, _link_href) => {
                if x_px == 0 {
                    continue;
                }
                let space_width_px = engine.measurer.text_width_px(" ", *style)?;
                if x_px.saturating_add(space_width_px) > max_width {
                    continue;
                }
                let metrics = engine.measurer.font_metrics_px(*style);
                line.push(Fragment::Text(
                    " ".to_owned(),
                    *style,
                    space_width_px,
                    metrics,
                    false,
                    None,
                ));
                x_px = x_px.saturating_add(space_width_px);
            }
            InlineToken::Word(text, style, _visible, _link_href) => {
                if text.is_empty() {
                    continue;
                }
                let word_width_px = engine.measurer.text_width_px(text, *style)?;
                if x_px != 0 && x_px.saturating_add(word_width_px) > max_width {
                    lines.push(std::mem::replace(
                        &mut line,
                        Line::new(explicit_line_height_px, base_metrics),
                    ));
                    x_px = 0;
                }

                let metrics = engine.measurer.font_metrics_px(*style);
                line.push(Fragment::Text(
                    text.clone(),
                    *style,
                    word_width_px,
                    metrics,
                    false,
                    None,
                ));
                x_px = x_px.saturating_add(word_width_px);
            }
            InlineToken::Spacer(size) => {
                line.push(Fragment::Spacer(*size));
                x_px = x_px.saturating_add(size.width);
            }
            InlineToken::ElementBox(b) => {
                if x_px != 0 && x_px.saturating_add(b.size.width) > max_width {
                    lines.push(std::mem::replace(
                        &mut line,
                        Line::new(explicit_line_height_px, base_metrics),
                    ));
                    x_px = 0;
                }
                line.push(Fragment::ElementBox(b.clone()));
                x_px = x_px.saturating_add(b.size.width);
            }
        }
    }

    if !line.fragments.is_empty() {
        lines.push(line);
    }

    let mut width_px = 0i32;
    let mut height_px = 0i32;
    for line in lines {
        width_px = width_px.max(line.width_px);
        height_px = height_px.saturating_add(line.height_px);
    }

    Ok(Size {
        width: width_px.max(0),
        height: height_px.max(0),
    })
}

#[derive(Clone, Debug)]
enum Fragment<'doc> {
    Text(String, TextStyle, i32, FontMetricsPx, bool, Option<Rc<str>>),
    Spacer(Size),
    ElementBox(InlineElementBox<'doc>),
}

struct Line<'doc> {
    fragments: Vec<Fragment<'doc>>,
    width_px: i32,
    ascent_px: i32,
    descent_px: i32,
    height_px: i32,
    max_element_height_px: i32,
    explicit_line_height_px: Option<i32>,
}

impl<'doc> Line<'doc> {
    fn new(explicit_line_height_px: Option<i32>, base_metrics: FontMetricsPx) -> Line<'doc> {
        let ascent_px = base_metrics.ascent_px.max(1);
        let descent_px = base_metrics.descent_px.max(0);
        let text_height_px = ascent_px.saturating_add(descent_px).max(1);
        let height_px = explicit_line_height_px.unwrap_or(text_height_px).max(1);
        let mut line = Line {
            fragments: Vec::new(),
            width_px: 0,
            ascent_px,
            descent_px,
            height_px,
            max_element_height_px: 0,
            explicit_line_height_px,
        };
        line.recompute_height();
        line
    }

    fn push(&mut self, fragment: Fragment<'doc>) {
        match &fragment {
            Fragment::Text(_, _, width, metrics, _, _) => {
                self.width_px = self.width_px.saturating_add(*width);
                self.ascent_px = self.ascent_px.max(metrics.ascent_px.max(1));
                self.descent_px = self.descent_px.max(metrics.descent_px.max(0));
            }
            Fragment::Spacer(size) => {
                self.width_px = self.width_px.saturating_add(size.width);
            }
            Fragment::ElementBox(element_box) => {
                self.width_px = self.width_px.saturating_add(element_box.size.width);
                self.max_element_height_px = self
                    .max_element_height_px
                    .max(element_box.size.height.max(1));
            }
        }
        self.recompute_height();
        self.fragments.push(fragment);
    }

    fn recompute_height(&mut self) {
        let text_height_px = self.ascent_px.saturating_add(self.descent_px).max(1);
        let base_height_px = self
            .explicit_line_height_px
            .unwrap_or(text_height_px)
            .max(1);
        self.height_px = base_height_px.max(self.max_element_height_px).max(1);
    }

    fn baseline_offset_px(&self) -> i32 {
        let text_height_px = self.ascent_px.saturating_add(self.descent_px).max(1);
        let extra = self.height_px.saturating_sub(text_height_px).max(0);
        self.ascent_px.saturating_add(extra / 2)
    }
}
