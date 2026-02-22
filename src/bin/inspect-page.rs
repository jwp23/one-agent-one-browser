use one_agent_one_browser::browser::BrowserApp;
use one_agent_one_browser::geom::Color;
use one_agent_one_browser::image::Argb32Image;
use one_agent_one_browser::render::{FontMetricsPx, Painter, TextMeasurer, TextStyle, Viewport};
use std::ffi::OsString;
use std::time::{Duration, Instant};

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            std::process::ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let args = parse_args(std::env::args_os().skip(1).collect())?;
    let mut app = BrowserApp::from_url(&args.url)?;
    let viewport = Viewport {
        width_px: args.width_px,
        height_px: args.height_px,
    };
    let mut painter = CountingPainter::default();

    let timeout = Duration::from_secs(args.timeout_secs);
    let started = Instant::now();
    let mut frame = 0usize;
    let mut rendered_once = false;
    let mut last_stats = FrameStats::default();

    while started.elapsed() < timeout {
        let tick = app.tick()?;
        let should_render = tick.needs_redraw || !rendered_once;

        if should_render {
            frame = frame.saturating_add(1);
            painter.begin_frame();
            app.render(&mut painter, viewport)?;
            last_stats = painter.end_frame();
            rendered_once = true;

            println!(
                "frame={frame} ready={} pending={} text={} image={} svg={} svg_bytes={} image_px={}",
                tick.ready_for_screenshot,
                tick.pending_resources,
                last_stats.text_count,
                last_stats.image_count,
                last_stats.svg_count,
                last_stats.svg_total_bytes,
                last_stats.image_total_pixels
            );
            if !painter.svg_details.is_empty() {
                for (idx, detail) in painter.svg_details.iter().enumerate() {
                    println!(
                        "  svg[{idx}] x={} y={} w={} h={} bytes={}",
                        detail.x_px, detail.y_px, detail.width_px, detail.height_px, detail.bytes
                    );
                }
            }
            if !painter.image_details.is_empty() {
                for (idx, detail) in painter.image_details.iter().enumerate() {
                    println!(
                        "  img[{idx}] x={} y={} w={} h={} bytes={}",
                        detail.x_px, detail.y_px, detail.width_px, detail.height_px, detail.bytes
                    );
                }
            }
            if !painter.text_details.is_empty() {
                for (idx, detail) in painter
                    .text_details
                    .iter()
                    .filter(|detail| detail.y_px < 220)
                    .take(20)
                    .enumerate()
                {
                    println!(
                        "  txt[{idx}] x={} y={} {:?}",
                        detail.x_px, detail.y_px, detail.text
                    );
                }
                for detail in painter.text_details.iter().filter(|detail| {
                    detail.text.contains("Appearance")
                        || detail.text.contains("Small")
                        || detail.text.contains("Standard")
                        || detail.text.contains("Width")
                        || detail.text.contains("Color")
                        || detail.text.contains("Automatic")
                        || detail.text.contains("Light")
                        || detail.text.contains("Dark")
                        || detail.text.contains("(o)")
                        || detail.text.contains("( )")
                }) {
                    println!(
                        "  txt[appearance] x={} y={} {:?}",
                        detail.x_px, detail.y_px, detail.text
                    );
                }
            }
        }

        if rendered_once
            && tick.ready_for_screenshot
            && tick.pending_resources == 0
            && !tick.needs_redraw
        {
            break;
        }

        std::thread::sleep(Duration::from_millis(10));
    }

    println!(
        "final frames={} text={} image={} svg={} svg_bytes={} image_px={}",
        frame,
        last_stats.text_count,
        last_stats.image_count,
        last_stats.svg_count,
        last_stats.svg_total_bytes,
        last_stats.image_total_pixels
    );

    Ok(())
}

#[derive(Debug)]
struct Args {
    url: String,
    width_px: i32,
    height_px: i32,
    timeout_secs: u64,
}

