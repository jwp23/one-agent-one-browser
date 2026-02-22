use super::WindowOptions;
use super::painter::WinPainter;
use super::scale::ScaleFactor;
use super::scaled::ScaledPainter;
use crate::app::App;
use crate::render::Viewport;
use std::time::{Duration, Instant};

const SCREENSHOT_RESOURCE_WAIT_TIMEOUT: Duration = Duration::from_secs(5);

pub(super) fn run<A: App>(options: WindowOptions, app: &mut A) -> Result<(), String> {
    let initial_width_css = options.initial_width_px.unwrap_or(1024);
    let initial_height_css = options.initial_height_px.unwrap_or(768);
    if initial_width_css <= 0 || initial_height_css <= 0 {
        return Err(format!(
            "Invalid initial window size: {initial_width_css}x{initial_height_css}"
        ));
    }

    let scale = ScaleFactor::detect(true, None);
    let initial_width_device = scale.css_size_to_device_px(initial_width_css);
    let initial_height_device = scale.css_size_to_device_px(initial_height_css);

    let mut painter = WinPainter::new(
        Viewport {
            width_px: initial_width_device,
            height_px: initial_height_device,
        },
        None,
    )?;

    let viewport = Viewport {
        width_px: initial_width_device,
        height_px: initial_height_device,
    };
    let css_viewport = Viewport {
        width_px: scale.device_size_to_css_px(viewport.width_px),
        height_px: scale.device_size_to_css_px(viewport.height_px),
    };

    let mut screenshot_path = options.screenshot_path;
    let headless = options.headless;

    let mut needs_redraw = true;
    let mut has_rendered_ready_state = false;
    let mut resource_wait_started: Option<Instant> = None;

    loop {
        let tick = app.tick()?;
        if tick.needs_redraw {
            needs_redraw = true;
        }
        let ready_for_screenshot = tick.ready_for_screenshot;
        if !ready_for_screenshot {
            has_rendered_ready_state = false;
            resource_wait_started = None;
        }

        let should_wait_for_resources = tick.pending_resources > 0;
        let timed_out_waiting_for_resources = resource_wait_started
            .is_some_and(|started| started.elapsed() >= SCREENSHOT_RESOURCE_WAIT_TIMEOUT);
        let can_complete = !should_wait_for_resources || timed_out_waiting_for_resources;

        let wants_screenshot = screenshot_path.is_some();
        let should_complete_headless = headless && !wants_screenshot;
        let should_complete_screenshot =
            wants_screenshot && ready_for_screenshot && has_rendered_ready_state;

        let mut capture_now = false;
        let mut capture_after_render = false;
        let mut exit_headless_now = false;

        if ready_for_screenshot && (wants_screenshot || headless) && !has_rendered_ready_state {
            needs_redraw = true;
        } else if ready_for_screenshot && should_wait_for_resources && has_rendered_ready_state {
            resource_wait_started.get_or_insert(Instant::now());
        } else if ready_for_screenshot && has_rendered_ready_state {
            resource_wait_started = None;
        }

        if ready_for_screenshot && has_rendered_ready_state && can_complete {
            if should_complete_screenshot {
                if needs_redraw {
                    capture_after_render = true;
                } else {
                    capture_now = true;
                }
            } else if should_complete_headless && !needs_redraw {
                exit_headless_now = true;
            }
        }

        if exit_headless_now {
            break;
        }

        if capture_now {
            let Some(path) = screenshot_path.take() else {
                return Err(
                    "Internal error: capture_now set but screenshot path missing".to_owned(),
                );
            };
            let rgb = painter.capture_back_buffer_rgb()?;
            crate::png::write_rgb_png(&path, &rgb)?;
            break;
        }

        if needs_redraw {
            painter.ensure_back_buffer(viewport)?;
            let mut scaled_painter = ScaledPainter::new(&mut painter, scale);
            app.render(&mut scaled_painter, css_viewport)?;
            needs_redraw = false;

            if ready_for_screenshot {
                has_rendered_ready_state = true;
                if capture_after_render {
                    let Some(path) = screenshot_path.take() else {
                        return Err(
                            "Internal error: capture_after_render set but screenshot path missing"
                                .to_owned(),
                        );
                    };
                    let rgb = painter.capture_back_buffer_rgb()?;
                    crate::png::write_rgb_png(&path, &rgb)?;
                    break;
                }
            }
        }

        if !needs_redraw {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    Ok(())
}
