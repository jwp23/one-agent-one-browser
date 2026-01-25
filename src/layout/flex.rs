use crate::dom::{Element, Node};
use crate::geom::{Rect, Size};
use crate::render::DrawRoundedRect;
use crate::style::{
    ComputedStyle, Display, FlexAlignItems, FlexDirection, FlexJustifyContent, FlexWrap, Position,
    Visibility,
};
use std::rc::Rc;

use super::{inline, table, LayoutEngine};

pub(super) fn layout_flex_row<'doc>(
    engine: &mut LayoutEngine<'_>,
    element: &'doc Element,
    style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    content_box: Rect,
    paint: bool,
) -> Result<i32, String> {
    if style.display != Display::Flex {
        return Ok(0);
    }

    match style.flex_direction {
        FlexDirection::Row => layout_flex_row_container(engine, element, style, ancestors, content_box, paint),
        FlexDirection::Column => {
            layout_flex_column_container(engine, element, style, ancestors, content_box, paint)
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum FlexNode<'doc> {
    Text(&'doc Node),
    Element(&'doc Element),
}

struct FlexItem<'doc> {
    node: FlexNode<'doc>,
    style: ComputedStyle,
    margin: crate::geom::Edges,
}

fn layout_flex_row_container<'doc>(
    engine: &mut LayoutEngine<'_>,
    element: &'doc Element,
    style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    content_box: Rect,
    paint: bool,
) -> Result<i32, String> {
    if content_box.width <= 0 {
        layout_positioned_children(engine, element, style, ancestors, content_box, paint)?;
        return Ok(0);
    }

    let items = collect_items(engine, element, style, ancestors)?;
    if items.is_empty() {
        layout_positioned_children(engine, element, style, ancestors, content_box, paint)?;
        return Ok(0);
    }

    let height = match style.flex_wrap {
        FlexWrap::NoWrap => layout_flex_row_single_line(engine, style, ancestors, content_box, paint, &items),
        FlexWrap::Wrap => layout_flex_row_wrapped(engine, style, ancestors, content_box, paint, &items),
    }?;

    layout_positioned_children(engine, element, style, ancestors, content_box, paint)?;
    Ok(height)
}

fn layout_flex_row_single_line<'doc>(
    engine: &mut LayoutEngine<'_>,
    container_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    content_box: Rect,
    paint: bool,
    items: &[FlexItem<'doc>],
) -> Result<i32, String> {
    let mut sizes = Vec::with_capacity(items.len());
    for item in items {
        let main_size = measure_item_main_size_row(engine, container_style, ancestors, item, content_box.width)?;
        let border_width = main_size.clamp(0, content_box.width);
        let border_height =
            measure_item_border_height(engine, container_style, ancestors, item, border_width)?;
        sizes.push(Size {
            width: border_width,
            height: border_height,
        });
    }

    distribute_flex_grow_row(container_style, items, content_box.width, &mut sizes);

    let mut outer_heights: Vec<i32> = Vec::with_capacity(items.len());
    for (item, size) in items.iter().zip(&sizes) {
        outer_heights.push(
            item.margin
                .top
                .saturating_add(size.height)
                .saturating_add(item.margin.bottom),
        );
    }
    let line_height = outer_heights.iter().copied().max().unwrap_or(0).max(0);

    let positions = compute_main_positions(
        container_style.flex_justify_content,
        content_box.width,
        container_style.flex_gap_px,
        items,
        &sizes,
    );

    for ((item, size), x_offset) in items.iter().zip(&sizes).zip(positions) {
        let outer_x = content_box.x.saturating_add(x_offset);
        let border_x = outer_x.saturating_add(item.margin.left);
        let border_y = align_cross_start(
            container_style.flex_align_items,
            content_box.y,
            line_height,
            size.height,
            item.margin.top,
            item.margin.bottom,
        );

        layout_item_box(
            engine,
            container_style,
            ancestors,
            item,
            Rect {
                x: border_x,
                y: border_y,
                width: size.width,
                height: size.height,
            },
            paint,
        )?;
    }

    Ok(line_height.max(0))
}

fn layout_flex_row_wrapped<'doc>(
    engine: &mut LayoutEngine<'_>,
    container_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    content_box: Rect,
    paint: bool,
    items: &[FlexItem<'doc>],
) -> Result<i32, String> {
    let gap = container_style.flex_gap_px.max(0);
    let mut cursor_y = content_box.y;
    let mut line_start = 0usize;
    let mut line_used = 0i32;

    let mut measured_main: Vec<i32> = Vec::with_capacity(items.len());
    for item in items {
        measured_main.push(measure_item_main_size_row(engine, container_style, ancestors, item, content_box.width)?);
    }

    for (idx, item) in items.iter().enumerate() {
        let outer = item
            .margin
            .left
            .saturating_add(measured_main[idx].max(0))
            .saturating_add(item.margin.right);
        let addition = if idx == line_start { outer } else { gap.saturating_add(outer) };
        if line_used > 0 && line_used.saturating_add(addition) > content_box.width {
            let height = layout_flex_row_line(
                engine,
                container_style,
                ancestors,
                Rect {
                    x: content_box.x,
                    y: cursor_y,
                    width: content_box.width,
                    height: content_box.height,
                },
                paint,
                &items[line_start..idx],
                &measured_main[line_start..idx],
            )?;
            cursor_y = cursor_y.saturating_add(height);
            line_start = idx;
            line_used = outer;
        } else {
            line_used = line_used.saturating_add(addition);
        }
    }

    if line_start < items.len() {
        let height = layout_flex_row_line(
            engine,
            container_style,
            ancestors,
            Rect {
                x: content_box.x,
                y: cursor_y,
                width: content_box.width,
                height: content_box.height,
            },
            paint,
            &items[line_start..],
            &measured_main[line_start..],
        )?;
        cursor_y = cursor_y.saturating_add(height);
    }

    Ok(cursor_y.saturating_sub(content_box.y).max(0))
}

fn layout_flex_row_line<'doc>(
    engine: &mut LayoutEngine<'_>,
    container_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    line_box: Rect,
    paint: bool,
    line_items: &[FlexItem<'doc>],
    measured_main_sizes: &[i32],
) -> Result<i32, String> {
    if line_items.is_empty() || line_box.width <= 0 {
        return Ok(0);
    }

    let mut sizes = Vec::with_capacity(line_items.len());
    for (item, &main_size) in line_items.iter().zip(measured_main_sizes) {
        let border_width = main_size.clamp(0, line_box.width);
        let border_height =
            measure_item_border_height(engine, container_style, ancestors, item, border_width)?;
        sizes.push(Size {
            width: border_width,
            height: border_height,
        });
    }

    distribute_flex_grow_row(container_style, line_items, line_box.width, &mut sizes);

    let mut line_height = 0i32;
    for (item, size) in line_items.iter().zip(&sizes) {
        let outer = item
            .margin
            .top
            .saturating_add(size.height)
            .saturating_add(item.margin.bottom);
        line_height = line_height.max(outer);
    }
    line_height = line_height.max(0);

    let positions = compute_main_positions(
        container_style.flex_justify_content,
        line_box.width,
        container_style.flex_gap_px,
        line_items,
        &sizes,
    );

    for ((item, size), x_offset) in line_items.iter().zip(&sizes).zip(positions) {
        let outer_x = line_box.x.saturating_add(x_offset);
        let border_x = outer_x.saturating_add(item.margin.left);
        let border_y = align_cross_start(
            container_style.flex_align_items,
            line_box.y,
            line_height,
            size.height,
            item.margin.top,
            item.margin.bottom,
        );
        layout_item_box(
            engine,
            container_style,
            ancestors,
            item,
            Rect {
                x: border_x,
                y: border_y,
                width: size.width,
                height: size.height,
            },
            paint,
        )?;
    }

    Ok(line_height.max(0))
}

fn layout_flex_column_container<'doc>(
    engine: &mut LayoutEngine<'_>,
    element: &'doc Element,
    style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    content_box: Rect,
    paint: bool,
) -> Result<i32, String> {
    let items = collect_items(engine, element, style, ancestors)?;
    if items.is_empty() {
        layout_positioned_children(engine, element, style, ancestors, content_box, paint)?;
        return Ok(0);
    }

    let mut cursor_y = content_box.y;
    let gap = style.flex_gap_px.max(0);

    for (idx, item) in items.iter().enumerate() {
        if cursor_y >= engine.viewport.height_px {
            break;
        }

        let border_width = resolve_column_item_width(content_box.width, item);
        let border_height =
            measure_item_border_height(engine, style, ancestors, item, border_width)?;

        let aligned_x = align_column_cross_start(
            style.flex_align_items,
            content_box.x,
            content_box.width,
            border_width,
            item.margin.left,
            item.margin.right,
        );

        layout_item_box(
            engine,
            style,
            ancestors,
            item,
            Rect {
                x: aligned_x,
                y: cursor_y.saturating_add(item.margin.top),
                width: border_width,
                height: border_height,
            },
            paint,
        )?;

        cursor_y = cursor_y
            .saturating_add(item.margin.top)
            .saturating_add(border_height)
            .saturating_add(item.margin.bottom);
        if idx + 1 < items.len() {
            cursor_y = cursor_y.saturating_add(gap);
        }
    }

    layout_positioned_children(engine, element, style, ancestors, content_box, paint)?;
    Ok(cursor_y.saturating_sub(content_box.y).max(0))
}

fn align_cross_start(
    align: FlexAlignItems,
    line_y: i32,
    line_height: i32,
    item_height: i32,
    margin_top: i32,
    margin_bottom: i32,
) -> i32 {
    let item_height = item_height.max(0);
    let line_height = line_height.max(0);
    match align {
        FlexAlignItems::Start => line_y.saturating_add(margin_top),
        FlexAlignItems::Center => {
            let remaining = line_height
                .saturating_sub(margin_top.saturating_add(item_height).saturating_add(margin_bottom))
                .max(0);
            line_y
                .saturating_add(margin_top)
                .saturating_add(remaining / 2)
        }
        FlexAlignItems::End => {
            let remaining = line_height
                .saturating_sub(margin_top.saturating_add(item_height).saturating_add(margin_bottom))
                .max(0);
            line_y
                .saturating_add(margin_top)
                .saturating_add(remaining)
        }
    }
}

fn align_column_cross_start(
    align: FlexAlignItems,
    container_x: i32,
    container_width: i32,
    item_width: i32,
    margin_left: i32,
    margin_right: i32,
) -> i32 {
    let available = container_width
        .saturating_sub(margin_left.saturating_add(margin_right))
        .max(0);
    let item_width = item_width.max(0);
    match align {
        FlexAlignItems::Start => container_x.saturating_add(margin_left),
        FlexAlignItems::Center => container_x
            .saturating_add(margin_left)
            .saturating_add((available.saturating_sub(item_width).max(0)) / 2),
        FlexAlignItems::End => container_x
            .saturating_add(margin_left)
            .saturating_add(available.saturating_sub(item_width).max(0)),
    }
}

fn collect_items<'doc>(
    engine: &LayoutEngine<'_>,
    element: &'doc Element,
    style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
) -> Result<Vec<FlexItem<'doc>>, String> {
    let mut items = Vec::new();
    for child in &element.children {
        match child {
            Node::Text(text) => {
                if text.trim().is_empty() {
                    continue;
                }
                items.push(FlexItem {
                    node: FlexNode::Text(child),
                    style: *style,
                    margin: crate::geom::Edges::ZERO,
                });
            }
            Node::Element(el) => {
                let child_style = engine.styles.compute_style(el, style, ancestors);
                if child_style.display == Display::None {
                    continue;
                }
                if matches!(child_style.position, Position::Absolute | Position::Fixed) {
                    continue;
                }
                items.push(FlexItem {
                    node: FlexNode::Element(el),
                    style: child_style,
                    margin: child_style.margin,
                });
            }
        }
    }
    Ok(items)
}