fn parse_args(args: Vec<OsString>) -> Result<Args, String> {
    let mut url: Option<String> = None;
    let mut width_px = 1366i32;
    let mut height_px = 768i32;
    let mut timeout_secs = 20u64;

    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        let Some(arg) = arg.to_str() else {
            return Err("Argument is not valid UTF-8".to_owned());
        };
        match arg {
            "--width" => {
                let Some(value) = it.next() else {
                    return Err("Missing value for --width".to_owned());
                };
                let Some(value) = value.to_str() else {
                    return Err("Invalid --width value".to_owned());
                };
                width_px = value
                    .parse::<i32>()
                    .map_err(|_| format!("Invalid --width value: {value}"))?;
            }
            "--height" => {
                let Some(value) = it.next() else {
                    return Err("Missing value for --height".to_owned());
                };
                let Some(value) = value.to_str() else {
                    return Err("Invalid --height value".to_owned());
                };
                height_px = value
                    .parse::<i32>()
                    .map_err(|_| format!("Invalid --height value: {value}"))?;
            }
            "--timeout" => {
                let Some(value) = it.next() else {
                    return Err("Missing value for --timeout".to_owned());
                };
                let Some(value) = value.to_str() else {
                    return Err("Invalid --timeout value".to_owned());
                };
                timeout_secs = value
                    .parse::<u64>()
                    .map_err(|_| format!("Invalid --timeout value: {value}"))?;
            }
            _ if arg.starts_with('-') => {
                return Err(format!("Unknown flag: {arg}"));
            }
            _ => {
                if url.is_some() {
                    return Err("Unexpected extra positional argument".to_owned());
                }
                url = Some(arg.to_owned());
            }
        }
    }

    let Some(url) = url else {
        return Err(
            "Usage: inspect-page <url> [--width <px>] [--height <px>] [--timeout <seconds>]"
                .to_owned(),
        );
    };
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_owned());
    }
    if width_px <= 0 || height_px <= 0 {
        return Err("Viewport dimensions must be positive".to_owned());
    }
    if timeout_secs == 0 {
        return Err("Timeout must be >= 1 second".to_owned());
    }

    Ok(Args {
        url,
        width_px,
        height_px,
        timeout_secs,
    })
}

#[derive(Clone, Copy, Debug, Default)]
struct FrameStats {
    text_count: usize,
    image_count: usize,
    svg_count: usize,
    image_total_pixels: u64,
    svg_total_bytes: u64,
}

#[derive(Default)]
struct CountingPainter {
    current: FrameStats,
    svg_details: Vec<AssetDraw>,
    image_details: Vec<AssetDraw>,
    text_details: Vec<TextDraw>,
}

impl CountingPainter {
    fn begin_frame(&mut self) {
        self.current = FrameStats::default();
        self.svg_details.clear();
        self.image_details.clear();
        self.text_details.clear();
    }

    fn end_frame(&self) -> FrameStats {
        self.current
    }
}

#[derive(Clone, Debug)]
struct AssetDraw {
    x_px: i32,
    y_px: i32,
    width_px: i32,
    height_px: i32,
    bytes: usize,
}

#[derive(Clone, Debug)]
struct TextDraw {
    x_px: i32,
    y_px: i32,
    text: String,
}

impl TextMeasurer for CountingPainter {
    fn font_metrics_px(&self, _style: TextStyle) -> FontMetricsPx {
        FontMetricsPx {
            ascent_px: 12,
            descent_px: 4,
        }
    }

    fn text_width_px(&self, text: &str, _style: TextStyle) -> Result<i32, String> {
        Ok((text.chars().count() as i32).saturating_mul(8))
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
        x_px: i32,
        y_px: i32,
        text: &str,
        _style: TextStyle,
    ) -> Result<(), String> {
        self.current.text_count = self.current.text_count.saturating_add(1);
        if self.text_details.len() < 1000 {
            self.text_details.push(TextDraw {
                x_px,
                y_px,
                text: text.chars().take(48).collect(),
            });
        }
        Ok(())
    }

    fn draw_image(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        image: &Argb32Image,
        _opacity: u8,
    ) -> Result<(), String> {
        self.current.image_count = self.current.image_count.saturating_add(1);
        let pixels = (width_px.max(0) as u64).saturating_mul(height_px.max(0) as u64);
        self.current.image_total_pixels = self.current.image_total_pixels.saturating_add(pixels);
        self.image_details.push(AssetDraw {
            x_px,
            y_px,
            width_px,
            height_px,
            bytes: image.data.len(),
        });
        Ok(())
    }

    fn draw_svg(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        svg_xml: &str,
        _opacity: u8,
    ) -> Result<(), String> {
        self.current.svg_count = self.current.svg_count.saturating_add(1);
        self.current.svg_total_bytes = self
            .current
            .svg_total_bytes
            .saturating_add(svg_xml.len() as u64);
        self.svg_details.push(AssetDraw {
            x_px,
            y_px,
            width_px,
            height_px,
            bytes: svg_xml.len(),
        });
        Ok(())
    }

    fn flush(&mut self) -> Result<(), String> {
        Ok(())
    }
}
