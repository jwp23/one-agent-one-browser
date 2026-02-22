use crate::dom::{Element, Node};
use crate::geom::Rect;
use crate::style::{ComputedStyle, Display, Position};
use std::collections::HashMap;

use super::{LayoutEngine, flex};

#[derive(Clone, Copy, Debug)]
struct AreaPlacement {
    row_start: usize,
    row_end: usize,
    col_start: usize,
    col_end: usize,
}

struct GridItem<'doc> {
    element: &'doc Element,
    style: ComputedStyle,
    placement: Option<AreaPlacement>,
}

struct PositionedItem<'doc> {
    element: &'doc Element,
    style: ComputedStyle,
}

#[derive(Clone, Copy, Debug)]
enum Track {
    Fixed(i32),
    Fr(f32),
    Content,
}

pub(super) fn layout_grid<'doc>(
    engine: &mut LayoutEngine<'_>,
    element: &'doc Element,
    style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    content_box: Rect,
    paint: bool,
) -> Result<i32, String> {
    if style.display != Display::Grid {
        return Ok(0);
    }

    let mut template_rows =
        parse_template_areas(style.grid_template_areas.as_deref().unwrap_or(""));
    if template_rows.is_empty() {
        return engine.layout_flow_children(
            &element.children,
            style,
            ancestors,
            content_box,
            paint,
        );
    }
    normalize_template_rows(&mut template_rows);

    let areas = build_area_map(&template_rows);
    if areas.is_empty() {
        return engine.layout_flow_children(
            &element.children,
            style,
            ancestors,
            content_box,
            paint,
        );
    }

    let mut items = Vec::new();
    let mut positioned = Vec::new();
    for child in &element.children {
        match child {
            Node::Text(text) => {
                if !text.trim().is_empty() {
                    return engine.layout_flow_children(
                        &element.children,
                        style,
                        ancestors,
                        content_box,
                        paint,
                    );
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
                if matches!(child_style.position, Position::Absolute | Position::Fixed) {
                    positioned.push(PositionedItem {
                        element: el,
                        style: child_style,
                    });
                    continue;
                }

                let placement = child_style
                    .grid_area
                    .as_deref()
                    .and_then(|name| areas.get(name).copied());
                items.push(GridItem {
                    element: el,
                    style: child_style,
                    placement,
                });
            }
        }
    }

    let column_count = template_rows.iter().map(Vec::len).max().unwrap_or(1).max(1);
    let mut tracks = parse_track_list(style.grid_template_columns.as_deref().unwrap_or(""));
    if tracks.len() < column_count {
        tracks.resize(column_count, Track::Fr(1.0));
    } else if tracks.len() > column_count {
        tracks.truncate(column_count);
    }
    let gap = style.flex_gap_px.max(0);
    let column_widths = resolve_column_widths(
        engine,
        &items,
        ancestors,
        &tracks,
        column_count,
        content_box.width,
        gap,
    )?;

    let mut placed = vec![false; items.len()];
    let mut row_y = content_box.y;
    for row_index in 0..template_rows.len() {
        let mut row_height = 0i32;
        for (item_index, item) in items.iter().enumerate() {
            let Some(placement) = item.placement else {
                continue;
            };
            if placed[item_index] || placement.row_start != row_index {
                continue;
            }

            let x_offset = column_offset(&column_widths, gap, placement.col_start);
            let span_width =
                column_span_width(&column_widths, gap, placement.col_start, placement.col_end);
            if span_width <= 0 {
                placed[item_index] = true;
                continue;
            }

            let mut cursor_y = row_y;
            let containing = Rect {
                x: content_box.x.saturating_add(x_offset),
                y: row_y,
                width: span_width,
                height: content_box
                    .height
                    .saturating_sub(row_y.saturating_sub(content_box.y))
                    .max(0),
            };
            engine.layout_block_box(
                item.element,
                &item.style,
                style,
                ancestors,
                containing,
                &mut cursor_y,
                paint,
                None,
            )?;
            let row_span = placement.row_end.saturating_sub(placement.row_start);
            if row_span <= 1 {
                row_height = row_height.max(cursor_y.saturating_sub(row_y));
            }
            placed[item_index] = true;
        }
        row_y = row_y.saturating_add(row_height.max(0));
    }

    let mut cursor_y = row_y;
    for (item_index, item) in items.iter().enumerate() {
        if placed[item_index] {
            continue;
        }
        engine.layout_block_box(
            item.element,
            &item.style,
            style,
            ancestors,
            content_box,
            &mut cursor_y,
            paint,
            None,
        )?;
    }

    let containing = engine.current_positioned_containing_block();
    for item in positioned {
        engine.layout_positioned_box(item.element, &item.style, ancestors, containing, paint)?;
    }

    Ok(cursor_y.saturating_sub(content_box.y).max(0))
}

fn parse_template_areas(input: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut chars = input.char_indices().peekable();
    while let Some((start_idx, ch)) = chars.next() {
        if ch != '\'' && ch != '"' {
            continue;
        }
        let quote = ch;
        let content_start = start_idx + ch.len_utf8();
        while let Some((end_idx, current)) = chars.next() {
            if current != quote {
                continue;
            }
            let row_text = &input[content_start..end_idx];
            let row: Vec<String> = row_text
                .split_whitespace()
                .filter(|token| !token.is_empty())
                .map(|token| token.to_owned())
                .collect();
            if !row.is_empty() {
                rows.push(row);
            }
            break;
        }
    }
    rows
}

fn normalize_template_rows(rows: &mut [Vec<String>]) {
    let max_cols = rows.iter().map(Vec::len).max().unwrap_or(0);
    for row in rows {
        while row.len() < max_cols {
            row.push(".".to_owned());
        }
    }
}

fn build_area_map(rows: &[Vec<String>]) -> HashMap<String, AreaPlacement> {
    #[derive(Clone, Copy)]
    struct Bounds {
        row_min: usize,
        row_max: usize,
        col_min: usize,
        col_max: usize,
    }

    let mut bounds = HashMap::<String, Bounds>::new();
    for (row_idx, row) in rows.iter().enumerate() {
        for (col_idx, name) in row.iter().enumerate() {
            if name == "." {
                continue;
            }
            bounds
                .entry(name.clone())
                .and_modify(|entry| {
                    entry.row_min = entry.row_min.min(row_idx);
                    entry.row_max = entry.row_max.max(row_idx);
                    entry.col_min = entry.col_min.min(col_idx);
                    entry.col_max = entry.col_max.max(col_idx);
                })
                .or_insert(Bounds {
                    row_min: row_idx,
                    row_max: row_idx,
                    col_min: col_idx,
                    col_max: col_idx,
                });
        }
    }

    let mut areas = HashMap::new();
    for (name, bounds) in bounds {
        let mut rectangular = true;
        for row_idx in bounds.row_min..=bounds.row_max {
            for col_idx in bounds.col_min..=bounds.col_max {
                let same = rows
                    .get(row_idx)
                    .and_then(|row| row.get(col_idx))
                    .is_some_and(|token| token == &name);
                if !same {
                    rectangular = false;
                    break;
                }
            }
            if !rectangular {
                break;
            }
        }
        if !rectangular {
            continue;
        }

        areas.insert(
            name,
            AreaPlacement {
                row_start: bounds.row_min,
                row_end: bounds.row_max.saturating_add(1),
                col_start: bounds.col_min,
                col_end: bounds.col_max.saturating_add(1),
            },
        );
    }

    areas
}

fn parse_track_list(input: &str) -> Vec<Track> {
    split_track_tokens(input)
        .into_iter()
        .map(|token| parse_track_token(&token))
        .collect()
}

fn split_track_tokens(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut depth = 0usize;
    let mut start: Option<usize> = None;

    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => {
                depth = depth.saturating_add(1);
                if start.is_none() {
                    start = Some(idx);
                }
            }
            ')' => {
                depth = depth.saturating_sub(1);
            }
            _ if ch.is_whitespace() && depth == 0 => {
                if let Some(start_idx) = start.take() {
                    let token = input[start_idx..idx].trim();
                    if !token.is_empty() {
                        tokens.push(token.to_owned());
                    }
                }
                continue;
            }
            _ => {
                if start.is_none() {
                    start = Some(idx);
                }
            }
        }
    }

    if let Some(start_idx) = start {
        let token = input[start_idx..].trim();
        if !token.is_empty() {
            tokens.push(token.to_owned());
        }
    }

    tokens
}