fn layout_positioned_children<'doc>(
    engine: &mut LayoutEngine<'_>,
    element: &'doc Element,
    container_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    containing: Rect,
    paint: bool,
) -> Result<(), String> {
    for child in &element.children {
        let Node::Element(el) = child else { continue };
        let style = engine.styles.compute_style(el, container_style, ancestors);
        if style.display == Display::None {
            continue;
        }
        if matches!(style.position, Position::Absolute | Position::Fixed) {
            engine.layout_positioned_box(el, &style, ancestors, containing, paint)?;
        }
    }
    Ok(())
}

fn measure_item_main_size_row<'doc>(
    engine: &LayoutEngine<'_>,
    parent_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    item: &FlexItem<'doc>,
    max_width: i32,
) -> Result<i32, String> {
    let border_width = if let Some(basis) = item.style.flex_basis_px {
        basis
    } else if let Some(width) = item.style.width_px {
        width
    } else {
        match item.node {
            FlexNode::Text(node) => inline::measure_inline_nodes(
                engine,
                &[node],
                parent_style,
                ancestors,
                max_width,
            )?
            .width,
            FlexNode::Element(el) => measure_element_max_content_width(engine, el, &item.style, ancestors, max_width)?,
        }
    };

    let mut border_width = border_width.max(0);
    if let Some(min) = item.style.min_width_px {
        border_width = border_width.max(min.max(0));
    }
    if let Some(max) = item.style.max_width_px {
        border_width = border_width.min(max.max(0));
    }
    Ok(border_width.min(max_width.max(0)))
}

