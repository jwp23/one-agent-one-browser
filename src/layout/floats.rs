use crate::dom::Element;
use crate::geom::Rect;
use crate::style::{ComputedStyle, Float};

use super::{flex, inline, LayoutEngine};

#[derive(Clone, Copy, Debug)]
pub(super) struct FloatPlacement {
    pub(super) side: Float,
    pub(super) rect: Rect,
}

#[derive(Clone, Copy, Debug, Default)]
struct FloatClearance {
    left_offset_px: i32,
    right_offset_px: i32,
    next_y: Option<i32>,
}

pub(super) fn flow_area_at_y(
    floats: &[FloatPlacement],
    containing: Rect,
    start_y: i32,
) -> (Rect, i32) {
    let mut y = start_y;
    loop {
        let clearance = clearance_at_y(floats, containing, y);
        let available_width = containing
            .width
            .saturating_sub(clearance.left_offset_px.saturating_add(clearance.right_offset_px))
            .max(0);

        if available_width > 0 || clearance.next_y.is_none() {
            return (
                Rect {
                    x: containing.x.saturating_add(clearance.left_offset_px),
                    y: containing.y,
                    width: available_width,
                    height: containing.height,
                },
                y,
            );
        }

        let Some(next_y) = clearance.next_y else {
            return (containing, y);
        };
        if next_y <= y {
            return (containing, y);
        }
        y = next_y;
    }
}

pub(super) fn flow_area_at_exact_y(floats: &[FloatPlacement], containing: Rect, y: i32) -> Rect {
    let clearance = clearance_at_y(floats, containing, y);
    let available_width = containing
        .width
        .saturating_sub(clearance.left_offset_px.saturating_add(clearance.right_offset_px))
        .max(0);

    Rect {
        x: containing.x.saturating_add(clearance.left_offset_px),
        y: containing.y,
        width: available_width,
        height: containing.height,
    }
}

pub(super) fn flow_area_for_width(
    floats: &[FloatPlacement],
    containing: Rect,
    start_y: i32,
    required_outer_width: i32,
) -> (Rect, i32) {
    let required_outer_width = required_outer_width.max(1);

    let mut y = start_y;
    loop {
        let clearance = clearance_at_y(floats, containing, y);
        let available_width = containing
            .width
            .saturating_sub(clearance.left_offset_px.saturating_add(clearance.right_offset_px))
            .max(0);
        if available_width >= required_outer_width {
            return (
                Rect {
                    x: containing.x.saturating_add(clearance.left_offset_px),
                    y: containing.y,
                    width: available_width,
                    height: containing.height,
                },
                y,
            );
        }

        let Some(next_y) = clearance.next_y else {
            return (containing, y);
        };
        if next_y <= y {
            return (containing, y);
        }
        y = next_y;
    }
}

