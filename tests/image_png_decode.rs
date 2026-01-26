mod support;

use one_agent_one_browser::browser::BrowserApp;
use one_agent_one_browser::geom::Color;
use one_agent_one_browser::image::{Argb32Image, RgbImage};
use one_agent_one_browser::render::{FontMetricsPx, Painter, TextMeasurer, TextStyle, Viewport};
use support::http::{HttpTestServer, Route};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[test]
fn png_images_decode_and_render() {
    let png_bytes = make_test_png_bytes().unwrap();
    let html = br#"<!doctype html><img src="/pixel.png" width="10" height="10">"#.to_vec();

    let server = HttpTestServer::new(vec![
        Route {
            path: "/index.html".to_owned(),
            status: 200,
            content_type: "text/html; charset=utf-8".to_owned(),
            body: html,
            delay: Duration::ZERO,
        },
        Route {
            path: "/pixel.png".to_owned(),
            status: 200,
            content_type: "image/png".to_owned(),
            body: png_bytes,
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
    assert_eq!(server.requests_for_path("/pixel.png"), 1);

    server.shutdown();
}

fn make_test_png_bytes() -> Result<Vec<u8>, String> {
    let image = RgbImage::new(1, 1, vec![0xff, 0x00, 0x00])?;
    let path = temp_path("png");
    let temp = TempPath(path.clone());
    one_agent_one_browser::png::write_rgb_png(&path, &image)?;
    let bytes = std::fs::read(&path)
        .map_err(|err| format!("Failed to read {}: {err}", path.display()))?;
    drop(temp);
    Ok(bytes)
}

fn temp_path(ext: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("one-agent-one-browser-{nanos}.{ext}"))
}

struct TempPath(PathBuf);

impl Drop for TempPath {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
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

