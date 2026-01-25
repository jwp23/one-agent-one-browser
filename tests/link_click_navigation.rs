use one_agent_one_browser::app::App;
use one_agent_one_browser::browser::BrowserApp;
use one_agent_one_browser::geom::Color;
use one_agent_one_browser::image::Argb32Image;
use one_agent_one_browser::render::{FontMetricsPx, Painter, TextMeasurer, TextStyle, Viewport};

struct NoopPainter;

impl TextMeasurer for NoopPainter {
    fn font_metrics_px(&self, _style: TextStyle) -> FontMetricsPx {
        FontMetricsPx {
            ascent_px: 8,
            descent_px: 2,
        }
    }

    fn text_width_px(&self, text: &str, _style: TextStyle) -> Result<i32, String> {
        Ok(text.len() as i32)
    }
}

impl Painter for NoopPainter {
    fn clear(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn push_opacity(&mut self, _opacity: u8) -> Result<(), String> {
        Ok(())
    }

    fn pop_opacity(&mut self, _opacity: u8) -> Result<(), String> {
        Ok(())
    }

    fn fill_rect(
        &mut self,
        _x_px: i32,
        _y_px: i32,
        _width_px: i32,
        _height_px: i32,
        _color: Color,
    ) -> Result<(), String> {
        Ok(())
    }

    fn fill_rounded_rect(
        &mut self,
        _x_px: i32,
        _y_px: i32,
        _width_px: i32,
        _height_px: i32,
        _radius_px: i32,
        _color: Color,
    ) -> Result<(), String> {
        Ok(())
    }

    fn stroke_rounded_rect(
        &mut self,
        _x_px: i32,
        _y_px: i32,
        _width_px: i32,
        _height_px: i32,
        _radius_px: i32,
        _border_width_px: i32,
        _color: Color,
    ) -> Result<(), String> {
        Ok(())
    }

    fn draw_text(
        &mut self,
        _x_px: i32,
        _y_px: i32,
        _text: &str,
        _style: TextStyle,
    ) -> Result<(), String> {
        Ok(())
    }

    fn draw_image(
        &mut self,
        _x_px: i32,
        _y_px: i32,
        _width_px: i32,
        _height_px: i32,
        _image: &Argb32Image,
        _opacity: u8,
    ) -> Result<(), String> {
        Ok(())
    }

    fn draw_svg(
        &mut self,
        _x_px: i32,
        _y_px: i32,
        _width_px: i32,
        _height_px: i32,
        _svg_xml: &str,
        _opacity: u8,
    ) -> Result<(), String> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), String> {
        Ok(())
    }
}

#[test]
fn clicks_anchor_navigates_to_file() {
    let root = std::env::temp_dir().join(format!(
        "one-agent-one-browser-link-click-{}",
        unique_id()
    ));
    std::fs::create_dir_all(&root).unwrap();

    let page1 = root.join("page1.html");
    let page2 = root.join("page2.html");

    std::fs::write(&page1, r#"<p><a href="page2.html">Go</a></p>"#).unwrap();
    std::fs::write(&page2, "<p>Page 2</p>").unwrap();

    let mut app = BrowserApp::from_file(&page1).unwrap();
    let viewport = Viewport {
        width_px: 200,
        height_px: 200,
    };

    let mut painter = NoopPainter;
    app.render(&mut painter, viewport).unwrap();

    let click = app.mouse_down(0, 0, viewport).unwrap();
    assert!(click.needs_redraw);
    assert_eq!(app.title(), "page2.html");

    let _ = std::fs::remove_file(&page1);
    let _ = std::fs::remove_file(&page2);
    let _ = std::fs::remove_dir(&root);
}

fn unique_id() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
}
