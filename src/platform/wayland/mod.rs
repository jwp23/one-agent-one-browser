mod cairo;
mod callbacks;
mod painter;
mod scale;
mod scaled;
mod sys;

use super::WindowOptions;
use crate::app::App;
use crate::render::Viewport;
use core::ffi::{c_int, c_void};
use std::ffi::CString;
use std::fs::OpenOptions;
use std::io;
use std::os::fd::{AsRawFd, OwnedFd};
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use callbacks::{
    CallbackState, REGISTRY_LISTENER, WL_BUFFER_LISTENER, XDG_SURFACE_LISTENER,
    XDG_TOPLEVEL_LISTENER, add_proxy_listener, take_setup_error,
};
use painter::WaylandPainter;
use scale::ScaleFactor;
use scaled::ScaledPainter;
use sys::*;

const SCREENSHOT_RESOURCE_WAIT_TIMEOUT: Duration = Duration::from_secs(5);

const POLLIN: i16 = 0x001;
const POLLERR: i16 = 0x008;
const POLLHUP: i16 = 0x010;

const PROT_READ: c_int = 0x1;
const PROT_WRITE: c_int = 0x2;
const MAP_SHARED: c_int = 0x01;

#[repr(C)]
struct PollFd {
    fd: c_int,
    events: i16,
    revents: i16,
}

unsafe extern "C" {
    fn poll(fds: *mut PollFd, nfds: usize, timeout: c_int) -> c_int;

    fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: c_int,
        flags: c_int,
        fd: c_int,
        offset: isize,
    ) -> *mut c_void;
    fn munmap(addr: *mut c_void, len: usize) -> c_int;
}

pub fn run_window<A: App>(title: &str, options: WindowOptions, app: &mut A) -> Result<(), String> {
    let display = unsafe { wl_display_connect(std::ptr::null()) };
    if display.is_null() {
        return Err(
            "wl_display_connect failed: ensure a Wayland compositor is running and $WAYLAND_DISPLAY is set"
                .to_owned(),
        );
    }

    let result = run_window_with_display(display, title, options, app);

    unsafe {
        wl_display_disconnect(display);
    }

    result
}