pub(super) fn layout_float<'doc>(
    engine: &mut LayoutEngine<'_>,
    element: &'doc Element,
    style: &ComputedStyle,
    parent_style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    containing: Rect,
    cursor_y: i32,
    floats: &[FloatPlacement],
    paint: bool,
) -> Result<FloatPlacement, String> {
    let side = style.float;
    if !matches!(side, Float::Left | Float::Right) {
        return Err(format!("layout_float called with non-floating style: {side:?}"));
    }

    let margin_left_px = if style.margin_auto.left {
        0
    } else {
        style.margin.left
    };
    let margin_right_px = if style.margin_auto.right {
        0
    } else {
        style.margin.right
    };

    let mut y_outer = cursor_y;
    loop {
        let clearance = clearance_at_y(floats, containing, y_outer);
        let available_width = containing
            .width
            .saturating_sub(clearance.left_offset_px.saturating_add(clearance.right_offset_px))
            .max(0);

        let border_width = measure_float_border_width(engine, element, style, ancestors, available_width)?;
        let outer_width = margin_left_px
            .saturating_add(border_width)
            .saturating_add(margin_right_px);

        if clearance.next_y.is_none() || outer_width <= available_width {
            let x_outer = match side {
                Float::Left => containing.x.saturating_add(clearance.left_offset_px),
                Float::Right => containing
                    .right()
                    .saturating_sub(clearance.right_offset_px)
                    .saturating_sub(outer_width),
                Float::None => containing.x,
            };

            let mut float_cursor_y = y_outer;
            engine.layout_block_box(
                element,
                style,
                parent_style,
                ancestors,
                Rect {
                    x: x_outer,
                    y: y_outer,
                    width: outer_width,
                    height: containing.height,
                },
                &mut float_cursor_y,
                paint,
                None,
            )?;

            let outer_height = float_cursor_y.saturating_sub(y_outer).max(0);
            return Ok(FloatPlacement {
                side,
                rect: Rect {
                    x: x_outer,
                    y: y_outer,
                    width: outer_width,
                    height: outer_height,
                },
            });
        }

        let Some(next_y) = clearance.next_y else { break };
        if next_y <= y_outer {
            break;
        }
        y_outer = next_y;
    }

    let border_width = measure_float_border_width(engine, element, style, ancestors, containing.width.max(0))?;
    let outer_width = margin_left_px
        .saturating_add(border_width)
        .saturating_add(margin_right_px);
    let x_outer = match side {
        Float::Right => containing.right().saturating_sub(outer_width),
        _ => containing.x,
    };

    let mut float_cursor_y = y_outer;
    engine.layout_block_box(
        element,
        style,
        parent_style,
        ancestors,
        Rect {
            x: x_outer,
            y: y_outer,
            width: outer_width,
            height: containing.height,
        },
        &mut float_cursor_y,
        paint,
        None,
    )?;
    let outer_height = float_cursor_y.saturating_sub(y_outer).max(0);
    Ok(FloatPlacement {
        side,
        rect: Rect {
            x: x_outer,
            y: y_outer,
            width: outer_width,
            height: outer_height,
        },
    })
}

fn measure_float_border_width<'doc>(
    engine: &LayoutEngine<'_>,
    element: &'doc Element,
    style: &ComputedStyle,
    ancestors: &mut Vec<&'doc Element>,
    max_width: i32,
) -> Result<i32, String> {
    let max_width = max_width.max(0);

    let mut border_width = if inline::is_replaced_element(element) {
        let size = inline::measure_replaced_element_outer_size(element, style, max_width)?;
        size.width
            .saturating_sub(style.margin.left.saturating_add(style.margin.right))
            .max(0)
    } else if let Some(width) = style.width_px {
        width.resolve_px(max_width).max(0)
    } else {
        flex::measure_element_max_content_width(engine, element, style, ancestors, max_width)?
    };

    if let Some(min_width) = style.min_width_px {
        border_width = border_width.max(min_width.resolve_px(max_width).max(0));
    }
    if let Some(max_width_value) = style.max_width_px {
        border_width = border_width.min(max_width_value.resolve_px(max_width).max(0));
    }

    Ok(border_width.max(0))
}

fn clearance_at_y(floats: &[FloatPlacement], containing: Rect, y: i32) -> FloatClearance {
    let mut clearance = FloatClearance::default();

    for float in floats {
        if !overlaps_y(float.rect, y) {
            continue;
        }

        clearance.next_y = match clearance.next_y {
            Some(existing) => Some(existing.min(float.rect.bottom())),
            None => Some(float.rect.bottom()),
        };

        match float.side {
            Float::Left => {
                clearance.left_offset_px = clearance
                    .left_offset_px
                    .max(float.rect.right().saturating_sub(containing.x));
            }
            Float::Right => {
                clearance.right_offset_px = clearance
                    .right_offset_px
                    .max(containing.right().saturating_sub(float.rect.x));
            }
            Float::None => {}
        }
    }

    clearance
}

fn overlaps_y(rect: Rect, y: i32) -> bool {
    if rect.height <= 0 {
        return false;
    }
    y >= rect.y && y < rect.bottom()
}
