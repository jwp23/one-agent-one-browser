use super::parse::parse_css_length_px_with_viewport;

#[derive(Clone, Copy, Debug)]
pub enum CssLength {
    Px(i32),
    Percent(f32),
}

impl CssLength {
    pub fn resolve_px(self, reference_px: i32) -> i32 {
        let reference_px = reference_px.max(0);
        match self {
            CssLength::Px(px) => px,
            CssLength::Percent(percent) => ((reference_px as f32) * (percent / 100.0)).round() as i32,
        }
    }
}

pub(super) fn parse_css_length(
    value: &str,
    viewport_width_px: Option<i32>,
    viewport_height_px: Option<i32>,
) -> Option<CssLength> {
    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    if let Some(percent) = value.strip_suffix('%') {
        let percent = percent.trim();
        if percent.is_empty() {
            return None;
        }
        let number: f32 = percent.parse().ok()?;
        return Some(CssLength::Percent(number));
    }

    parse_css_length_px_with_viewport(value, viewport_width_px, viewport_height_px)
        .map(CssLength::Px)
}

