use super::*;
use crate::resources::ResourceLoader;
use std::sync::Arc;

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

struct SvgOnlyResources;

impl ResourceLoader for SvgOnlyResources {
    fn load_bytes(&self, reference: &str) -> Result<Option<Arc<Vec<u8>>>, String> {
        if !reference.ends_with(".svg") {
            return Ok(None);
        }
        Ok(Some(Arc::new(
            br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10"><rect width="10" height="10" fill="red"/></svg>"#
                .to_vec(),
        )))
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
fn nowrap_keeps_words_on_single_line() {
    let doc = crate::html::parse_document(
        r#"
            <style>
                body { margin: 0; }
                .line { width: 8px; white-space: nowrap; }
            </style>
            <div class="line">hello world</div>
        "#,
    );
    let viewport = Viewport {
        width_px: 40,
        height_px: 80,
    };
    let styles = crate::style::StyleComputer::from_document(&doc);
    let output = layout_document(
        &doc,
        &styles,
        &FixedMeasurer,
        viewport,
        &crate::resources::NoResources,
    )
    .expect("layout should succeed");

    let mut hello_y = None;
    let mut world_y = None;
    for command in &output.display_list.commands {
        let DisplayCommand::Text(text) = command else {
            continue;
        };
        if text.text == "hello" {
            hello_y = Some(text.y_px);
        } else if text.text == "world" {
            world_y = Some(text.y_px);
        }
    }

    let hello_y = hello_y.expect("first word should render");
    let world_y = world_y.expect("second word should render");
    assert_eq!(hello_y, world_y, "nowrap text should not wrap to a new line");
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

#[test]
fn flex_row_shrinks_items_to_fit_container_width() {
    let doc = crate::html::parse_document(
        r#"
            <style>
                body { margin: 0; }
                .row { display: flex; width: 100px; }
                .a { width: 80px; height: 10px; background: #ff0000; flex-shrink: 1; }
                .b { width: 80px; height: 10px; background: #00ff00; flex-shrink: 1; }
            </style>
            <div class="row">
                <div class="a"></div>
                <div class="b"></div>
            </div>
        "#,
    );
    let viewport = Viewport {
        width_px: 160,
        height_px: 80,
    };
    let styles = crate::style::StyleComputer::from_document(&doc);
    let output = layout_document(
        &doc,
        &styles,
        &FixedMeasurer,
        viewport,
        &crate::resources::NoResources,
    )
    .expect("layout should succeed");

    let mut red = None;
    let mut green = None;
    for command in &output.display_list.commands {
        let DisplayCommand::Rect(rect) = command else {
            continue;
        };
        if rect.color.r == 255 && rect.color.g == 0 && rect.color.b == 0 {
            red = Some(rect.clone());
        } else if rect.color.r == 0 && rect.color.g == 255 && rect.color.b == 0 {
            green = Some(rect.clone());
        }
    }

    let red = red.expect("red flex item should render");
    let green = green.expect("green flex item should render");
    assert!(
        green.x_px.saturating_add(green.width_px) <= 100,
        "flex items should shrink to remain inside the container"
    );
    assert!(
        red.width_px < 80 && green.width_px < 80,
        "both flex items should receive shrinkage"
    );
}

#[test]
fn grid_containers_fallback_to_block_flow() {
    let doc = crate::html::parse_document(
        r#"
            <style>
                .layout { display: grid; }
                .cell { width: 40px; height: 20px; }
                .a { background: #ff0000; }
                .b { background: #00ff00; }
            </style>
            <div class="layout">
                <div class="cell a"></div>
                <div class="cell b"></div>
            </div>
        "#,
    );
    let viewport = Viewport {
        width_px: 200,
        height_px: 120,
    };
    let styles = crate::style::StyleComputer::from_document(&doc);
    let output = layout_document(
        &doc,
        &styles,
        &FixedMeasurer,
        viewport,
        &crate::resources::NoResources,
    )
    .expect("layout should succeed");

    let mut red = None;
    let mut green = None;
    for command in &output.display_list.commands {
        let DisplayCommand::Rect(rect) = command else {
            continue;
        };
        if rect.color.r == 255 && rect.color.g == 0 && rect.color.b == 0 {
            red = Some(rect.clone());
        } else if rect.color.r == 0 && rect.color.g == 255 && rect.color.b == 0 {
            green = Some(rect.clone());
        }
    }

    let red = red.expect("red cell should be painted");
    let green = green.expect("green cell should be painted");
    assert_eq!(green.x_px, red.x_px);
    assert!(
        green.y_px > red.y_px,
        "grid fallback should stack cells vertically"
    );
}

#[test]
fn grid_template_places_named_areas_into_columns() {
    let doc = crate::html::parse_document(
        r#"
            <style>
                .layout {
                    display: grid;
                    column-gap: 10px;
                    grid-template-columns: 40px 1fr;
                    grid-template-areas: 'left right';
                }
                .left { grid-area: left; width: 40px; height: 20px; background: #ff0000; }
                .right { grid-area: right; height: 20px; background: #00ff00; }
            </style>
            <div class="layout">
                <div class="left"></div>
                <div class="right"></div>
            </div>
        "#,
    );
    let viewport = Viewport {
        width_px: 200,
        height_px: 120,
    };
    let styles = crate::style::StyleComputer::from_document(&doc);
    let output = layout_document(
        &doc,
        &styles,
        &FixedMeasurer,
        viewport,
        &crate::resources::NoResources,
    )
    .expect("layout should succeed");

    let mut red = None;
    let mut green = None;
    for command in &output.display_list.commands {
        let DisplayCommand::Rect(rect) = command else {
            continue;
        };
        if rect.color.r == 255 && rect.color.g == 0 && rect.color.b == 0 {
            red = Some(rect.clone());
        } else if rect.color.r == 0 && rect.color.g == 255 && rect.color.b == 0 {
            green = Some(rect.clone());
        }
    }

    let red = red.expect("red cell should be painted");
    let green = green.expect("green cell should be painted");
    assert!(
        green.x_px > red.x_px,
        "right area should be placed to the right"
    );
    assert_eq!(green.y_px, red.y_px);
}

#[test]
fn spanning_grid_area_does_not_force_first_row_height() {
    let doc = crate::html::parse_document(
        r#"
            <style>
                .layout {
                    display: grid;
                    grid-template-columns: 40px 40px;
                    grid-template-areas:
                        'a side'
                        'b side';
                }
                .a { grid-area: a; height: 20px; background: #ff0000; }
                .b { grid-area: b; height: 20px; background: #00ff00; }
                .side { grid-area: side; height: 120px; background: #0000ff; }
            </style>
            <div class="layout">
                <div class="a"></div>
                <div class="b"></div>
                <div class="side"></div>
            </div>
        "#,
    );
    let viewport = Viewport {
        width_px: 220,
        height_px: 180,
    };
    let styles = crate::style::StyleComputer::from_document(&doc);
    let output = layout_document(
        &doc,
        &styles,
        &FixedMeasurer,
        viewport,
        &crate::resources::NoResources,
    )
    .expect("layout should succeed");

    let mut red = None;
    let mut green = None;
    let mut blue = None;
    for command in &output.display_list.commands {
        let DisplayCommand::Rect(rect) = command else {
            continue;
        };
        if rect.color.r == 255 && rect.color.g == 0 && rect.color.b == 0 {
            red = Some(rect.clone());
        } else if rect.color.r == 0 && rect.color.g == 255 && rect.color.b == 0 {
            green = Some(rect.clone());
        } else if rect.color.r == 0 && rect.color.g == 0 && rect.color.b == 255 {
            blue = Some(rect.clone());
        }
    }

    let red = red.expect("red area should be painted");
    let green = green.expect("green area should be painted");
    let blue = blue.expect("spanning side area should be painted");
    assert!(
        green.y_px < blue.y_px + 60,
        "row 2 should not be pushed below by the spanning side area"
    );
    assert!(
        green.y_px > red.y_px,
        "second row should still be placed after the first row"
    );
}

#[test]
fn table_layout_supports_tbody_and_th_cells() {
    let doc = crate::html::parse_document(
        r#"
            <table>
                <tbody>
                    <tr><th>Header</th><td>Value</td></tr>
                </tbody>
            </table>
        "#,
    );
    let viewport = Viewport {
        width_px: 320,
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
    .expect("layout should succeed");

    let mut saw_header = false;
    let mut saw_value = false;
    for command in &output.display_list.commands {
        let DisplayCommand::Text(text) = command else {
            continue;
        };
        if text.text == "Header" {
            saw_header = true;
        } else if text.text == "Value" {
            saw_value = true;
        }
    }

    assert!(saw_header, "table header text should be rendered");
    assert!(saw_value, "table data text should be rendered");
}

#[test]
fn renders_svg_img_as_draw_svg_command() {
    let doc = crate::html::parse_document(r#"<img src="/logo.svg" width="50" height="50">"#);
    let viewport = Viewport {
        width_px: 200,
        height_px: 200,
    };
    let styles = crate::style::StyleComputer::from_document(&doc);
    let output = layout_document(&doc, &styles, &FixedMeasurer, viewport, &SvgOnlyResources)
        .expect("layout should succeed");

    assert!(
        output
            .display_list
            .commands
            .iter()
            .any(|cmd| matches!(cmd, DisplayCommand::Svg(_))),
        "SVG image should render via DrawSvg"
    );
}

#[test]
fn media_query_can_enable_svg_img_rendering() {
    let doc = crate::html::parse_document(
        r#"
            <style>
                .logo { display: none; }
                @media all and (min-width: 640px) {
                    .logo { display: block; }
                }
            </style>
            <img class="logo" src="/logo.svg" width="50" height="50">
        "#,
    );
    let styles = crate::style::StyleComputer::from_document(&doc);

    let narrow = layout_document(
        &doc,
        &styles,
        &FixedMeasurer,
        Viewport {
            width_px: 600,
            height_px: 200,
        },
        &SvgOnlyResources,
    )
    .expect("narrow layout should succeed");
    assert!(
        !narrow
            .display_list
            .commands
            .iter()
            .any(|cmd| matches!(cmd, DisplayCommand::Svg(_))),
        "SVG should be hidden below media-query threshold"
    );

    let wide = layout_document(
        &doc,
        &styles,
        &FixedMeasurer,
        Viewport {
            width_px: 700,
            height_px: 200,
        },
        &SvgOnlyResources,
    )
    .expect("wide layout should succeed");
    assert!(
        wide.display_list
            .commands
            .iter()
            .any(|cmd| matches!(cmd, DisplayCommand::Svg(_))),
        "SVG should render when media query enables display"
    );
}