fn measure_element_max_content_width<'doc>(
    engine: &LayoutEngine<'_>,
    element: &'doc Element,
    style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    max_width: i32,
) -> Result<i32, String> {
    let max_width = max_width.max(0);
    if let Some(width) = style.width_px {
        return Ok(width.max(0).min(max_width));
    }

    if style.display == Display::Flex {
        return measure_flex_container_max_content_width(engine, element, style, ancestors, max_width);
    }

    if super::inline::is_replaced_element(element) {
        let size = super::inline::measure_replaced_element_outer_size(element, style, max_width)?;
        let border_width = size
            .width
            .saturating_sub(style.margin.left.saturating_add(style.margin.right))
            .max(0);
        return Ok(border_width.min(max_width.max(0)));
    }

    let is_block = super::is_flow_block(style, element);
    let mut width_px = 0i32;

    ancestors.push(element);
    if is_block {
        for child in &element.children {
            width_px = width_px.max(measure_node_max_content_width(
                engine,
                child,
                style,
                ancestors,
                max_width,
            )?);
        }
    } else {
        width_px = measure_inline_children_width(engine, &element.children, style, ancestors, max_width)?;
    }
    ancestors.pop();

    width_px = width_px
        .saturating_add(style.border_width.left)
        .saturating_add(style.border_width.right)
        .saturating_add(style.padding.left)
        .saturating_add(style.padding.right);

    Ok(width_px.max(0).min(max_width))
}

