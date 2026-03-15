use crate::dom::{Element, Node};
use crate::geom::{Edges, Rect, Size};
use crate::render::{DisplayCommand, TextStyle};
use crate::style::{ComputedStyle, Display, TextAlign, Visibility};

use super::LayoutEngine;

pub(super) fn measure_auto_table_width<'doc>(
    engine: &LayoutEngine<'_>,
    table: &'doc Element,
    table_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    available_width: i32,
) -> Result<i32, String> {
    let cellspacing = table
        .attributes
        .get("cellspacing")
        .and_then(parse_i32)
        .unwrap_or(0)
        .max(0);
    let (col_widths, _) =
        compute_intrinsic_column_widths(engine, table, table_style, ancestors, cellspacing)?;
    let caption_width = measure_caption_min_width(engine, table, table_style, ancestors)?;
    Ok(sum_table_width(&col_widths, cellspacing)
        .max(caption_width)
        .min(available_width.max(0)))
}

pub(super) fn layout_table<'doc>(
    engine: &mut LayoutEngine<'_>,
    table: &'doc Element,
    table_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    content_box: Rect,
    paint: bool,
) -> Result<Size, String> {
    let cellpadding = table
        .attributes
        .get("cellpadding")
        .and_then(parse_i32)
        .unwrap_or(0)
        .max(0);
    let cellspacing = table
        .attributes
        .get("cellspacing")
        .and_then(parse_i32)
        .unwrap_or(0)
        .max(0);

    let rows = collect_table_rows(table);

    let grid = build_grid(rows);
    let (mut col_widths, fixed) =
        compute_intrinsic_column_widths(engine, table, table_style, ancestors, cellspacing)?;

    let total_min = sum_table_width(&col_widths, cellspacing);
    let extra = content_box.width.saturating_sub(total_min).max(0);
    if extra > 0 {
        if let Some(idx) = best_extra_column(&col_widths, &fixed) {
            col_widths[idx] = col_widths[idx].saturating_add(extra);
        }
    }

    let mut y = content_box.y;
    if let Some(caption) = collect_table_caption(table) {
        let mut caption_style = engine.styles.compute_style_in_viewport(
            caption,
            table_style,
            ancestors,
            engine.viewport.width_px,
            engine.viewport.height_px,
        );
        if caption_style.display == Display::Inline {
            caption_style.display = Display::Block;
        }
        if caption_style.text_align == TextAlign::Left {
            caption_style.text_align = TextAlign::Center;
        }
        engine.layout_block_box(
            caption,
            &caption_style,
            table_style,
            ancestors,
            Rect {
                x: content_box.x,
                y,
                width: content_box.width,
                height: content_box.height,
            },
            &mut y,
            paint,
            None,
        )?;
    }

    for row in &grid.rows {
        let row_style = engine.styles.compute_style_in_viewport(
            row.element,
            table_style,
            ancestors,
            engine.viewport.width_px,
            engine.viewport.height_px,
        );
        if row_style.display == Display::None {
            continue;
        }
        let row_paint = paint && row_style.visibility == Visibility::Visible;

        let mut row_height = row_style.height_px.unwrap_or(0).max(0);

        ancestors.push(row.element);
        let mut x = content_box.x;
        for cell in &row.cells {
            let cell_style = engine.styles.compute_style_in_viewport(
                cell.element,
                &row_style,
                ancestors,
                engine.viewport.width_px,
                engine.viewport.height_px,
            );
            if cell_style.display == Display::None {
                continue;
            }
            let mut cell_paint = row_paint && cell_style.visibility == Visibility::Visible;
            if cell_paint && cell_style.opacity == 0 {
                cell_paint = false;
            }
            let opacity = cell_style.opacity;
            let needs_opacity_group = cell_paint && opacity < 255;
            if needs_opacity_group {
                engine
                    .list
                    .commands
                    .push(DisplayCommand::PushOpacity(opacity));
            }

            let span_width =
                cell_span_width(&col_widths, cell.col_index, cell.colspan, cellspacing);

            let cell_padding = Edges {
                top: cellpadding,
                right: cellpadding,
                bottom: cellpadding,
                left: cellpadding,
            };
            let padding = add_edges(cell_padding, cell_style.padding.resolve_px(span_width));

            let border_box = Rect {
                x,
                y,
                width: span_width,
                height: 0,
            };
            let content = border_box.inset(padding);

            let background_index = if cell_paint {
                engine.push_background(border_box, &cell_style, 0)
            } else {
                None
            };

            ancestors.push(cell.element);
            let content_height = engine.layout_flow_children(
                &cell.element.children,
                &cell_style,
                ancestors,
                content,
                cell_paint,
            )?;
            ancestors.pop();
            let mut cell_height = padding
                .top
                .saturating_add(content_height)
                .saturating_add(padding.bottom);
            if let Some(min_height) = cell_style.height_px {
                cell_height = cell_height.max(min_height);
            }

            if let Some(index) = background_index {
                engine.set_background_height(index, cell_height);
            }

            if needs_opacity_group {
                engine
                    .list
                    .commands
                    .push(DisplayCommand::PopOpacity(opacity));
            }

            row_height = row_height.max(cell_height);
            x = x.saturating_add(span_width).saturating_add(cellspacing);
        }
        ancestors.pop();

        y = y.saturating_add(row_height).saturating_add(cellspacing);
    }

    Ok(Size {
        width: content_box.width,
        height: y.saturating_sub(content_box.y).max(0),
    })
}

