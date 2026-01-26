use crate::render::{DrawLinearGradientRect, Painter};

pub(super) fn clip_rect_to_viewport(
    x_px: i32,
    y_px: i32,
    width_px: i32,
    height_px: i32,
    viewport_width_px: i32,
    viewport_height_px: i32,
) -> Option<(i32, i32, i32, i32)> {
    if width_px <= 0 || height_px <= 0 {
        return None;
    }
    if viewport_width_px <= 0 || viewport_height_px <= 0 {
        return None;
    }

    let x0 = x_px as i64;
    let y0 = y_px as i64;
    let x1 = x0.saturating_add(width_px as i64);
    let y1 = y0.saturating_add(height_px as i64);

    let cx0 = x0.max(0);
    let cy0 = y0.max(0);
    let cx1 = x1.min(viewport_width_px as i64);
    let cy1 = y1.min(viewport_height_px as i64);

    let w = cx1.saturating_sub(cx0);
    let h = cy1.saturating_sub(cy0);
    if w <= 0 || h <= 0 {
        return None;
    }

    let x: i32 = cx0.try_into().ok()?;
    let y: i32 = cy0.try_into().ok()?;
    let w: i32 = w.try_into().ok()?;
    let h: i32 = h.try_into().ok()?;
    Some((x, y, w, h))
}

pub(super) fn fill_linear_gradient_rect_clipped(
    painter: &mut dyn Painter,
    rect: &DrawLinearGradientRect,
    clip_x_px: i32,
    clip_y_px: i32,
    clip_width_px: i32,
    clip_height_px: i32,
) -> Result<(), String> {
    if clip_width_px <= 0 || clip_height_px <= 0 {
        return Ok(());
    }

    let (start, end) = match rect.direction {
        crate::style::GradientDirection::TopToBottom => (rect.start_color, rect.end_color),
        crate::style::GradientDirection::BottomToTop => (rect.end_color, rect.start_color),
        crate::style::GradientDirection::LeftToRight => (rect.start_color, rect.end_color),
        crate::style::GradientDirection::RightToLeft => (rect.end_color, rect.start_color),
    };

    let den = match rect.direction {
        crate::style::GradientDirection::TopToBottom | crate::style::GradientDirection::BottomToTop => {
            rect.height_px.saturating_sub(1)
        }
        crate::style::GradientDirection::LeftToRight | crate::style::GradientDirection::RightToLeft => {
            rect.width_px.saturating_sub(1)
        }
    };
    if den <= 0 {
        painter.fill_rect(clip_x_px, clip_y_px, clip_width_px, clip_height_px, start)?;
        return Ok(());
    }

    fn lerp_channel(start: u8, end: u8, num: i32, den: i32) -> u8 {
        let start = start as i32;
        let end = end as i32;
        let num = num.clamp(0, den);
        ((start * (den - num) + end * num + den / 2) / den)
            .clamp(0, 255) as u8
    }

    match rect.direction {
        crate::style::GradientDirection::TopToBottom | crate::style::GradientDirection::BottomToTop => {
            let start_y_in_rect = clip_y_px.saturating_sub(rect.y_px);
            for y in 0..clip_height_px {
                let y_in_rect = start_y_in_rect.saturating_add(y);
                let color = crate::geom::Color {
                    r: lerp_channel(start.r, end.r, y_in_rect, den),
                    g: lerp_channel(start.g, end.g, y_in_rect, den),
                    b: lerp_channel(start.b, end.b, y_in_rect, den),
                    a: lerp_channel(start.a, end.a, y_in_rect, den),
                };
                painter.fill_rect(clip_x_px, clip_y_px.saturating_add(y), clip_width_px, 1, color)?;
            }
        }
        crate::style::GradientDirection::LeftToRight | crate::style::GradientDirection::RightToLeft => {
            let start_x_in_rect = clip_x_px.saturating_sub(rect.x_px);
            for x in 0..clip_width_px {
                let x_in_rect = start_x_in_rect.saturating_add(x);
                let color = crate::geom::Color {
                    r: lerp_channel(start.r, end.r, x_in_rect, den),
                    g: lerp_channel(start.g, end.g, x_in_rect, den),
                    b: lerp_channel(start.b, end.b, x_in_rect, den),
                    a: lerp_channel(start.a, end.a, x_in_rect, den),
                };
                painter.fill_rect(clip_x_px.saturating_add(x), clip_y_px, 1, clip_height_px, color)?;
            }
        }
    }

    Ok(())
}