fn run_window_with_display<A: App>(
    display: *mut wl_display,
    title: &str,
    options: WindowOptions,
    app: &mut A,
) -> Result<(), String> {
    let mut state = Box::new(CallbackState::default());
    let state_ptr: *mut CallbackState = &mut *state;

    let registry = unsafe { oab_wl_display_get_registry(display) };
    if registry.is_null() {
        return Err("wl_display_get_registry returned null".to_owned());
    }

    unsafe {
        add_proxy_listener(registry, &REGISTRY_LISTENER, state_ptr, "wl_registry")?;
    }

    roundtrip(display)?;
    roundtrip(display)?;
    if let Some(err) = take_setup_error(&mut state) {
        return Err(err);
    }

    if state.compositor.is_null() {
        return Err("Wayland compositor does not expose wl_compositor".to_owned());
    }
    if state.shm.is_null() {
        return Err("Wayland compositor does not expose wl_shm".to_owned());
    }
    if state.wm_base.is_null() {
        return Err("Wayland compositor does not expose xdg_wm_base".to_owned());
    }
    if !state.supports_argb8888 {
        return Err("Wayland wl_shm does not advertise WL_SHM_FORMAT_ARGB8888".to_owned());
    }

    let surface = unsafe { oab_wl_compositor_create_surface(state.compositor) };
    if surface.is_null() {
        return Err("wl_compositor_create_surface returned null".to_owned());
    }

    let xdg_surface = unsafe { oab_xdg_wm_base_get_xdg_surface(state.wm_base, surface) };
    if xdg_surface.is_null() {
        unsafe {
            oab_wl_surface_destroy(surface);
        }
        return Err("xdg_wm_base_get_xdg_surface returned null".to_owned());
    }

    let xdg_toplevel = unsafe { oab_xdg_surface_get_toplevel(xdg_surface) };
    if xdg_toplevel.is_null() {
        unsafe {
            oab_xdg_surface_destroy(xdg_surface);
            oab_wl_surface_destroy(surface);
        }
        return Err("xdg_surface_get_toplevel returned null".to_owned());
    }

    let title_cstr =
        CString::new(title).map_err(|_| "Window title contains an embedded NUL byte".to_owned())?;
    let app_id_cstr = CString::new("one-agent-one-browser")
        .map_err(|_| "Internal error constructing app id".to_owned())?;

    unsafe {
        add_proxy_listener(xdg_surface, &XDG_SURFACE_LISTENER, state_ptr, "xdg_surface")?;
        add_proxy_listener(
            xdg_toplevel,
            &XDG_TOPLEVEL_LISTENER,
            state_ptr,
            "xdg_toplevel",
        )?;

        oab_xdg_toplevel_set_title(xdg_toplevel, title_cstr.as_ptr());
        oab_xdg_toplevel_set_app_id(xdg_toplevel, app_id_cstr.as_ptr());
    }

    let detected_scale = ScaleFactor::detect();
    let buffer_scale = detected_scale.scale_int().max(1);
    let scale = ScaleFactor::new((buffer_scale as u32).saturating_mul(1024));

    unsafe {
        oab_wl_surface_set_buffer_scale(surface, buffer_scale);
        oab_wl_surface_commit(surface);
    }

    for _ in 0..4 {
        if state.configured {
            break;
        }
        roundtrip(display)?;
    }
    if !state.configured {
        unsafe {
            oab_xdg_toplevel_destroy(xdg_toplevel);
            oab_xdg_surface_destroy(xdg_surface);
            oab_wl_surface_destroy(surface);
        }
        return Err("Wayland compositor did not send an initial configure event".to_owned());
    }

    let mut css_viewport = Viewport {
        width_px: options.initial_width_px.unwrap_or(1024),
        height_px: options.initial_height_px.unwrap_or(768),
    };
    if css_viewport.width_px <= 0 || css_viewport.height_px <= 0 {
        unsafe {
            oab_xdg_toplevel_destroy(xdg_toplevel);
            oab_xdg_surface_destroy(xdg_surface);
            oab_wl_surface_destroy(surface);
        }
        return Err(format!(
            "Invalid initial window size: {}x{}",
            css_viewport.width_px, css_viewport.height_px
        ));
    }

    if let Some((width_css, height_css)) = state.pending_resize.take()
        && width_css > 0
        && height_css > 0
    {
        css_viewport = Viewport {
            width_px: width_css,
            height_px: height_css,
        };
    }

    let mut viewport = Viewport {
        width_px: scale.css_size_to_device_px(css_viewport.width_px),
        height_px: scale.css_size_to_device_px(css_viewport.height_px),
    };

    let mut painter = WaylandPainter::new(viewport)?;
    let mut shm_buffer: Option<ShmBuffer> = None;

    let mut screenshot_path = options.screenshot_path;
    let headless = options.headless;

    let loop_result = (|| {
        let mut needs_redraw = true;
        let mut has_rendered_ready_state = false;
        let mut resource_wait_started: Option<Instant> = None;

        loop {
            dispatch_events(display, 0)?;

            if state.should_exit {
                break;
            }

            if let Some((width_css, height_css)) = state.pending_resize.take()
                && width_css > 0
                && height_css > 0
            {
                if width_css != css_viewport.width_px || height_css != css_viewport.height_px {
                    css_viewport = Viewport {
                        width_px: width_css,
                        height_px: height_css,
                    };
                    viewport = Viewport {
                        width_px: scale.css_size_to_device_px(width_css),
                        height_px: scale.css_size_to_device_px(height_css),
                    };
                    needs_redraw = true;
                    has_rendered_ready_state = false;
                    resource_wait_started = None;
                }
            }

            consume_input_events(app, &mut state, css_viewport, &mut needs_redraw)?;

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
                let rgb = painter.capture_back_buffer_rgb()?;
                crate::png::write_rgb_png(&path, &rgb)?;
                break;
            }

            let can_present = state.configured && !state.buffer_busy;
            if needs_redraw && can_present {
                painter.ensure_back_buffer(viewport)?;
                let mut scaled_painter = ScaledPainter::new(&mut painter, scale);
                app.render(&mut scaled_painter, css_viewport)?;
                needs_redraw = false;

                let shm = state.shm;
                ensure_shm_buffer(
                    &mut shm_buffer,
                    &mut state,
                    state_ptr,
                    shm,
                    viewport.width_px,
                    viewport.height_px,
                )?;

                let buffer = shm_buffer
                    .as_mut()
                    .ok_or_else(|| "Internal error: shared-memory buffer missing".to_owned())?;
                copy_bgra_to_shm(buffer, painter.bgra())?;

                unsafe {
                    oab_wl_surface_set_buffer_scale(surface, buffer_scale);
                    oab_wl_surface_attach(surface, buffer.buffer, 0, 0);
                    oab_wl_surface_damage_buffer(
                        surface,
                        0,
                        0,
                        viewport.width_px,
                        viewport.height_px,
                    );
                    oab_wl_surface_commit(surface);
                }
                state.buffer_busy = true;

                flush_display(display)?;

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
                dispatch_events(display, 10)?;
            }
        }

        Ok(())
    })();

    drop(shm_buffer);

    unsafe {
        if !state.pointer.is_null() {
            wl_proxy_destroy(state.pointer.cast::<wl_proxy>());
            state.pointer = std::ptr::null_mut();
        }
        if !state.seat.is_null() {
            wl_proxy_destroy(state.seat.cast::<wl_proxy>());
            state.seat = std::ptr::null_mut();
        }

        oab_xdg_toplevel_destroy(xdg_toplevel);
        oab_xdg_surface_destroy(xdg_surface);
        oab_wl_surface_destroy(surface);

        if !state.wm_base.is_null() {
            oab_xdg_wm_base_destroy(state.wm_base);
            state.wm_base = std::ptr::null_mut();
        }
        if !state.shm.is_null() {
            oab_wl_shm_release(state.shm);
            state.shm = std::ptr::null_mut();
        }
        if !state.compositor.is_null() {
            wl_proxy_destroy(state.compositor.cast::<wl_proxy>());
            state.compositor = std::ptr::null_mut();
        }
        wl_proxy_destroy(registry.cast::<wl_proxy>());
    }

    loop_result
}

