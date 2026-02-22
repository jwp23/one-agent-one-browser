mod support;

use one_agent_one_browser::browser::BrowserApp;
use one_agent_one_browser::geom::Color;
use one_agent_one_browser::image::Argb32Image;
use one_agent_one_browser::render::{FontMetricsPx, Painter, TextMeasurer, TextStyle, Viewport};
use std::time::Duration;
use support::http::{HttpTestServer, Route};

const JPEG_BASE64: &str = "/9j/4AAQSkZJRgABAQAAAQABAAD/2wBDAAMCAgICAgMCAgIDAwMDBAYEBAQEBAgGBgUGCQgKCgkICQkKDA8MCgsOCwkJDRENDg8QEBEQCgwSExIQEw8QEBD/2wBDAQMDAwQDBAgEBAgQCwkLEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBD/wAARCAABAAEDAREAAhEBAxEB/8QAFAABAAAAAAAAAAAAAAAAAAAACP/EABQQAQAAAAAAAAAAAAAAAAAAAAD/xAAVAQEBAAAAAAAAAAAAAAAAAAAHCf/EABQRAQAAAAAAAAAAAAAAAAAAAAD/2gAMAwEAAhEDEQA/ADoDFU3/2Q==";

#[test]
fn jpeg_images_decode_and_render() {
    let jpeg_bytes = decode_base64(JPEG_BASE64).unwrap();
    let html = br#"<!doctype html><img src="/pixel.jpg" width="10" height="10">"#.to_vec();

    let server = HttpTestServer::new(vec![
        Route {
            path: "/index.html".to_owned(),
            status: 200,
            content_type: "text/html; charset=utf-8".to_owned(),
            body: html,
            delay: Duration::ZERO,
        },
        Route {
            path: "/pixel.jpg".to_owned(),
            status: 200,
            content_type: "image/jpeg".to_owned(),
            body: jpeg_bytes,
            delay: Duration::ZERO,
        },
    ]);

    let mut app = BrowserApp::from_url(&server.url("/index.html")).unwrap();
    let viewport = Viewport {
        width_px: 64,
        height_px: 64,
    };

    wait_until(Duration::from_secs(2), || {
        app.tick().unwrap().ready_for_screenshot
    });

    let mut first = CountingPainter::default();
    app.render(&mut first, viewport).unwrap();
    assert_eq!(first.images_drawn, 0);

    wait_until(Duration::from_secs(2), || app.tick().unwrap().needs_redraw);

    let mut second = CountingPainter::default();
    app.render(&mut second, viewport).unwrap();
    assert!(second.images_drawn > 0);
    assert_eq!(server.requests_for_path("/pixel.jpg"), 1);

    server.shutdown();
}

fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    let mut acc: u32 = 0;
    let mut bits: u8 = 0;

    for byte in input.bytes() {
        if byte.is_ascii_whitespace() {
            continue;
        }
        if byte == b'=' {
            break;
        }
        let value = base64_value(byte)
            .ok_or_else(|| format!("Invalid base64 character: {}", byte as char))?;

        acc = (acc << 6) | (value as u32);
        bits = bits.saturating_add(6);

        while bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xff) as u8);
            if bits == 0 {
                acc = 0;
            } else {
                acc &= (1u32 << bits) - 1;
            }
        }
    }

    Ok(out)
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
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