fn measure_flex_container_max_content_width<'doc>(
    engine: &LayoutEngine<'_>,
    element: &'doc Element,
    style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    max_width: i32,
) -> Result<i32, String> {
    let max_width = max_width.max(0);
    let gap = style.flex_gap_px.max(0);

    let mut primary = match style.flex_direction {
        FlexDirection::Row => 0i32,
        FlexDirection::Column => 0i32,
    };
    let mut has_any_item = false;

    ancestors.push(element);
    for child in &element.children {
        let (child_width, margin_left, margin_right) = match child {
            Node::Text(text) => {
                if text.trim().is_empty() {
                    continue;
                }
                let width = inline::measure_inline_nodes(
                    engine,
                    &[child],
                    style,
                    ancestors,
                    max_width,
                )?
                .width;
                (width, 0i32, 0i32)
            }
            Node::Element(el) => {
                let child_style = engine.styles.compute_style(el, style, ancestors);
                if child_style.display == Display::None {
                    continue;
                }

                let mut width = if let Some(basis) = child_style.flex_basis_px {
                    basis.max(0)
                } else if let Some(width) = child_style.width_px {
                    width.max(0)
                } else {
                    measure_element_max_content_width(engine, el, &child_style, ancestors, max_width)?
                };

                if let Some(min) = child_style.min_width_px {
                    width = width.max(min.max(0));
                }
                if let Some(max) = child_style.max_width_px {
                    width = width.min(max.max(0));
                }

                (width.min(max_width), child_style.margin.left, child_style.margin.right)
            }
        };

        let outer_width = margin_left
            .saturating_add(child_width.max(0))
            .saturating_add(margin_right);

        match style.flex_direction {
            FlexDirection::Row => {
                if has_any_item {
                    primary = primary.saturating_add(gap);
                }
                primary = primary.saturating_add(outer_width);
            }
            FlexDirection::Column => {
                primary = primary.max(outer_width);
            }
        }
        has_any_item = true;
    }
    ancestors.pop();

    let total = primary
        .saturating_add(style.border_width.left)
        .saturating_add(style.border_width.right)
        .saturating_add(style.padding.left)
        .saturating_add(style.padding.right);

    Ok(total.max(0).min(max_width))
}

