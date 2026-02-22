mod support;

use one_agent_one_browser::browser::BrowserApp;
use one_agent_one_browser::geom::Color;
use one_agent_one_browser::image::Argb32Image;
use one_agent_one_browser::render::{FontMetricsPx, Painter, TextMeasurer, TextStyle, Viewport};
use std::time::{Duration, Instant};
use support::http::{HttpTestServer, Route};

#[test]
fn unsupported_images_are_negative_cached() {
    let html = br#"<!doctype html><img src="/unsupported.gif" width="20" height="20">"#.to_vec();
    let gif_bytes = b"GIF89anot-a-real-gif".to_vec();

    let server = HttpTestServer::new(vec![
        Route {
            path: "/index.html".to_owned(),
            status: 200,
            content_type: "text/html; charset=utf-8".to_owned(),
            body: html,
            delay: Duration::ZERO,
        },
        Route {
            path: "/unsupported.gif".to_owned(),
            status: 200,
            content_type: "image/gif".to_owned(),
            body: gif_bytes,
            delay: Duration::ZERO,
        },
    ]);

    let mut app = BrowserApp::from_url(&server.url("/index.html")).unwrap();
    wait_until(Duration::from_secs(2), || {
        app.tick().unwrap().ready_for_screenshot
    });

    let mut painter = NoopPainter;
    let viewport1 = Viewport {
        width_px: 200,
        height_px: 200,
    };
    app.render(&mut painter, viewport1).unwrap();

    wait_until(Duration::from_secs(2), || {
        server.requests_for_path("/unsupported.gif") == 1
    });
    for _ in 0..200 {
        let _ = app.tick();
        std::thread::sleep(Duration::from_millis(10));
    }

    let viewport2 = Viewport {
        width_px: 201,
        height_px: 200,
    };
    app.render(&mut painter, viewport2).unwrap();

    std::thread::sleep(Duration::from_millis(250));
    assert_eq!(server.requests_for_path("/unsupported.gif"), 1);

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