fn parse_track_token(token: &str) -> Track {
    let token = token.trim();
    if token.is_empty() {
        return Track::Fr(1.0);
    }

    let lower = token.to_ascii_lowercase();
    if lower.starts_with("minmax(") && lower.ends_with(')') {
        let inner = &token[7..token.len().saturating_sub(1)];
        if let Some(second) = split_minmax_arguments(inner).get(1) {
            return parse_track_token(second);
        }
        return Track::Content;
    }

    if let Some(fr) = lower
        .strip_suffix("fr")
        .and_then(|v| v.trim().parse::<f32>().ok())
    {
        return Track::Fr(fr.max(0.0));
    }

    if let Some(px) = parse_length_px(token) {
        return Track::Fixed(px.max(0));
    }

    match lower.as_str() {
        "auto" | "min-content" | "max-content" => Track::Content,
        _ => Track::Content,
    }
}

fn split_minmax_arguments(input: &str) -> Vec<&str> {
    let mut args = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;

    for (idx, ch) in input.char_indices() {
        match ch {
            '(' => depth = depth.saturating_add(1),
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                args.push(input[start..idx].trim());
                start = idx.saturating_add(1);
            }
            _ => {}
        }
    }
    args.push(input[start..].trim());
    args
}

fn parse_length_px(input: &str) -> Option<i32> {
    let input = input.trim();
    if input.is_empty() {
        return None;
    }

    if input == "0" {
        return Some(0);
    }

    let mut end = 0usize;
    for (idx, ch) in input.char_indices() {
        if !(ch.is_ascii_digit() || ch == '.' || ch == '-') {
            break;
        }
        end = idx + ch.len_utf8();
    }
    if end == 0 {
        return None;
    }

    let number: f32 = input[..end].parse().ok()?;
    let unit = input[end..].trim().to_ascii_lowercase();
    let px = match unit.as_str() {
        "px" | "" => number,
        "pt" => number * (96.0 / 72.0),
        "rem" | "em" => number * 16.0,
        _ => return None,
    };
    Some(px.round() as i32)
}