fn compute_intrinsic_column_widths<'doc>(
    engine: &LayoutEngine<'_>,
    table: &'doc Element,
    table_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    cellspacing: i32,
) -> Result<(Vec<i32>, Vec<bool>), String> {
    let cellpadding = table
        .attributes
        .get("cellpadding")
        .and_then(parse_i32)
        .unwrap_or(0)
        .max(0);
    let rows = collect_table_rows(table);
    let grid = build_grid(rows);
    let mut col_widths = vec![0i32; grid.columns];
    let mut fixed = vec![false; grid.columns];

    for row in &grid.rows {
        for cell in &row.cells {
            let cell_style = engine.styles.compute_style_in_viewport(
                cell.element,
                table_style,
                ancestors,
                engine.viewport.width_px,
                engine.viewport.height_px,
            );
            let min_width =
                measure_cell_min_width(engine, cell.element, &cell_style, ancestors, cellpadding)?;
            let target_width = cell_style
                .width_px
                .map(|width| width.resolve_px(0))
                .unwrap_or(min_width);

            apply_cell_target_width(
                &mut col_widths,
                &mut fixed,
                cell,
                target_width,
                cellspacing,
                cell_style.width_px.is_some() || cell_style.text_align == TextAlign::Right,
            );
        }
    }

    Ok((col_widths, fixed))
}

fn apply_cell_target_width(
    col_widths: &mut [i32],
    fixed: &mut [bool],
    cell: &GridCell<'_>,
    target_width: i32,
    cellspacing: i32,
    mark_fixed: bool,
) {
    if cell.colspan == 1 {
        if let Some(width) = col_widths.get_mut(cell.col_index) {
            *width = (*width).max(target_width);
        }
        if mark_fixed {
            if let Some(slot) = fixed.get_mut(cell.col_index) {
                *slot = true;
            }
        }
        return;
    }

    let current_width = cell_span_width(col_widths, cell.col_index, cell.colspan, cellspacing);
    let deficit = target_width.saturating_sub(current_width);
    if deficit > 0 {
        distribute_span_extra(col_widths, cell.col_index, cell.colspan, deficit);
    }
    if mark_fixed {
        for idx in cell.col_index..cell.col_index.saturating_add(cell.colspan) {
            if let Some(slot) = fixed.get_mut(idx) {
                *slot = true;
            }
        }
    }
}

fn distribute_span_extra(col_widths: &mut [i32], start: usize, span: usize, extra: i32) {
    let end = start.saturating_add(span).min(col_widths.len());
    let count = end.saturating_sub(start);
    if count == 0 || extra <= 0 {
        return;
    }

    let base = extra / count as i32;
    let remainder = extra % count as i32;
    for (offset, width) in col_widths[start..end].iter_mut().enumerate() {
        let bump = base + i32::from(offset < remainder as usize);
        *width = width.saturating_add(bump);
    }
}

struct GridRow<'doc> {
    element: &'doc Element,
    cells: Vec<GridCell<'doc>>,
}

struct GridCell<'doc> {
    element: &'doc Element,
    col_index: usize,
    colspan: usize,
}

struct Grid<'doc> {
    columns: usize,
    rows: Vec<GridRow<'doc>>,
}

fn build_grid<'doc>(rows: Vec<&'doc Element>) -> Grid<'doc> {
    let mut grid_rows = Vec::new();
    let mut columns = 0usize;

    for row in rows {
        let mut col_index = 0usize;
        let mut cells = Vec::new();
        for child in &row.children {
            let Node::Element(el) = child else {
                continue;
            };
            if el.name != "td" && el.name != "th" {
                continue;
            }
            let colspan = el
                .attributes
                .get("colspan")
                .and_then(parse_usize)
                .unwrap_or(1)
                .max(1);
            cells.push(GridCell {
                element: el,
                col_index,
                colspan,
            });
            col_index = col_index.saturating_add(colspan);
        }
        columns = columns.max(col_index);
        grid_rows.push(GridRow {
            element: row,
            cells,
        });
    }

    Grid {
        columns: columns.max(1),
        rows: grid_rows,
    }
}

fn collect_table_rows<'doc>(table: &'doc Element) -> Vec<&'doc Element> {
    let mut rows = Vec::new();
    for child in &table.children {
        let Node::Element(el) = child else {
            continue;
        };
        if el.name == "tr" {
            rows.push(el);
            continue;
        }
        if is_table_row_group(el.name.as_str()) {
            for grandchild in &el.children {
                let Node::Element(row) = grandchild else {
                    continue;
                };
                if row.name == "tr" {
                    rows.push(row);
                }
            }
        }
    }
    rows
}