fn measure_node_max_content_width<'doc>(
    engine: &LayoutEngine<'_>,
    node: &'doc Node,
    parent_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    max_width: i32,
) -> Result<i32, String> {
    match node {
        Node::Text(text) => measure_text_run_width(engine, text, engine.text_style_for(parent_style)),
        Node::Element(el) => {
            let style = engine.styles.compute_style(el, parent_style, ancestors);
            if style.display == Display::None {
                return Ok(0);
            }
            if let Some(width) = style.width_px {
                return Ok(width.max(0).min(max_width));
            }
            measure_element_max_content_width(engine, el, &style, ancestors, max_width)
        }
    }
}

fn measure_inline_children_width<'doc>(
    engine: &LayoutEngine<'_>,
    children: &'doc [Node],
    parent_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    max_width: i32,
) -> Result<i32, String> {
    let mut total = 0i32;
    let mut pending_space = false;

    for child in children {
        match child {
            Node::Text(text) => {
                let (segment_width, space_after) =
                    measure_text_run_width_with_pending_space(engine, text, engine.text_style_for(parent_style), pending_space)?;
                total = total.saturating_add(segment_width);
                pending_space = space_after;
            }
            Node::Element(el) => {
                let style = engine.styles.compute_style(el, parent_style, ancestors);
                if style.display == Display::None {
                    continue;
                }
                let width = if super::inline::is_replaced_element(el) {
                    let size = super::inline::measure_replaced_element_outer_size(el, &style, max_width)?;
                    size.width
                        .saturating_sub(style.margin.left.saturating_add(style.margin.right))
                        .max(0)
                } else if super::is_flow_block(&style, el) {
                    measure_element_max_content_width(engine, el, &style, ancestors, max_width)?
                } else {
                    measure_element_max_content_width(engine, el, &style, ancestors, max_width)?
                };

                if pending_space && width > 0 {
                    total = total.saturating_add(engine.measurer.text_width_px(" ", engine.text_style_for(parent_style))?);
                }
                pending_space = false;

                total = total
                    .saturating_add(width)
                    .saturating_add(style.margin.left)
                    .saturating_add(style.margin.right);
            }
        }
    }

    Ok(total.max(0).min(max_width))
}

fn measure_text_run_width(engine: &LayoutEngine<'_>, text: &str, style: crate::render::TextStyle) -> Result<i32, String> {
    let mut width = 0i32;
    let mut first = true;
    for word in text.split_whitespace() {
        if word.is_empty() {
            continue;
        }
        let word_width = engine.measurer.text_width_px(word, style)?;
        if !first {
            width = width.saturating_add(engine.measurer.text_width_px(" ", style)?);
        }
        first = false;
        width = width.saturating_add(word_width);
    }
    Ok(width.max(0))
}

fn measure_text_run_width_with_pending_space(
    engine: &LayoutEngine<'_>,
    text: &str,
    style: crate::render::TextStyle,
    pending_space: bool,
) -> Result<(i32, bool), String> {
    let mut width = 0i32;
    let mut any_word = false;
    let mut first_word = true;
    for word in text.split_whitespace() {
        if word.is_empty() {
            continue;
        }
        if pending_space && first_word {
            width = width.saturating_add(engine.measurer.text_width_px(" ", style)?);
        } else if any_word {
            width = width.saturating_add(engine.measurer.text_width_px(" ", style)?);
        }
        first_word = false;
        any_word = true;
        width = width.saturating_add(engine.measurer.text_width_px(word, style)?);
    }

    let ends_with_space = text.chars().last().is_some_and(|ch| ch.is_whitespace());
    Ok((width.max(0), ends_with_space))
}

