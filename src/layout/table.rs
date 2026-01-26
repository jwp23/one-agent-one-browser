use crate::dom::{Element, Node};
use crate::geom::{Edges, Rect, Size};
use crate::render::{DisplayCommand, TextStyle};
use crate::style::{ComputedStyle, Display, Visibility};

use super::LayoutEngine;

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

    let mut rows: Vec<&'doc Element> = Vec::new();
    for child in &table.children {
        if let Node::Element(el) = child {
            if el.name == "tr" {
                rows.push(el);
            }
        }
    }

    let grid = build_grid(rows);
    let mut col_widths = vec![0i32; grid.columns];
    let mut fixed = vec![false; grid.columns];

    for row in &grid.rows {
        for cell in &row.cells {
            if cell.colspan != 1 {
                continue;
            }
            let cell_style = engine.styles.compute_style_in_viewport(
                cell.element,
                table_style,
                ancestors,
                engine.viewport.width_px,
                engine.viewport.height_px,
            );
            let min_width =
                measure_cell_min_width(engine, cell.element, &cell_style, ancestors, cellpadding)?;

            if let Some(width) = cell_style.width_px.map(|width| width.resolve_px(content_box.width)) {
                col_widths[cell.col_index] = col_widths[cell.col_index].max(width);
                fixed[cell.col_index] = true;
            } else {
                col_widths[cell.col_index] = col_widths[cell.col_index].max(min_width);
                if cell_style.text_align == crate::style::TextAlign::Right {
                    fixed[cell.col_index] = true;
                }
            }
        }
    }

    let total_min = sum_table_width(&col_widths, cellspacing);
    let extra = content_box.width.saturating_sub(total_min).max(0);
    if extra > 0 {
        if let Some(idx) = best_extra_column(&col_widths, &fixed) {
            col_widths[idx] = col_widths[idx].saturating_add(extra);
        }
    }

    let mut y = content_box.y;
    for row in &grid.rows {
        if y >= engine.viewport.height_px {
            break;
        }
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
                engine.list.commands.push(DisplayCommand::PushOpacity(opacity));
            }

            let span_width = cell_span_width(&col_widths, cell.col_index, cell.colspan, cellspacing);

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
            let mut cell_height =
                padding.top.saturating_add(content_height).saturating_add(padding.bottom);
            if let Some(min_height) = cell_style.height_px {
                cell_height = cell_height.max(min_height);
            }

            if let Some(index) = background_index {
                engine.set_background_height(index, cell_height);
            }

            if needs_opacity_group {
                engine.list.commands.push(DisplayCommand::PopOpacity(opacity));
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
            if el.name != "td" {
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
        grid_rows.push(GridRow { element: row, cells });
    }

    Grid {
        columns: columns.max(1),
        rows: grid_rows,
    }
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
