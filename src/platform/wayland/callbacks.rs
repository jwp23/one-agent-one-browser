use super::sys::*;
use core::ffi::{c_char, c_void};
use std::ffi::CStr;

const WHEEL_SCROLL_STEP_PX: i32 = 48;

pub(super) struct CallbackState {
    pub(super) setup_error: Option<String>,

    pub(super) compositor: *mut wl_compositor,
    pub(super) shm: *mut wl_shm,
    pub(super) seat: *mut wl_seat,
    pub(super) pointer: *mut wl_pointer,
    pub(super) wm_base: *mut xdg_wm_base,

    pub(super) supports_argb8888: bool,
    pub(super) configured: bool,
    pub(super) pending_resize: Option<(i32, i32)>,
    pub(super) should_exit: bool,

    pub(super) pointer_x_css_px: i32,
    pub(super) pointer_y_css_px: i32,
    pub(super) pending_mouse_downs: u32,
    pub(super) pending_back_navigations: u32,
    pub(super) pending_wheel_css_px: i32,

    pub(super) buffer_ptr: *mut wl_buffer,
    pub(super) buffer_busy: bool,
}

impl Default for CallbackState {
    fn default() -> Self {
        Self {
            setup_error: None,
            compositor: std::ptr::null_mut(),
            shm: std::ptr::null_mut(),
            seat: std::ptr::null_mut(),
            pointer: std::ptr::null_mut(),
            wm_base: std::ptr::null_mut(),
            supports_argb8888: false,
            configured: false,
            pending_resize: None,
            should_exit: false,
            pointer_x_css_px: 0,
            pointer_y_css_px: 0,
            pending_mouse_downs: 0,
            pending_back_navigations: 0,
            pending_wheel_css_px: 0,
            buffer_ptr: std::ptr::null_mut(),
            buffer_busy: false,
        }
    }
}

pub(super) fn take_setup_error(state: &mut CallbackState) -> Option<String> {
    state.setup_error.take()
}

fn record_setup_error(state: &mut CallbackState, message: String) {
    if state.setup_error.is_none() {
        state.setup_error = Some(message);
    }
}

unsafe fn state_from_data<'a>(data: *mut c_void) -> &'a mut CallbackState {
    unsafe { &mut *data.cast::<CallbackState>() }
}

fn fixed_to_i32(value: wl_fixed_t) -> i32 {
    wl_fixed_to_f64(value)
        .round()
        .clamp(f64::from(i32::MIN), f64::from(i32::MAX)) as i32
}

pub(super) unsafe fn add_proxy_listener<P, L>(
    proxy: *mut P,
    listener: &L,
    data: *mut CallbackState,
    name: &str,
) -> Result<(), String> {
    let rc = unsafe {
        wl_proxy_add_listener(
            proxy.cast::<wl_proxy>(),
            (listener as *const L)
                .cast::<Option<unsafe extern "C" fn()>>()
                .cast_mut(),
            data.cast::<c_void>(),
        )
    };
    if rc != 0 {
        return Err(format!(
            "{name}: wl_proxy_add_listener failed with code {rc}"
        ));
    }
    Ok(())
}

pub(super) const REGISTRY_LISTENER: wl_registry_listener = wl_registry_listener {
    global: Some(handle_registry_global),
    global_remove: Some(handle_registry_global_remove),
};

const SHM_LISTENER: wl_shm_listener = wl_shm_listener {
    format: Some(handle_shm_format),
};

const SEAT_LISTENER: wl_seat_listener = wl_seat_listener {
    capabilities: Some(handle_seat_capabilities),
    name: Some(handle_seat_name),
};

const POINTER_LISTENER: wl_pointer_listener = wl_pointer_listener {
    enter: Some(handle_pointer_enter),
    leave: Some(handle_pointer_leave),
    motion: Some(handle_pointer_motion),
    button: Some(handle_pointer_button),
    axis: Some(handle_pointer_axis),
    frame: Some(handle_pointer_frame),
    axis_source: Some(handle_pointer_axis_source),
    axis_stop: Some(handle_pointer_axis_stop),
    axis_discrete: Some(handle_pointer_axis_discrete),
    axis_value120: Some(handle_pointer_axis_value120),
    axis_relative_direction: Some(handle_pointer_axis_relative_direction),
};

const WM_BASE_LISTENER: xdg_wm_base_listener = xdg_wm_base_listener {
    ping: Some(handle_wm_base_ping),
};

pub(super) const XDG_SURFACE_LISTENER: xdg_surface_listener = xdg_surface_listener {
    configure: Some(handle_xdg_surface_configure),
};

pub(super) const XDG_TOPLEVEL_LISTENER: xdg_toplevel_listener = xdg_toplevel_listener {
    configure: Some(handle_xdg_toplevel_configure),
    close: Some(handle_xdg_toplevel_close),
    configure_bounds: Some(handle_xdg_toplevel_configure_bounds),
    wm_capabilities: Some(handle_xdg_toplevel_wm_capabilities),
};

