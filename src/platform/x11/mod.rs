mod cairo;
mod painter;
mod scale;
mod xft;
mod xlib;

use super::WindowOptions;
use crate::app::App;
use crate::geom::Color;
use crate::image::Argb32Image;
use crate::render::{FontMetricsPx, Painter, TextMeasurer, TextStyle, Viewport};
use core::ffi::{c_int, c_uint, c_ulong};
use std::ffi::CString;
use std::time::{Duration, Instant};

use painter::X11Painter;
use scale::ScaleFactor;
use xlib::*;

// Avoid starving rendering when the X server generates events faster than we can drain them
// (e.g. during drag-resize). Rendering at least once per tick keeps the window responsive.
const MAX_X11_EVENTS_PER_TICK: usize = 512;

const SCREENSHOT_RESOURCE_WAIT_TIMEOUT: Duration = Duration::from_secs(5);

const WHEEL_SCROLL_STEP_PX: i32 = 48;

pub fn run_window<A: App>(title: &str, options: WindowOptions, app: &mut A) -> Result<(), String> {
    let display = unsafe { XOpenDisplay(std::ptr::null()) };
    if display.is_null() {
        return Err("XOpenDisplay failed: is $DISPLAY set and an X server available?".to_owned());
    }

    let result = run_window_with_display(display, title, options, app);

    unsafe {
        XCloseDisplay(display);
    }

    result
}