fn collect_table_caption(table: &Element) -> Option<&Element> {
    table.children.iter().find_map(|child| {
        let Node::Element(el) = child else {
            return None;
        };
        (el.name == "caption").then_some(el)
    })
}

fn is_table_row_group(name: &str) -> bool {
    matches!(name, "tbody" | "thead" | "tfoot")
}

fn sum_table_width(col_widths: &[i32], cellspacing: i32) -> i32 {
    let mut total = 0i32;
    for (idx, width) in col_widths.iter().enumerate() {
        total = total.saturating_add(*width);
        if idx + 1 < col_widths.len() {
            total = total.saturating_add(cellspacing);
        }
    }
    total
}

fn cell_span_width(col_widths: &[i32], start: usize, span: usize, cellspacing: i32) -> i32 {
    let mut width = 0i32;
    for idx in 0..span {
        let col = start + idx;
        if col >= col_widths.len() {
            break;
        }
        width = width.saturating_add(col_widths[col]);
        if idx + 1 < span {
            width = width.saturating_add(cellspacing);
        }
    }
    width
}

fn best_extra_column(col_widths: &[i32], fixed: &[bool]) -> Option<usize> {
    let mut best: Option<(usize, i32)> = None;
    for (idx, width) in col_widths.iter().enumerate() {
        if fixed.get(idx).copied().unwrap_or(true) {
            continue;
        }
        best = Some(match best {
            Some((best_idx, best_width)) => {
                if *width > best_width {
                    (idx, *width)
                } else {
                    (best_idx, best_width)
                }
            }
            None => (idx, *width),
        });
    }
    best.map(|(idx, _)| idx)
}

fn add_edges(a: Edges, b: Edges) -> Edges {
    Edges {
        top: a.top.saturating_add(b.top),
        right: a.right.saturating_add(b.right),
        bottom: a.bottom.saturating_add(b.bottom),
        left: a.left.saturating_add(b.left),
    }
}

fn parse_i32(value: &str) -> Option<i32> {
    value.trim().parse().ok()
}

fn parse_usize(value: &str) -> Option<usize> {
    value.trim().parse().ok()
}

fn measure_caption_min_width<'doc>(
    engine: &LayoutEngine<'_>,
    table: &'doc Element,
    table_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
) -> Result<i32, String> {
    let Some(caption) = collect_table_caption(table) else {
        return Ok(0);
    };
    let style = engine.styles.compute_style_in_viewport(
        caption,
        table_style,
        ancestors,
        engine.viewport.width_px,
        engine.viewport.height_px,
    );
    let mut width = 0i32;
    ancestors.push(caption);
    measure_inline_words(
        engine,
        &caption.children,
        &style,
        ancestors,
        &mut width,
        engine.text_style_for(&style),
    )?;
    ancestors.pop();

    let padding = style.padding.resolve_px(0);
    Ok(width
        .saturating_add(padding.left)
        .saturating_add(padding.right))
}

fn measure_cell_min_width<'doc>(
    engine: &LayoutEngine<'_>,
    cell: &'doc Element,
    cell_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    cellpadding: i32,
) -> Result<i32, String> {
    let mut max_width = 0i32;
    let text_style = engine.text_style_for(cell_style);

    ancestors.push(cell);
    measure_inline_words(
        engine,
        &cell.children,
        cell_style,
        ancestors,
        &mut max_width,
        text_style,
    )?;
    ancestors.pop();

    let padding = cell_style.padding.resolve_px(0);
    let padding = padding.left.saturating_add(padding.right);
    Ok(max_width
        .saturating_add(cellpadding.saturating_mul(2))
        .saturating_add(padding))
}

fn measure_inline_words<'doc>(
    engine: &LayoutEngine<'_>,
    nodes: &'doc [Node],
    style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    out: &mut i32,
    text_style: TextStyle,
) -> Result<(), String> {
    for node in nodes {
        match node {
            Node::Text(text) => {
                for word in text.split_whitespace() {
                    if word.is_empty() {
                        continue;
                    }
                    let width = engine.measurer.text_width_px(word, text_style)?;
                    *out = (*out).max(width);
                }
            }
            Node::Element(el) => {
                let child_style = engine.styles.compute_style_in_viewport(
                    el,
                    style,
                    ancestors,
                    engine.viewport.width_px,
                    engine.viewport.height_px,
                );
                if child_style.display == Display::None {
                    continue;
                }

                if let Some(width) = child_style.width_px {
                    let padding = child_style.padding.resolve_px(0);
                    let total = width
                        .resolve_px(0)
                        .saturating_add(child_style.margin.left)
                        .saturating_add(child_style.margin.right)
                        .saturating_add(padding.left)
                        .saturating_add(padding.right);
                    *out = (*out).max(total);
                }

                ancestors.push(el);
                let child_text_style = engine.text_style_for(&child_style);
                measure_inline_words(
                    engine,
                    &el.children,
                    &child_style,
                    ancestors,
                    out,
                    child_text_style,
                )?;
                ancestors.pop();
            }
        }
    }
    Ok(())
}

// Keep table layout logic local; no extra helpers needed yet.