pub(super) const WL_BUFFER_LISTENER: wl_buffer_listener = wl_buffer_listener {
    release: Some(handle_buffer_release),
};

unsafe extern "C" fn handle_registry_global(
    data: *mut c_void,
    registry: *mut wl_registry,
    name: u32,
    interface: *const c_char,
    version: u32,
) {
    let state = unsafe { state_from_data(data) };

    if interface.is_null() {
        return;
    }

    let interface_name = unsafe { CStr::from_ptr(interface) }.to_bytes();

    if interface_name == b"wl_compositor" && state.compositor.is_null() {
        state.compositor =
            unsafe { oab_wl_registry_bind_compositor(registry, name, version.min(4)) };
        if state.compositor.is_null() {
            record_setup_error(
                state,
                "wl_registry_bind wl_compositor returned null".to_owned(),
            );
        }
        return;
    }

    if interface_name == b"wl_shm" && state.shm.is_null() {
        state.shm = unsafe { oab_wl_registry_bind_shm(registry, name, version.min(1)) };
        if state.shm.is_null() {
            record_setup_error(state, "wl_registry_bind wl_shm returned null".to_owned());
            return;
        }

        let add_result = unsafe { add_proxy_listener(state.shm, &SHM_LISTENER, state, "wl_shm") };
        if let Err(err) = add_result {
            record_setup_error(state, err);
        }
        return;
    }

    if interface_name == b"wl_seat" && state.seat.is_null() {
        state.seat = unsafe { oab_wl_registry_bind_seat(registry, name, version.min(7)) };
        if state.seat.is_null() {
            record_setup_error(state, "wl_registry_bind wl_seat returned null".to_owned());
            return;
        }

        let add_result =
            unsafe { add_proxy_listener(state.seat, &SEAT_LISTENER, state, "wl_seat") };
        if let Err(err) = add_result {
            record_setup_error(state, err);
        }
        return;
    }

    if interface_name == b"xdg_wm_base" && state.wm_base.is_null() {
        state.wm_base = unsafe { oab_wl_registry_bind_xdg_wm_base(registry, name, version.min(6)) };
        if state.wm_base.is_null() {
            record_setup_error(
                state,
                "wl_registry_bind xdg_wm_base returned null".to_owned(),
            );
            return;
        }

        let add_result =
            unsafe { add_proxy_listener(state.wm_base, &WM_BASE_LISTENER, state, "xdg_wm_base") };
        if let Err(err) = add_result {
            record_setup_error(state, err);
        }
    }
}

unsafe extern "C" fn handle_registry_global_remove(
    _data: *mut c_void,
    _registry: *mut wl_registry,
    _name: u32,
) {
}

unsafe extern "C" fn handle_shm_format(data: *mut c_void, _shm: *mut wl_shm, format: u32) {
    let state = unsafe { state_from_data(data) };
    if format == WL_SHM_FORMAT_ARGB8888 {
        state.supports_argb8888 = true;
    }
}

unsafe extern "C" fn handle_seat_capabilities(
    data: *mut c_void,
    seat: *mut wl_seat,
    capabilities: u32,
) {
    let state = unsafe { state_from_data(data) };

    if (capabilities & WL_SEAT_CAPABILITY_POINTER) != 0 {
        if state.pointer.is_null() {
            let pointer = unsafe { oab_wl_seat_get_pointer(seat) };
            if pointer.is_null() {
                record_setup_error(state, "wl_seat_get_pointer returned null".to_owned());
                return;
            }

            let add_result =
                unsafe { add_proxy_listener(pointer, &POINTER_LISTENER, state, "wl_pointer") };
            if let Err(err) = add_result {
                unsafe {
                    wl_proxy_destroy(pointer.cast::<wl_proxy>());
                }
                record_setup_error(state, err);
                return;
            }

            state.pointer = pointer;
        }
    } else if !state.pointer.is_null() {
        unsafe {
            wl_proxy_destroy(state.pointer.cast::<wl_proxy>());
        }
        state.pointer = std::ptr::null_mut();
    }
}

unsafe extern "C" fn handle_seat_name(
    _data: *mut c_void,
    _seat: *mut wl_seat,
    _name: *const c_char,
) {
}

unsafe extern "C" fn handle_pointer_enter(
    data: *mut c_void,
    _pointer: *mut wl_pointer,
    _serial: u32,
    _surface: *mut wl_surface,
    surface_x: wl_fixed_t,
    surface_y: wl_fixed_t,
) {
    let state = unsafe { state_from_data(data) };
    state.pointer_x_css_px = fixed_to_i32(surface_x);
    state.pointer_y_css_px = fixed_to_i32(surface_y);
}