fn distribute_flex_grow_row<'doc>(
    container_style: &ComputedStyle,
    items: &[FlexItem<'doc>],
    max_width: i32,
    sizes: &mut [Size],
) {
    let max_width = max_width.max(0);
    if max_width <= 0 || items.is_empty() {
        return;
    }

    let gap = container_style.flex_gap_px.max(0);
    let total_gap = gap.saturating_mul((items.len().saturating_sub(1)) as i32);

    let mut total_outer = total_gap;
    for (item, size) in items.iter().zip(sizes.iter()) {
        total_outer = total_outer
            .saturating_add(item.margin.left)
            .saturating_add(size.width)
            .saturating_add(item.margin.right);
    }

    let remaining = max_width.saturating_sub(total_outer).max(0);
    if remaining == 0 {
        return;
    }

    let mut total_grow = 0i32;
    for item in items {
        total_grow = total_grow.saturating_add(item.style.flex_grow.max(0));
    }
    if total_grow <= 0 {
        return;
    }

    let mut distributed = 0i32;
    for (idx, (item, size)) in items.iter().zip(sizes.iter_mut()).enumerate() {
        let grow = item.style.flex_grow.max(0);
        if grow == 0 {
            continue;
        }
        let extra = if idx + 1 == items.len() {
            remaining.saturating_sub(distributed)
        } else {
            (remaining as i64 * grow as i64 / total_grow as i64) as i32
        };
        distributed = distributed.saturating_add(extra);
        size.width = size.width.saturating_add(extra).min(max_width);
    }
}

fn compute_main_positions<'doc>(
    justify: FlexJustifyContent,
    max_width: i32,
    gap_px: i32,
    items: &[FlexItem<'doc>],
    sizes: &[Size],
) -> Vec<i32> {
    let gap = gap_px.max(0);
    if items.is_empty() || sizes.is_empty() {
        return Vec::new();
    }

    let mut total_outer = 0i32;
    for (item, size) in items.iter().zip(sizes.iter()) {
        total_outer = total_outer
            .saturating_add(item.margin.left)
            .saturating_add(size.width)
            .saturating_add(item.margin.right);
    }
    total_outer = total_outer.saturating_add(gap.saturating_mul((items.len().saturating_sub(1)) as i32));

    let remaining = max_width.saturating_sub(total_outer).max(0);

    let (start_offset, spacing) = match justify {
        FlexJustifyContent::Start => (0, gap),
        FlexJustifyContent::Center => (remaining / 2, gap),
        FlexJustifyContent::End => (remaining, gap),
        FlexJustifyContent::SpaceBetween => {
            if items.len() <= 1 {
                (0, gap)
            } else {
                (0, gap.saturating_add(remaining / (items.len().saturating_sub(1)) as i32))
            }
        }
    };

    let mut positions = Vec::with_capacity(items.len());
    let mut cursor = start_offset.max(0);
    for (idx, (item, size)) in items.iter().zip(sizes.iter()).enumerate() {
        positions.push(cursor);
        cursor = cursor
            .saturating_add(item.margin.left)
            .saturating_add(size.width)
            .saturating_add(item.margin.right);
        if idx + 1 < items.len() {
            cursor = cursor.saturating_add(spacing);
        }
    }

    positions
}

fn measure_item_border_height<'doc>(
    engine: &mut LayoutEngine<'_>,
    parent_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    item: &FlexItem<'doc>,
    border_width: i32,
) -> Result<i32, String> {
    let border_width = border_width.max(0);
    match item.node {
        FlexNode::Text(node) => inline::measure_inline_nodes(
            engine,
            &[node],
            parent_style,
            ancestors,
            border_width,
        )
        .map(|s| s.height.max(0)),
        FlexNode::Element(_el) => {
            let border_height = layout_item_box(
                engine,
                parent_style,
                ancestors,
                item,
                Rect {
                    x: 0,
                    y: 0,
                    width: border_width,
                    height: engine.viewport.height_px,
                },
                false,
            )?;
            Ok(border_height.max(0))
        }
    }
}