fn consume_input_events<A: App>(
    app: &mut A,
    state: &mut CallbackState,
    css_viewport: Viewport,
    needs_redraw: &mut bool,
) -> Result<(), String> {
    let mouse_downs = std::mem::take(&mut state.pending_mouse_downs);
    for _ in 0..mouse_downs {
        let tick = app.mouse_down(state.pointer_x_css_px, state.pointer_y_css_px, css_viewport)?;
        if tick.needs_redraw {
            *needs_redraw = true;
        }
    }

    let back_navigations = std::mem::take(&mut state.pending_back_navigations);
    for _ in 0..back_navigations {
        let tick = app.navigate_back()?;
        if tick.needs_redraw {
            *needs_redraw = true;
        }
    }

    let wheel_delta = std::mem::take(&mut state.pending_wheel_css_px);
    if wheel_delta != 0 {
        let tick = app.mouse_wheel(wheel_delta, css_viewport)?;
        if tick.needs_redraw {
            *needs_redraw = true;
        }
    }

    Ok(())
}

fn ensure_shm_buffer(
    slot: &mut Option<ShmBuffer>,
    state: &mut CallbackState,
    state_ptr: *mut CallbackState,
    shm: *mut wl_shm,
    width_px: i32,
    height_px: i32,
) -> Result<(), String> {
    if width_px <= 0 || height_px <= 0 {
        return Err(format!(
            "Invalid Wayland buffer size: {}x{}",
            width_px, height_px
        ));
    }

    let needs_recreate = slot
        .as_ref()
        .is_none_or(|buffer| buffer.width_px != width_px || buffer.height_px != height_px);

    if needs_recreate {
        if let Some(old) = slot.take()
            && state.buffer_ptr == old.buffer
        {
            state.buffer_ptr = std::ptr::null_mut();
            state.buffer_busy = false;
        }

        let mut buffer = ShmBuffer::new(shm, width_px, height_px)?;

        unsafe {
            add_proxy_listener(buffer.buffer, &WL_BUFFER_LISTENER, state_ptr, "wl_buffer")?;
        }

        state.buffer_ptr = buffer.buffer;
        state.buffer_busy = false;
        buffer.clear();

        *slot = Some(buffer);
    }

    Ok(())
}