unsafe extern "C" fn handle_pointer_leave(
    _data: *mut c_void,
    _pointer: *mut wl_pointer,
    _serial: u32,
    _surface: *mut wl_surface,
) {
}

unsafe extern "C" fn handle_pointer_motion(
    data: *mut c_void,
    _pointer: *mut wl_pointer,
    _time: u32,
    surface_x: wl_fixed_t,
    surface_y: wl_fixed_t,
) {
    let state = unsafe { state_from_data(data) };
    state.pointer_x_css_px = fixed_to_i32(surface_x);
    state.pointer_y_css_px = fixed_to_i32(surface_y);
}

unsafe extern "C" fn handle_pointer_button(
    data: *mut c_void,
    _pointer: *mut wl_pointer,
    _serial: u32,
    _time: u32,
    button: u32,
    state_value: u32,
) {
    if state_value != WL_POINTER_BUTTON_STATE_PRESSED {
        return;
    }

    let state = unsafe { state_from_data(data) };
    if button == BTN_LEFT {
        state.pending_mouse_downs = state.pending_mouse_downs.saturating_add(1);
    } else if button == BTN_SIDE {
        state.pending_back_navigations = state.pending_back_navigations.saturating_add(1);
    }
}

unsafe extern "C" fn handle_pointer_axis(
    data: *mut c_void,
    _pointer: *mut wl_pointer,
    _time: u32,
    axis: u32,
    value: wl_fixed_t,
) {
    if axis != WL_POINTER_AXIS_VERTICAL_SCROLL {
        return;
    }

    let mut delta = fixed_to_i32(value);
    if delta == 0 {
        let sign = wl_fixed_to_f64(value).signum() as i32;
        if sign != 0 {
            delta = sign.saturating_mul(WHEEL_SCROLL_STEP_PX);
        }
    }

    if delta == 0 {
        return;
    }

    let state = unsafe { state_from_data(data) };
    state.pending_wheel_css_px = state.pending_wheel_css_px.saturating_add(delta);
}

unsafe extern "C" fn handle_pointer_frame(_data: *mut c_void, _pointer: *mut wl_pointer) {}

unsafe extern "C" fn handle_pointer_axis_source(
    _data: *mut c_void,
    _pointer: *mut wl_pointer,
    _axis_source: u32,
) {
}

unsafe extern "C" fn handle_pointer_axis_stop(
    _data: *mut c_void,
    _pointer: *mut wl_pointer,
    _time: u32,
    _axis: u32,
) {
}

unsafe extern "C" fn handle_pointer_axis_discrete(
    _data: *mut c_void,
    _pointer: *mut wl_pointer,
    _axis: u32,
    _discrete: i32,
) {
}

unsafe extern "C" fn handle_pointer_axis_value120(
    _data: *mut c_void,
    _pointer: *mut wl_pointer,
    _axis: u32,
    _value120: i32,
) {
}

unsafe extern "C" fn handle_pointer_axis_relative_direction(
    _data: *mut c_void,
    _pointer: *mut wl_pointer,
    _axis: u32,
    _direction: u32,
) {
}

unsafe extern "C" fn handle_wm_base_ping(
    _data: *mut c_void,
    wm_base: *mut xdg_wm_base,
    serial: u32,
) {
    unsafe {
        oab_xdg_wm_base_pong(wm_base, serial);
    }
}

unsafe extern "C" fn handle_xdg_surface_configure(
    data: *mut c_void,
    surface: *mut xdg_surface,
    serial: u32,
) {
    let state = unsafe { state_from_data(data) };
    unsafe {
        oab_xdg_surface_ack_configure(surface, serial);
    }
    state.configured = true;
}

unsafe extern "C" fn handle_xdg_toplevel_configure(
    data: *mut c_void,
    _toplevel: *mut xdg_toplevel,
    width: i32,
    height: i32,
    _states: *mut wl_array,
) {
    let state = unsafe { state_from_data(data) };
    if width > 0 && height > 0 {
        state.pending_resize = Some((width, height));
    }
}

unsafe extern "C" fn handle_xdg_toplevel_close(data: *mut c_void, _toplevel: *mut xdg_toplevel) {
    let state = unsafe { state_from_data(data) };
    state.should_exit = true;
}

unsafe extern "C" fn handle_xdg_toplevel_configure_bounds(
    _data: *mut c_void,
    _toplevel: *mut xdg_toplevel,
    _width: i32,
    _height: i32,
) {
}

unsafe extern "C" fn handle_xdg_toplevel_wm_capabilities(
    _data: *mut c_void,
    _toplevel: *mut xdg_toplevel,
    _capabilities: *mut wl_array,
) {
}

unsafe extern "C" fn handle_buffer_release(data: *mut c_void, buffer: *mut wl_buffer) {
    let state = unsafe { state_from_data(data) };
    if state.buffer_ptr == buffer {
        state.buffer_busy = false;
    }
}
