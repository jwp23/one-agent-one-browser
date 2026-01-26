mod support;

use one_agent_one_browser::browser::BrowserApp;
use one_agent_one_browser::geom::Color;
use one_agent_one_browser::image::Argb32Image;
use one_agent_one_browser::render::{FontMetricsPx, Painter, TextMeasurer, TextStyle, Viewport};
use support::http::{HttpTestServer, Route};
use std::time::{Duration, Instant};

#[test]
fn image_fetch_does_not_block_render_and_pops_in_later() {
    let webp = std::fs::read("tests/cases/medium-assets/hero.webp").unwrap();
    let html = br#"<!doctype html><img src="/slow.webp" width="20" height="20">"#.to_vec();
    let server = HttpTestServer::new(vec![
        Route {
            path: "/index.html".to_owned(),
            status: 200,
            content_type: "text/html; charset=utf-8".to_owned(),
            body: html,
            delay: Duration::ZERO,
        },
        Route {
            path: "/slow.webp".to_owned(),
            status: 200,
            content_type: "image/webp".to_owned(),
            body: webp,
            delay: Duration::from_secs(2),
        },
    ]);

    let mut app = BrowserApp::from_url(&server.url("/index.html")).unwrap();
    let viewport = Viewport {
        width_px: 200,
        height_px: 200,
    };

    wait_until(Duration::from_secs(2), || {
        app.tick().unwrap().ready_for_screenshot
    });

    let mut first_painter = CountingPainter::default();
    let started = Instant::now();
    app.render(&mut first_painter, viewport).unwrap();
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "render unexpectedly blocked on network I/O"
    );
    assert_eq!(first_painter.images_drawn, 0);

    wait_until(Duration::from_secs(5), || app.tick().unwrap().needs_redraw);

    let mut second_painter = CountingPainter::default();
    app.render(&mut second_painter, viewport).unwrap();
    assert!(
        second_painter.images_drawn > 0,
        "expected image to render after it finished downloading"
    );
    assert_eq!(server.requests_for_path("/slow.webp"), 1);

    server.shutdown();
}

fn wait_until(timeout: Duration, mut predicate: impl FnMut() -> bool) {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if predicate() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timeout after {timeout:?}");
}

#[derive(Default)]
struct CountingPainter {
    images_drawn: usize,
}

impl TextMeasurer for CountingPainter {
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

impl Painter for CountingPainter {
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
        self.images_drawn = self.images_drawn.saturating_add(1);
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
