#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_uint, c_void};

#[repr(C)]
pub struct wl_proxy {
    _private: [u8; 0],
}

pub type wl_display = wl_proxy;
pub type wl_registry = wl_proxy;
pub type wl_compositor = wl_proxy;
pub type wl_surface = wl_proxy;
pub type wl_shm = wl_proxy;
pub type wl_shm_pool = wl_proxy;
pub type wl_buffer = wl_proxy;
pub type wl_seat = wl_proxy;
pub type wl_pointer = wl_proxy;
pub type xdg_wm_base = wl_proxy;
pub type xdg_surface = wl_proxy;
pub type xdg_toplevel = wl_proxy;

pub type wl_fixed_t = i32;

#[repr(C)]
pub struct wl_array {
    pub size: usize,
    pub alloc: usize,
    pub data: *mut c_void,
}

#[repr(C)]
pub struct wl_registry_listener {
    pub global: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            registry: *mut wl_registry,
            name: u32,
            interface: *const c_char,
            version: u32,
        ),
    >,
    pub global_remove:
        Option<unsafe extern "C" fn(data: *mut c_void, registry: *mut wl_registry, name: u32)>,
}

#[repr(C)]
pub struct wl_shm_listener {
    pub format: Option<unsafe extern "C" fn(data: *mut c_void, shm: *mut wl_shm, format: u32)>,
}

#[repr(C)]
pub struct wl_buffer_listener {
    pub release: Option<unsafe extern "C" fn(data: *mut c_void, buffer: *mut wl_buffer)>,
}

#[repr(C)]
pub struct wl_seat_listener {
    pub capabilities:
        Option<unsafe extern "C" fn(data: *mut c_void, seat: *mut wl_seat, capabilities: u32)>,
    pub name:
        Option<unsafe extern "C" fn(data: *mut c_void, seat: *mut wl_seat, name: *const c_char)>,
}

#[repr(C)]
pub struct wl_pointer_listener {
    pub enter: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            pointer: *mut wl_pointer,
            serial: u32,
            surface: *mut wl_surface,
            surface_x: wl_fixed_t,
            surface_y: wl_fixed_t,
        ),
    >,
    pub leave: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            pointer: *mut wl_pointer,
            serial: u32,
            surface: *mut wl_surface,
        ),
    >,
    pub motion: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            pointer: *mut wl_pointer,
            time: u32,
            surface_x: wl_fixed_t,
            surface_y: wl_fixed_t,
        ),
    >,
    pub button: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            pointer: *mut wl_pointer,
            serial: u32,
            time: u32,
            button: u32,
            state: u32,
        ),
    >,
    pub axis: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            pointer: *mut wl_pointer,
            time: u32,
            axis: u32,
            value: wl_fixed_t,
        ),
    >,
    pub frame: Option<unsafe extern "C" fn(data: *mut c_void, pointer: *mut wl_pointer)>,
    pub axis_source:
        Option<unsafe extern "C" fn(data: *mut c_void, pointer: *mut wl_pointer, axis_source: u32)>,
    pub axis_stop: Option<
        unsafe extern "C" fn(data: *mut c_void, pointer: *mut wl_pointer, time: u32, axis: u32),
    >,
    pub axis_discrete: Option<
        unsafe extern "C" fn(data: *mut c_void, pointer: *mut wl_pointer, axis: u32, discrete: i32),
    >,
    pub axis_value120: Option<
        unsafe extern "C" fn(data: *mut c_void, pointer: *mut wl_pointer, axis: u32, value120: i32),
    >,
    pub axis_relative_direction: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            pointer: *mut wl_pointer,
            axis: u32,
            direction: u32,
        ),
    >,
}

#[repr(C)]
pub struct xdg_wm_base_listener {
    pub ping:
        Option<unsafe extern "C" fn(data: *mut c_void, wm_base: *mut xdg_wm_base, serial: u32)>,
}

#[repr(C)]
pub struct xdg_surface_listener {
    pub configure:
        Option<unsafe extern "C" fn(data: *mut c_void, surface: *mut xdg_surface, serial: u32)>,
}

#[repr(C)]
pub struct xdg_toplevel_listener {
    pub configure: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            toplevel: *mut xdg_toplevel,
            width: i32,
            height: i32,
            states: *mut wl_array,
        ),
    >,
    pub close: Option<unsafe extern "C" fn(data: *mut c_void, toplevel: *mut xdg_toplevel)>,
    pub configure_bounds: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            toplevel: *mut xdg_toplevel,
            width: i32,
            height: i32,
        ),
    >,
    pub wm_capabilities: Option<
        unsafe extern "C" fn(
            data: *mut c_void,
            toplevel: *mut xdg_toplevel,
            capabilities: *mut wl_array,
        ),
    >,
}

pub const WL_SHM_FORMAT_ARGB8888: u32 = 0;

