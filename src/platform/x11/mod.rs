mod painter;
mod xft;
mod xlib;

use super::WindowOptions;
use crate::app::App;
use crate::render::Viewport;
use core::ffi::{c_int, c_uint, c_ulong};
use std::ffi::CString;
use std::time::Duration;

use painter::X11Painter;
use xlib::*;

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
) -> Result<(), String>
{
    let screen = unsafe { XDefaultScreen(display) };
    let visual = unsafe { XDefaultVisual(display, screen) };
    if visual.is_null() {
        return Err("XDefaultVisual returned null".to_owned());
    }
    let visual_masks = unsafe { ((*visual).red_mask, (*visual).green_mask, (*visual).blue_mask) };
    let colormap = unsafe { XDefaultColormap(display, screen) };
    let root_window = unsafe { XRootWindow(display, screen) };
    let black_pixel = unsafe { XBlackPixel(display, screen) };
    let white_pixel = unsafe { XWhitePixel(display, screen) };

    let initial_width: c_uint = 1024;
    let initial_height: c_uint = 768;

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

    let depth_i32 = unsafe { XDefaultDepth(display, screen) };
    let depth: c_uint = depth_i32
        .try_into()
        .map_err(|_| format!("XDefaultDepth returned an invalid value: {depth_i32}"))?;

    let gc = unsafe { XDefaultGC(display, screen) };
    unsafe {
        XSetForeground(display, gc, black_pixel);
        XSetBackground(display, gc, white_pixel);
    }

    let back_buffer = unsafe { XCreatePixmap(display, window, initial_width, initial_height, depth) };
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
        width_px: initial_width as i32,
        height_px: initial_height as i32,
    };

    let mut screenshot_path = options.screenshot_path;

    let loop_result = (|| {
        let mut needs_redraw = true;
        let mut should_exit = false;

        loop {
            while unsafe { XPending(display) } > 0 {
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
                        needs_redraw = true;
                    }
                    EVENT_TYPE_BUTTON_PRESS => {
                        let button: &XButtonEvent =
                            unsafe { &*(event.inner.as_ptr() as *const XButtonEvent) };
                        if button.button == 1 {
                            let tick = app.mouse_down(button.x, button.y, viewport)?;
                            if tick.needs_redraw {
                                needs_redraw = true;
                            }
                        }
                    }
                    EVENT_TYPE_KEY_PRESS => {
                        should_exit = true;
                        break;
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
            }

            if should_exit {
                break;
            }

            let tick = app.tick()?;
            if tick.needs_redraw {
                needs_redraw = true;
            }
            let ready_for_screenshot = tick.ready_for_screenshot;
            if screenshot_path.is_some() && ready_for_screenshot {
                needs_redraw = true;
            }

            if needs_redraw {
                painter.ensure_back_buffer(viewport)?;
                app.render(&mut painter, viewport)?;
                needs_redraw = false;

                if ready_for_screenshot {
                    if let Some(path) = screenshot_path.take() {
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
