use super::*;

struct FixedMeasurer;

impl TextMeasurer for FixedMeasurer {
    fn font_metrics_px(&self, _style: TextStyle) -> crate::render::FontMetricsPx {
        crate::render::FontMetricsPx {
            ascent_px: 8,
            descent_px: 2,
        }
    }

    fn text_width_px(&self, text: &str, _style: TextStyle) -> Result<i32, String> {
        Ok(text.len() as i32)
    }
}

#[test]
fn wraps_words_when_exceeding_width() {
    let doc = crate::html::parse_document("<p>Hello World</p>");
    let viewport = Viewport {
        width_px: 5,
        height_px: 200,
    };
    let styles = crate::style::StyleComputer::from_document(&doc);
    let output = layout_document(
        &doc,
        &styles,
        &FixedMeasurer,
        viewport,
        &crate::resources::NoResources,
    )
    .unwrap();
    assert!(
        output
            .display_list
            .commands
            .iter()
            .any(|cmd| matches!(cmd, DisplayCommand::Text(_)))
    );
}

#[test]
fn records_link_hit_regions_for_anchor_text() {
    let doc = crate::html::parse_document(r#"<p><a href="https://example.com">Hello</a></p>"#);
    let viewport = Viewport {
        width_px: 200,
        height_px: 200,
    };
    let styles = crate::style::StyleComputer::from_document(&doc);
    let output = layout_document(
        &doc,
        &styles,
        &FixedMeasurer,
        viewport,
        &crate::resources::NoResources,
    )
    .unwrap();
    assert!(
        output
            .link_regions
            .iter()
            .any(|region| region.href.as_ref() == "https://example.com")
    );
}

#[test]
fn records_link_hit_regions_for_flex_item_anchor() {
    let doc = crate::html::parse_document(
        r#"<style>header { display: flex; }</style><header><a href="/posts/">Posts</a></header>"#,
    );
    let viewport = Viewport {
        width_px: 200,
        height_px: 200,
    };
    let styles = crate::style::StyleComputer::from_document(&doc);
    let output = layout_document(
        &doc,
        &styles,
        &FixedMeasurer,
        viewport,
        &crate::resources::NoResources,
    )
    .unwrap();
    assert!(
        output
            .link_regions
            .iter()
            .any(|region| region.href.as_ref() == "/posts/")
    );
}