fn copy_bgra_to_shm(buffer: &mut ShmBuffer, bgra: &[u8]) -> Result<(), String> {
    if bgra.len() != buffer.len {
        return Err(format!(
            "Wayland shared-memory buffer length mismatch: expected {}, got {}",
            buffer.len,
            bgra.len()
        ));
    }

    unsafe {
        std::ptr::copy_nonoverlapping(bgra.as_ptr(), buffer.data_ptr, bgra.len());
    }

    Ok(())
}

fn roundtrip(display: *mut wl_display) -> Result<(), String> {
    let rc = unsafe { wl_display_roundtrip(display) };
    if rc < 0 {
        return Err(wayland_display_error("wl_display_roundtrip", display, None));
    }
    Ok(())
}

fn dispatch_events(display: *mut wl_display, timeout_ms: i32) -> Result<(), String> {
    let pending_rc = unsafe { wl_display_dispatch_pending(display) };
    if pending_rc < 0 {
        return Err(wayland_display_error(
            "wl_display_dispatch_pending",
            display,
            None,
        ));
    }

    flush_display(display)?;

    let fd = unsafe { wl_display_get_fd(display) };
    if fd < 0 {
        return Err("wl_display_get_fd returned an invalid fd".to_owned());
    }

    let mut pollfd = PollFd {
        fd,
        events: POLLIN,
        revents: 0,
    };

    let timeout_ms = timeout_ms.max(0);
    let poll_rc = unsafe { poll(&mut pollfd, 1, timeout_ms) };
    if poll_rc < 0 {
        let err = io::Error::last_os_error();
        if err.kind() == io::ErrorKind::Interrupted {
            return Ok(());
        }
        return Err(format!("poll on Wayland display fd failed: {err}"));
    }

    if poll_rc == 0 {
        return Ok(());
    }

    if (pollfd.revents & (POLLERR | POLLHUP)) != 0 {
        return Err(wayland_display_error(
            "poll",
            display,
            Some(format!("revents=0x{:x}", pollfd.revents)),
        ));
    }

    if (pollfd.revents & POLLIN) != 0 {
        let rc = unsafe { wl_display_dispatch(display) };
        if rc < 0 {
            return Err(wayland_display_error("wl_display_dispatch", display, None));
        }
    }

    Ok(())
}

fn flush_display(display: *mut wl_display) -> Result<(), String> {
    let rc = unsafe { wl_display_flush(display) };
    if rc >= 0 {
        return Ok(());
    }

    let display_error = unsafe { wl_display_get_error(display) };
    if display_error == 0 {
        return Ok(());
    }

    Err(wayland_display_error("wl_display_flush", display, None))
}

fn wayland_display_error(step: &str, display: *mut wl_display, extra: Option<String>) -> String {
    let err = unsafe { wl_display_get_error(display) };
    let mut message = if err != 0 {
        format!("{step} failed: {}", io::Error::from_raw_os_error(err))
    } else {
        format!("{step} failed")
    };
    if let Some(extra) = extra {
        message.push_str(" (");
        message.push_str(&extra);
        message.push(')');
    }
    message
}