pub const WL_SEAT_CAPABILITY_POINTER: u32 = 1;

pub const WL_POINTER_BUTTON_STATE_PRESSED: u32 = 1;
pub const WL_POINTER_AXIS_VERTICAL_SCROLL: u32 = 0;

pub const BTN_LEFT: u32 = 0x110;
pub const BTN_SIDE: u32 = 0x113;

#[link(name = "wayland-client")]
unsafe extern "C" {
    pub fn wl_display_connect(name: *const c_char) -> *mut wl_display;
    pub fn wl_display_disconnect(display: *mut wl_display);
    pub fn wl_display_roundtrip(display: *mut wl_display) -> c_int;
    pub fn wl_display_dispatch(display: *mut wl_display) -> c_int;
    pub fn wl_display_dispatch_pending(display: *mut wl_display) -> c_int;
    pub fn wl_display_flush(display: *mut wl_display) -> c_int;
    pub fn wl_display_get_fd(display: *mut wl_display) -> c_int;
    pub fn wl_display_get_error(display: *mut wl_display) -> c_int;

    pub fn wl_proxy_add_listener(
        proxy: *mut wl_proxy,
        implementation: *mut Option<unsafe extern "C" fn()>,
        data: *mut c_void,
    ) -> c_int;
    pub fn wl_proxy_destroy(proxy: *mut wl_proxy);
}

unsafe extern "C" {
    pub fn oab_wl_display_get_registry(display: *mut wl_display) -> *mut wl_registry;

    pub fn oab_wl_registry_bind_compositor(
        registry: *mut wl_registry,
        name: c_uint,
        version: c_uint,
    ) -> *mut wl_compositor;
    pub fn oab_wl_registry_bind_shm(
        registry: *mut wl_registry,
        name: c_uint,
        version: c_uint,
    ) -> *mut wl_shm;
    pub fn oab_wl_registry_bind_seat(
        registry: *mut wl_registry,
        name: c_uint,
        version: c_uint,
    ) -> *mut wl_seat;
    pub fn oab_wl_registry_bind_xdg_wm_base(
        registry: *mut wl_registry,
        name: c_uint,
        version: c_uint,
    ) -> *mut xdg_wm_base;

    pub fn oab_wl_compositor_create_surface(compositor: *mut wl_compositor) -> *mut wl_surface;

    pub fn oab_wl_shm_create_pool(shm: *mut wl_shm, fd: c_int, size: c_int) -> *mut wl_shm_pool;
    pub fn oab_wl_shm_pool_create_buffer(
        pool: *mut wl_shm_pool,
        offset: c_int,
        width: c_int,
        height: c_int,
        stride: c_int,
        format: c_uint,
    ) -> *mut wl_buffer;
    pub fn oab_wl_shm_pool_destroy(pool: *mut wl_shm_pool);
    pub fn oab_wl_buffer_destroy(buffer: *mut wl_buffer);

    pub fn oab_wl_surface_attach(
        surface: *mut wl_surface,
        buffer: *mut wl_buffer,
        x: c_int,
        y: c_int,
    );
    pub fn oab_wl_surface_damage_buffer(
        surface: *mut wl_surface,
        x: c_int,
        y: c_int,
        width: c_int,
        height: c_int,
    );
    pub fn oab_wl_surface_set_buffer_scale(surface: *mut wl_surface, scale: c_int);
    pub fn oab_wl_surface_commit(surface: *mut wl_surface);
    pub fn oab_wl_surface_destroy(surface: *mut wl_surface);

    pub fn oab_wl_seat_get_pointer(seat: *mut wl_seat) -> *mut wl_pointer;
    pub fn oab_wl_shm_release(shm: *mut wl_shm);

    pub fn oab_xdg_wm_base_get_xdg_surface(
        wm_base: *mut xdg_wm_base,
        surface: *mut wl_surface,
    ) -> *mut xdg_surface;
    pub fn oab_xdg_wm_base_pong(wm_base: *mut xdg_wm_base, serial: c_uint);
    pub fn oab_xdg_wm_base_destroy(wm_base: *mut xdg_wm_base);

    pub fn oab_xdg_surface_get_toplevel(surface: *mut xdg_surface) -> *mut xdg_toplevel;
    pub fn oab_xdg_surface_ack_configure(surface: *mut xdg_surface, serial: c_uint);
    pub fn oab_xdg_surface_destroy(surface: *mut xdg_surface);

    pub fn oab_xdg_toplevel_set_title(toplevel: *mut xdg_toplevel, title: *const c_char);
    pub fn oab_xdg_toplevel_set_app_id(toplevel: *mut xdg_toplevel, app_id: *const c_char);
    pub fn oab_xdg_toplevel_destroy(toplevel: *mut xdg_toplevel);
}

#[inline]
pub fn wl_fixed_to_f64(value: wl_fixed_t) -> f64 {
    f64::from(value) / 256.0
}