fn run_window_with_display<A: App>(
    display: *mut Display,
    title: &str,
    options: WindowOptions,
    app: &mut A,
) -> Result<(), String> {
    let screen = unsafe { XDefaultScreen(display) };
    let scale = ScaleFactor::detect(display, screen);
    let visual = unsafe { XDefaultVisual(display, screen) };
    if visual.is_null() {
        return Err("XDefaultVisual returned null".to_owned());
    }
    let visual_masks = unsafe {
        (
            (*visual).red_mask,
            (*visual).green_mask,
            (*visual).blue_mask,
        )
    };
    let colormap = unsafe { XDefaultColormap(display, screen) };
    let root_window = unsafe { XRootWindow(display, screen) };
    let black_pixel = unsafe { XBlackPixel(display, screen) };
    let white_pixel = unsafe { XWhitePixel(display, screen) };

    let initial_width_css_i32 = options.initial_width_px.unwrap_or(1024);
    let initial_height_css_i32 = options.initial_height_px.unwrap_or(768);
    if initial_width_css_i32 <= 0 || initial_height_css_i32 <= 0 {
        return Err(format!(
            "Invalid initial window size: {initial_width_css_i32}x{initial_height_css_i32}"
        ));
    }
    let initial_width_device_i32 = scale.css_size_to_device_px(initial_width_css_i32);
    let initial_height_device_i32 = scale.css_size_to_device_px(initial_height_css_i32);
    let initial_width: c_uint = initial_width_device_i32
        .try_into()
        .map_err(|_| format!("Initial width out of range: {initial_width_device_i32}"))?;
    let initial_height: c_uint = initial_height_device_i32
        .try_into()
        .map_err(|_| format!("Initial height out of range: {initial_height_device_i32}"))?;

    let window = unsafe {
        XCreateSimpleWindow(
            display,
            root_window,
            0,
            0,
            initial_width,
            initial_height,
            1,
            black_pixel,
            white_pixel,
        )
    };

    let window_title =
        CString::new(title).map_err(|_| "Window title contains a NUL byte".to_owned())?;
    unsafe {
        XStoreName(display, window, window_title.as_ptr());
    }

    let wm_protocols_atom_name =
        CString::new("WM_PROTOCOLS").map_err(|_| "Invalid atom name".to_owned())?;
    let wm_protocols_atom = unsafe { XInternAtom(display, wm_protocols_atom_name.as_ptr(), 0) };

    let wm_delete_window_atom_name =
        CString::new("WM_DELETE_WINDOW").map_err(|_| "Invalid atom name".to_owned())?;
    let wm_delete_window = unsafe { XInternAtom(display, wm_delete_window_atom_name.as_ptr(), 0) };
    let mut wm_protocols = [wm_delete_window];
    unsafe {
        XSetWMProtocols(
            display,
            window,
            wm_protocols.as_mut_ptr(),
            wm_protocols.len() as c_int,
        );
    }

    unsafe {
        if !options.headless {
            XSelectInput(
                display,
                window,
                EVENT_MASK_EXPOSURE
                    | EVENT_MASK_KEY_PRESS
                    | EVENT_MASK_BUTTON_PRESS
                    | EVENT_MASK_STRUCTURE_NOTIFY,
            );
            XMapWindow(display, window);
        }
    }

    let depth_i32 = unsafe { XDefaultDepth(display, screen) };
    let depth: c_uint = depth_i32
        .try_into()
        .map_err(|_| format!("XDefaultDepth returned an invalid value: {depth_i32}"))?;

    let gc = unsafe { XDefaultGC(display, screen) };
    unsafe {
        XSetForeground(display, gc, black_pixel);
        XSetBackground(display, gc, white_pixel);
    }

    let back_buffer =
        unsafe { XCreatePixmap(display, window, initial_width, initial_height, depth) };
    if back_buffer == 0 {
        return Err("XCreatePixmap failed".to_owned());
    }

    let mut painter = X11Painter::new(
        display,
        window,
        gc,
        back_buffer,
        initial_width,
        initial_height,
        depth,
        black_pixel,
        white_pixel,
        visual_masks,
        visual,
        colormap,
        screen,
    )?;

    let mut viewport = Viewport {
        width_px: initial_width_device_i32,
        height_px: initial_height_device_i32,
    };
    let mut css_viewport = Viewport {
        width_px: scale.device_size_to_css_px(viewport.width_px),
        height_px: scale.device_size_to_css_px(viewport.height_px),
    };

    let mut screenshot_path = options.screenshot_path;
    let headless = options.headless;

    let loop_result = (|| {
        let mut needs_redraw = true;
        let mut should_exit = false;
        let mut has_rendered_ready_state = false;
        let mut resource_wait_started: Option<Instant> = None;

        loop {
            let mut processed_events = 0usize;
            while unsafe { XPending(display) } > 0 && processed_events < MAX_X11_EVENTS_PER_TICK {
                let mut event = XEvent { inner: [0; 24] };
                unsafe {
                    XNextEvent(display, &mut event);
                }

                match event.event_type() {
                    EVENT_TYPE_EXPOSE => {
                        let expose: &XExposeEvent =
                            unsafe { &*(event.inner.as_ptr() as *const XExposeEvent) };
                        if expose.count == 0 {
                            needs_redraw = true;
                        }
                    }
                    EVENT_TYPE_CONFIGURE_NOTIFY => {
                        let configure: &XConfigureEvent =
                            unsafe { &*(event.inner.as_ptr() as *const XConfigureEvent) };
                        viewport = Viewport {
                            width_px: configure.width,
                            height_px: configure.height,
                        };
                        css_viewport = Viewport {
                            width_px: scale.device_size_to_css_px(viewport.width_px),
                            height_px: scale.device_size_to_css_px(viewport.height_px),
                        };
                        needs_redraw = true;
                        has_rendered_ready_state = false;
                        resource_wait_started = None;
                    }
                    EVENT_TYPE_BUTTON_PRESS => {
                        let button: &XButtonEvent =
                            unsafe { &*(event.inner.as_ptr() as *const XButtonEvent) };
                        if button.button == 1 {
                            let x_css = scale.device_coord_to_css_px(button.x);
                            let y_css = scale.device_coord_to_css_px(button.y);
                            let tick = app.mouse_down(x_css, y_css, css_viewport)?;
                            if tick.needs_redraw {
                                needs_redraw = true;
                            }
                        } else if button.button == 8 {
                            let tick = app.navigate_back()?;
                            if tick.needs_redraw {
                                needs_redraw = true;
                            }
                        } else if button.button == 4 || button.button == 5 {
                            let delta_y_px = if button.button == 4 {
                                -WHEEL_SCROLL_STEP_PX
                            } else {
                                WHEEL_SCROLL_STEP_PX
                            };
                            let delta_y_css = scale.device_delta_to_css_px(delta_y_px);
                            let tick = app.mouse_wheel(delta_y_css, css_viewport)?;
                            if tick.needs_redraw {
                                needs_redraw = true;
                            }
                        }
                    }
                    EVENT_TYPE_KEY_PRESS => {
                        let key: &XKeyEvent =
                            unsafe { &*(event.inner.as_ptr() as *const XKeyEvent) };
                        let keysym =
                            unsafe { XLookupKeysym(key as *const XKeyEvent as *mut XKeyEvent, 0) };
                        if keysym == KEYSYM_ESCAPE {
                            should_exit = true;
                            break;
                        }
                    }
                    EVENT_TYPE_CLIENT_MESSAGE => {
                        let message: &XClientMessageEvent =
                            unsafe { &*(event.inner.as_ptr() as *const XClientMessageEvent) };
                        let data = unsafe { message.data.l };
                        if message.message_type == wm_protocols_atom
                            && data[0] as c_ulong == wm_delete_window
                        {
                            should_exit = true;
                            break;
                        }
                    }
                    _ => {}
                }
                processed_events += 1;
            }

            if should_exit {
                break;
            }

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
            } else if ready_for_screenshot && should_wait_for_resources && has_rendered_ready_state
            {
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
                unsafe {
                    XSync(display, 0);
                }
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
                            return Err("Internal error: capture_after_render set but screenshot path missing".to_owned());
                        };
                        unsafe {
                            XSync(display, 0);
                        }
                        let rgb = painter.capture_back_buffer_rgb()?;
                        crate::png::write_rgb_png(&path, &rgb)?;
                        break;
                    }
                }
            }

            if unsafe { XPending(display) } == 0 && !needs_redraw {
                std::thread::sleep(Duration::from_millis(10));
            }
        }

        Ok(())
    })();

    painter.destroy_xft_resources();

    unsafe {
        XFreePixmap(display, painter.back_buffer());
        XDestroyWindow(display, window);
        XFlush(display);
    }

    loop_result
}