fn create_shared_memory_file(size_bytes: usize) -> Result<OwnedFd, String> {
    if size_bytes == 0 {
        return Err("Shared-memory file size must be > 0".to_owned());
    }

    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
        .unwrap_or_else(std::env::temp_dir);

    let pid = std::process::id();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);

    for attempt in 0u32..256 {
        let file_path = dir.join(format!("oab-shm-{pid}-{nonce}-{attempt}"));
        let open_result = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&file_path);

        let file = match open_result {
            Ok(file) => file,
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => {
                return Err(format!(
                    "Failed to create Wayland shared-memory file {}: {err}",
                    file_path.display()
                ));
            }
        };

        if let Err(err) = file.set_len(size_bytes as u64) {
            let _ = std::fs::remove_file(&file_path);
            return Err(format!(
                "Failed to size Wayland shared-memory file {}: {err}",
                file_path.display()
            ));
        }

        let _ = std::fs::remove_file(&file_path);

        let owned_fd: OwnedFd = file.into();
        return Ok(owned_fd);
    }

    Err(format!(
        "Failed to allocate a unique Wayland shared-memory file in {}",
        dir.display()
    ))
}

fn is_map_failed(ptr: *mut c_void) -> bool {
    ptr == (-1isize as *mut c_void)
}

struct ShmBuffer {
    buffer: *mut wl_buffer,
    data_ptr: *mut u8,
    len: usize,
    width_px: i32,
    height_px: i32,
    _fd: OwnedFd,
}

impl ShmBuffer {
    fn new(shm: *mut wl_shm, width_px: i32, height_px: i32) -> Result<Self, String> {
        if shm.is_null() {
            return Err("Wayland wl_shm is null".to_owned());
        }
        if width_px <= 0 || height_px <= 0 {
            return Err(format!(
                "Invalid Wayland buffer size: {}x{}",
                width_px, height_px
            ));
        }

        let stride = width_px
            .checked_mul(4)
            .ok_or_else(|| "Wayland buffer stride overflow".to_owned())?;
        let len = (height_px as usize)
            .checked_mul(stride as usize)
            .ok_or_else(|| "Wayland buffer size overflow".to_owned())?;

        let fd = create_shared_memory_file(len)?;

        let mapped = unsafe {
            mmap(
                std::ptr::null_mut(),
                len,
                PROT_READ | PROT_WRITE,
                MAP_SHARED,
                fd.as_raw_fd(),
                0,
            )
        };
        if is_map_failed(mapped) {
            return Err(format!(
                "mmap failed for Wayland shared-memory buffer: {}",
                io::Error::last_os_error()
            ));
        }

        let pool = unsafe { oab_wl_shm_create_pool(shm, fd.as_raw_fd(), len as c_int) };
        if pool.is_null() {
            unsafe {
                munmap(mapped, len);
            }
            return Err("wl_shm_create_pool returned null".to_owned());
        }

        let buffer = unsafe {
            oab_wl_shm_pool_create_buffer(
                pool,
                0,
                width_px,
                height_px,
                stride,
                WL_SHM_FORMAT_ARGB8888,
            )
        };

        unsafe {
            oab_wl_shm_pool_destroy(pool);
        }

        if buffer.is_null() {
            unsafe {
                munmap(mapped, len);
            }
            return Err("wl_shm_pool_create_buffer returned null".to_owned());
        }

        Ok(Self {
            buffer,
            data_ptr: mapped.cast::<u8>(),
            len,
            width_px,
            height_px,
            _fd: fd,
        })
    }

    fn clear(&mut self) {
        unsafe {
            std::ptr::write_bytes(self.data_ptr, 0, self.len);
        }
    }
}

impl Drop for ShmBuffer {
    fn drop(&mut self) {
        if !self.buffer.is_null() {
            unsafe {
                oab_wl_buffer_destroy(self.buffer);
            }
            self.buffer = std::ptr::null_mut();
        }

        if !self.data_ptr.is_null() {
            unsafe {
                munmap(self.data_ptr.cast::<c_void>(), self.len);
            }
            self.data_ptr = std::ptr::null_mut();
        }
    }
}