fn resolve_column_widths<'doc>(
    engine: &LayoutEngine<'_>,
    items: &[GridItem<'doc>],
    ancestors: &mut Vec<&'doc Element>,
    tracks: &[Track],
    column_count: usize,
    container_width: i32,
    gap: i32,
) -> Result<Vec<i32>, String> {
    let mut widths = vec![0i32; column_count];
    let mut total_fr = 0.0f32;

    for (idx, track) in tracks.iter().enumerate().take(column_count) {
        match *track {
            Track::Fixed(px) => {
                widths[idx] = px.max(0);
            }
            Track::Fr(fr) => {
                total_fr += fr.max(0.0);
            }
            Track::Content => {
                let mut content_width = 0i32;
                for item in items {
                    let Some(placement) = item.placement else {
                        continue;
                    };
                    if placement.col_start != idx || placement.col_end != idx.saturating_add(1) {
                        continue;
                    }
                    let candidate = if let Some(width) = item.style.width_px {
                        width.resolve_px(container_width).max(0)
                    } else {
                        flex::measure_element_max_content_width(
                            engine,
                            item.element,
                            &item.style,
                            ancestors,
                            container_width.max(0),
                        )?
                    };
                    content_width = content_width.max(candidate);
                }
                widths[idx] = content_width.max(0);
            }
        }
    }

    let total_gap = gap
        .saturating_mul(
            column_count
                .saturating_sub(1)
                .try_into()
                .unwrap_or(i32::MAX),
        )
        .max(0);
    let fixed_sum: i32 = widths.iter().copied().fold(0i32, i32::saturating_add);
    let available = container_width.saturating_sub(total_gap).max(0);
    let remaining = available.saturating_sub(fixed_sum).max(0);

    if total_fr > 0.0 {
        let mut distributed = 0i32;
        for idx in 0..column_count {
            let Track::Fr(fr) = tracks[idx] else { continue };
            if fr <= 0.0 {
                continue;
            }
            let extra = if idx + 1 == column_count {
                remaining.saturating_sub(distributed)
            } else {
                ((remaining as f32) * (fr / total_fr)).round() as i32
            };
            widths[idx] = extra.max(0);
            distributed = distributed.saturating_add(extra.max(0));
        }
    }

    Ok(widths)
}

fn column_offset(widths: &[i32], gap: i32, col_start: usize) -> i32 {
    let mut offset = 0i32;
    for (idx, width) in widths.iter().enumerate() {
        if idx >= col_start {
            break;
        }
        offset = offset.saturating_add(*width);
        offset = offset.saturating_add(gap);
    }
    offset
}

fn column_span_width(widths: &[i32], gap: i32, col_start: usize, col_end: usize) -> i32 {
    if col_start >= col_end || col_start >= widths.len() {
        return 0;
    }
    let end = col_end.min(widths.len());
    let mut width = 0i32;
    for idx in col_start..end {
        width = width.saturating_add(widths[idx]);
        if idx + 1 < end {
            width = width.saturating_add(gap);
        }
    }
    width.max(0)
}