struct ScaledPainter<'a> {
    inner: &'a mut X11Painter,
    scale: ScaleFactor,
}

impl<'a> ScaledPainter<'a> {
    fn new(inner: &'a mut X11Painter, scale: ScaleFactor) -> Self {
        Self { inner, scale }
    }

    fn scale_style(&self, style: TextStyle) -> TextStyle {
        TextStyle {
            font_size_px: self.scale.css_size_to_device_px(style.font_size_px),
            letter_spacing_px: self.scale.css_coord_to_device_px(style.letter_spacing_px),
            ..style
        }
    }
}

impl TextMeasurer for ScaledPainter<'_> {
    fn font_metrics_px(&self, style: TextStyle) -> FontMetricsPx {
        let scaled_style = self.scale_style(style);
        let metrics = self.inner.font_metrics_px(scaled_style);
        FontMetricsPx {
            ascent_px: self.scale.device_delta_to_css_px(metrics.ascent_px).max(1),
            descent_px: self.scale.device_delta_to_css_px(metrics.descent_px).max(0),
        }
    }

    fn text_width_px(&self, text: &str, style: TextStyle) -> Result<i32, String> {
        let scaled_style = self.scale_style(style);
        let width_device_px = self.inner.text_width_px(text, scaled_style)?;
        Ok(self.scale.device_delta_to_css_px(width_device_px).max(0))
    }
}

impl Painter for ScaledPainter<'_> {
    fn clear(&mut self) -> Result<(), String> {
        self.inner.clear()
    }

    fn push_opacity(&mut self, opacity: u8) -> Result<(), String> {
        self.inner.push_opacity(opacity)
    }

    fn pop_opacity(&mut self, opacity: u8) -> Result<(), String> {
        self.inner.pop_opacity(opacity)
    }

    fn fill_rect(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        color: Color,
    ) -> Result<(), String> {
        let (x_device_px, width_device_px) = self.scale.css_span_to_device_px(x_px, width_px);
        let (y_device_px, height_device_px) = self.scale.css_span_to_device_px(y_px, height_px);
        self.inner.fill_rect(
            x_device_px,
            y_device_px,
            width_device_px,
            height_device_px,
            color,
        )
    }

    fn fill_rounded_rect(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        radius_px: i32,
        color: Color,
    ) -> Result<(), String> {
        let (x_device_px, width_device_px) = self.scale.css_span_to_device_px(x_px, width_px);
        let (y_device_px, height_device_px) = self.scale.css_span_to_device_px(y_px, height_px);
        let radius_device_px = self.scale.css_coord_to_device_px(radius_px).max(0);
        self.inner.fill_rounded_rect(
            x_device_px,
            y_device_px,
            width_device_px,
            height_device_px,
            radius_device_px,
            color,
        )
    }

    fn stroke_rounded_rect(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        radius_px: i32,
        border_width_px: i32,
        color: Color,
    ) -> Result<(), String> {
        let (x_device_px, width_device_px) = self.scale.css_span_to_device_px(x_px, width_px);
        let (y_device_px, height_device_px) = self.scale.css_span_to_device_px(y_px, height_px);
        let radius_device_px = self.scale.css_coord_to_device_px(radius_px).max(0);
        let border_width_device_px = self.scale.css_coord_to_device_px(border_width_px).max(0);
        self.inner.stroke_rounded_rect(
            x_device_px,
            y_device_px,
            width_device_px,
            height_device_px,
            radius_device_px,
            border_width_device_px,
            color,
        )
    }

    fn draw_text(
        &mut self,
        x_px: i32,
        y_px: i32,
        text: &str,
        style: TextStyle,
    ) -> Result<(), String> {
        let x_device_px = self.scale.css_coord_to_device_px(x_px);
        let y_device_px = self.scale.css_coord_to_device_px(y_px);
        let style = self.scale_style(style);
        self.inner.draw_text(x_device_px, y_device_px, text, style)
    }

    fn draw_image(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        image: &Argb32Image,
        opacity: u8,
    ) -> Result<(), String> {
        let (x_device_px, width_device_px) = self.scale.css_span_to_device_px(x_px, width_px);
        let (y_device_px, height_device_px) = self.scale.css_span_to_device_px(y_px, height_px);
        self.inner.draw_image(
            x_device_px,
            y_device_px,
            width_device_px,
            height_device_px,
            image,
            opacity,
        )
    }

    fn draw_svg(
        &mut self,
        x_px: i32,
        y_px: i32,
        width_px: i32,
        height_px: i32,
        svg_xml: &str,
        opacity: u8,
    ) -> Result<(), String> {
        let (x_device_px, width_device_px) = self.scale.css_span_to_device_px(x_px, width_px);
        let (y_device_px, height_device_px) = self.scale.css_span_to_device_px(y_px, height_px);
        self.inner.draw_svg(
            x_device_px,
            y_device_px,
            width_device_px,
            height_device_px,
            svg_xml,
            opacity,
        )
    }

    fn flush(&mut self) -> Result<(), String> {
        self.inner.flush()
    }
}
