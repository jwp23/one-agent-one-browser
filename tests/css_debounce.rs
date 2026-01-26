mod support;

use one_agent_one_browser::browser::BrowserApp;
use one_agent_one_browser::geom::Color;
use one_agent_one_browser::image::Argb32Image;
use one_agent_one_browser::render::{FontMetricsPx, Painter, TextMeasurer, TextStyle, Viewport};
use support::http::{HttpTestServer, Route};
use std::time::Duration;

const RED: Color = Color {
    r: 0xff,
    g: 0x00,
    b: 0x00,
    a: 0xff,
};

#[test]
fn stylesheet_arrivals_are_debounced_to_avoid_relayout_thrash() {
    let html = br#"<!doctype html>
<link rel="stylesheet" href="/a.css">
<link rel="stylesheet" href="/b.css">
<link rel="stylesheet" href="/c.css">
<p>hello</p>"#
        .to_vec();

    let server = HttpTestServer::new(vec![
        Route {
            path: "/index.html".to_owned(),
            status: 200,
            content_type: "text/html; charset=utf-8".to_owned(),
            body: html,
            delay: Duration::ZERO,
        },
        Route {
            path: "/a.css".to_owned(),
            status: 200,
            content_type: "text/css; charset=utf-8".to_owned(),
            body: b"p { color: #ff0000; }".to_vec(),
            delay: Duration::ZERO,
        },
        Route {
            path: "/b.css".to_owned(),
            status: 200,
            content_type: "text/css; charset=utf-8".to_owned(),
            body: b"p { font-weight: bold; }".to_vec(),
            delay: Duration::ZERO,
        },
        Route {
            path: "/c.css".to_owned(),
            status: 200,
            content_type: "text/css; charset=utf-8".to_owned(),
            body: b"/* keep pending to prevent ready_for_screenshot */".to_vec(),
            delay: Duration::from_secs(1),
        },
    ]);

    let mut app = BrowserApp::from_url(&server.url("/index.html")).unwrap();
    let viewport = Viewport {
        width_px: 240,
        height_px: 120,
    };

    wait_until(Duration::from_secs(2), || app.tick().unwrap().needs_redraw);

    let mut initial_painter = RecordingPainter::default();
    app.render(&mut initial_painter, viewport).unwrap();

    wait_until(Duration::from_secs(2), || {
        server.requests_for_path("/a.css") == 1
            && server.requests_for_path("/b.css") == 1
            && server.requests_for_path("/c.css") == 1
    });

    std::thread::sleep(Duration::from_millis(50));
    let tick = app.tick().unwrap();
    assert!(!tick.ready_for_screenshot);
    assert!(
        !tick.needs_redraw,
        "expected tick to debounce redraw while stylesheets are still arriving"
    );

    std::thread::sleep(Duration::from_millis(200));
    let tick = app.tick().unwrap();
    assert!(!tick.ready_for_screenshot);
    assert!(
        tick.needs_redraw,
        "expected tick to request redraw after debounce interval"
    );

    let mut painter = RecordingPainter::default();
    app.render(&mut painter, viewport).unwrap();
    assert!(
        painter.saw_red_text,
        "expected some text to be styled by loaded stylesheets"
    );

    server.shutdown();
}

fn wait_until(timeout: Duration, mut predicate: impl FnMut() -> bool) {
    let started = std::time::Instant::now();
    while started.elapsed() < timeout {
        if predicate() {
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timeout after {timeout:?}");
}

#[derive(Default)]
struct RecordingPainter {
    saw_red_text: bool,
}

impl TextMeasurer for RecordingPainter {
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

impl Painter for RecordingPainter {
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
        style: TextStyle,
    ) -> Result<(), String> {
        if style.color == RED {
            self.saw_red_text = true;
        }
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
