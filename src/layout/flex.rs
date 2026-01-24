use crate::dom::{Element, Node};
use crate::geom::{Rect, Size};
use crate::style::{ComputedStyle, Display, FlexAlignItems, FlexJustifyContent};

use super::inline;
use super::LayoutEngine;

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

    let items = collect_flex_items(element, style, engine, ancestors)?;
    if items.is_empty() || content_box.width <= 0 {
        return Ok(0);
    }

    let sizes = measure_items(engine, style, ancestors, content_box.width, &items)?;
    let row_height = sizes.iter().map(|size| size.height).max().unwrap_or(0);

    let x_positions = compute_x_positions(style, content_box.width, &sizes);
    for ((item, size), x_offset) in items.iter().zip(&sizes).zip(x_positions) {
        let item_x = content_box.x.saturating_add(x_offset);
        let item_y = match style.flex_align_items {
            FlexAlignItems::Center => content_box
                .y
                .saturating_add((row_height.saturating_sub(size.height)) / 2),
            FlexAlignItems::Start => content_box.y,
        };
        layout_item(engine, item, style, ancestors, item_x, item_y, size.width, paint)?;
    }

    Ok(row_height.max(0))
}

#[derive(Clone, Copy, Debug)]
enum FlexItem<'doc> {
    Text(&'doc Node),
    Element(&'doc Element),
}

fn collect_flex_items<'doc>(
    element: &'doc Element,
    style: &ComputedStyle,
    engine: &LayoutEngine<'_>,
    ancestors: &mut Vec<&'doc Element>,
) -> Result<Vec<FlexItem<'doc>>, String> {
    let mut items = Vec::new();
    for child in &element.children {
        match child {
            Node::Text(text) => {
                if text.trim().is_empty() {
                    continue;
                }
                items.push(FlexItem::Text(child));
            }
            Node::Element(el) => {
                let child_style = engine.styles.compute_style(el, style, ancestors);
                if child_style.display == Display::None {
                    continue;
                }
                items.push(FlexItem::Element(el));
            }
        }
    }
    Ok(items)
}

fn measure_items<'doc>(
    engine: &LayoutEngine<'_>,
    parent_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    max_width: i32,
    items: &[FlexItem<'doc>],
) -> Result<Vec<Size>, String> {
    let max_width = max_width.max(0);
    let mut sizes = Vec::with_capacity(items.len());
    for item in items {
        let size = match item {
            FlexItem::Text(node) => inline::measure_inline_nodes(
                engine,
                &[node],
                parent_style,
                ancestors,
                max_width,
            )?,
            FlexItem::Element(el) => {
                let child_style = engine.styles.compute_style(el, parent_style, ancestors);
                if child_style.display == Display::Flex {
                    ancestors.push(el);
                    let size = measure_flex_row(engine, el, &child_style, ancestors, max_width)?;
                    ancestors.pop();
                    size
                } else {
                    ancestors.push(el);
                    let nodes: Vec<&Node> = el.children.iter().collect();
                    let size = inline::measure_inline_nodes(engine, &nodes, &child_style, ancestors, max_width)?;
                    ancestors.pop();
                    size
                }
            }
        };
        sizes.push(Size {
            width: size.width.min(max_width).max(0),
            height: size.height.max(0),
        });
    }
    Ok(sizes)
}

fn measure_flex_row<'doc>(
    engine: &LayoutEngine<'_>,
    element: &'doc Element,
    style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    max_width: i32,
) -> Result<Size, String> {
    let items = collect_flex_items(element, style, engine, ancestors)?;
    if items.is_empty() || max_width <= 0 {
        return Ok(Size { width: 0, height: 0 });
    }

    let sizes = measure_items(engine, style, ancestors, max_width, &items)?;
    let gap = style.flex_gap_px.max(0);
    let total_gap = gap.saturating_mul((sizes.len().saturating_sub(1)) as i32);
    let content_width: i32 = sizes.iter().map(|s| s.width).sum::<i32>().saturating_add(total_gap);
    let content_height = sizes.iter().map(|s| s.height).max().unwrap_or(0);

    Ok(Size {
        width: content_width.min(max_width).max(0),
        height: content_height.max(0),
    })
}

fn compute_x_positions(style: &ComputedStyle, max_width: i32, sizes: &[Size]) -> Vec<i32> {
    let gap = style.flex_gap_px.max(0);
    if sizes.is_empty() {
        return Vec::new();
    }

    let sum_widths = sizes.iter().map(|s| s.width).sum::<i32>().max(0);
    let base_total = sum_widths.saturating_add(gap.saturating_mul((sizes.len() - 1) as i32));
    let remaining = max_width.saturating_sub(base_total).max(0);

    let spacing = match style.flex_justify_content {
        FlexJustifyContent::Start => gap,
        FlexJustifyContent::SpaceBetween => {
            if sizes.len() <= 1 {
                gap
            } else {
                gap.saturating_add(remaining / (sizes.len() - 1) as i32)
            }
        }
    };

    let mut x_positions = Vec::with_capacity(sizes.len());
    let mut cursor_x = 0i32;
    for (idx, size) in sizes.iter().enumerate() {
        x_positions.push(cursor_x);
        cursor_x = cursor_x.saturating_add(size.width);
        if idx + 1 < sizes.len() {
            cursor_x = cursor_x.saturating_add(spacing);
        }
    }
    x_positions
}

fn layout_item<'doc>(
    engine: &mut LayoutEngine<'_>,
    item: &FlexItem<'doc>,
    parent_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    x_px: i32,
    y_px: i32,
    width_px: i32,
    paint: bool,
) -> Result<(), String> {
    let width_px = width_px.max(0);
    let item_box = Rect {
        x: x_px,
        y: y_px,
        width: width_px,
        height: engine.viewport.height_px,
    };

    match item {
        FlexItem::Text(node) => {
            let _height = inline::layout_inline_nodes(
                engine,
                &[node],
                parent_style,
                ancestors,
                item_box,
                y_px,
                paint,
            )?;
        }
        FlexItem::Element(el) => {
            let child_style = engine.styles.compute_style(el, parent_style, ancestors);
            if child_style.display == Display::None {
                return Ok(());
            }
            ancestors.push(el);
            match child_style.display {
                Display::Flex => {
                    let _height = layout_flex_row(engine, el, &child_style, ancestors, item_box, paint)?;
                }
                _ => {
                    let nodes: Vec<&Node> = el.children.iter().collect();
                    let _height =
                        inline::layout_inline_nodes(engine, &nodes, &child_style, ancestors, item_box, y_px, paint)?;
                }
            }
            ancestors.pop();
        }
    }

    Ok(())
}