fn resolve_column_item_width(container_width: i32, item: &FlexItem<'_>) -> i32 {
    if let Some(width) = item.style.width_px {
        return width.max(0).min(container_width.max(0));
    }
    container_width.max(0)
}

fn layout_item_box<'doc>(
    engine: &mut LayoutEngine<'_>,
    parent_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    item: &FlexItem<'doc>,
    border_box: Rect,
    paint: bool,
) -> Result<i32, String> {
    if border_box.width <= 0 {
        return Ok(0);
    }

    let mut background_index = None;
    let mut paint = paint && item.style.visibility == Visibility::Visible;
    if paint && item.style.opacity == 0 {
        paint = false;
    }
    let opacity = item.style.opacity;
    let needs_opacity_group = paint && opacity < 255;
    if needs_opacity_group {
        engine.list.commands.push(crate::render::DisplayCommand::PushOpacity(opacity));
    }

    if paint {
        if let Some(color) = item.style.background_color {
            background_index = Some(engine.list.commands.len());
            if item.style.border_radius_px > 0 {
                engine
                    .list
                    .commands
                    .push(crate::render::DisplayCommand::RoundedRect(DrawRoundedRect {
                        x_px: border_box.x,
                        y_px: border_box.y,
                        width_px: border_box.width,
                        height_px: 0,
                        radius_px: item.style.border_radius_px,
                        color,
                    }));
            } else {
                engine.list.commands.push(crate::render::DisplayCommand::Rect(
                    crate::render::DrawRect {
                        x_px: border_box.x,
                        y_px: border_box.y,
                        width_px: border_box.width,
                        height_px: 0,
                        color,
                    },
                ));
            }
        }
    }

    let border = item.style.border_width;
    let padding = item.style.padding;
    let content_box = border_box.inset(super::add_edges(border, padding));

    let content_height = match item.node {
        FlexNode::Text(node) => inline::layout_inline_nodes(
            engine,
            &[node],
            parent_style,
            ancestors,
            content_box,
            content_box.y,
            paint,
        )?,
        FlexNode::Element(el) => {
            ancestors.push(el);
            let height = match item.style.display {
                Display::Table => table::layout_table(engine, el, &item.style, ancestors, content_box, paint)?.height,
                Display::Flex => layout_flex_row(engine, el, &item.style, ancestors, content_box, paint)?,
                Display::None => 0,
                _ => {
                    if el.name == "a" {
                        let nodes: Vec<&Node> = el.children.iter().collect();
                        inline::layout_inline_nodes_with_link(
                            engine,
                            &nodes,
                            &item.style,
                            ancestors,
                            content_box,
                            content_box.y,
                            paint,
                            anchor_href(el),
                        )?
                    } else {
                        engine.layout_flow_children(&el.children, &item.style, ancestors, content_box, paint)?
                    }
                }
            };
            ancestors.pop();
            height
        }
    };

    let mut border_height = border
        .top
        .saturating_add(padding.top)
        .saturating_add(content_height)
        .saturating_add(padding.bottom)
        .saturating_add(border.bottom)
        .max(0);
    if let Some(height) = item.style.height_px {
        border_height = border_height.max(height.max(0));
    }
    if let Some(min_height) = item.style.min_height_px {
        border_height = border_height.max(min_height.max(0));
    }

    if let Some(index) = background_index {
        if let Some(cmd) = engine.list.commands.get_mut(index) {
            match cmd {
                crate::render::DisplayCommand::Rect(rect) => rect.height_px = border_height,
                crate::render::DisplayCommand::RoundedRect(rect) => rect.height_px = border_height,
                _ => {}
            }
        }
    }

    if paint {
        engine.paint_border(
            Rect {
                x: border_box.x,
                y: border_box.y,
                width: border_box.width,
                height: border_height,
            },
            &item.style,
        );
    }

    if needs_opacity_group {
        engine.list.commands.push(crate::render::DisplayCommand::PopOpacity(opacity));
    }

    Ok(border_height)
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
