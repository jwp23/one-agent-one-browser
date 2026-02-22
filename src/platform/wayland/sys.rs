#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_uint, c_void};
#[repr(C)]
pub struct wl_proxy {
    _private: [u8; 0],
}

#[repr(C)]
pub struct wl_interface {
    pub name: *const c_char,
    pub version: c_int,
    pub method_count: c_int,
    pub methods: *const wl_message,
    pub event_count: c_int,
    pub events: *const wl_message,
}

#[repr(C)]
pub struct wl_message {
    pub name: *const c_char,
    pub signature: *const c_char,
    pub types: *const *const wl_interface,
}

unsafe impl Sync for wl_interface {}
unsafe impl Sync for wl_message {}

#[repr(transparent)]
struct InterfaceTypeList<const N: usize>([*const wl_interface; N]);

unsafe impl<const N: usize> Sync for InterfaceTypeList<N> {}

impl<const N: usize> InterfaceTypeList<N> {
    const fn as_ptr(&self) -> *const *const wl_interface {
        self.0.as_ptr()
    }
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

const WL_MARSHAL_FLAG_DESTROY: c_uint = 1 << 0;

const WL_DISPLAY_GET_REGISTRY: c_uint = 1;
const WL_REGISTRY_BIND: c_uint = 0;
const WL_COMPOSITOR_CREATE_SURFACE: c_uint = 0;
const WL_SHM_POOL_CREATE_BUFFER: c_uint = 0;
const WL_SHM_POOL_DESTROY: c_uint = 1;
const WL_SHM_CREATE_POOL: c_uint = 0;
const WL_SHM_RELEASE: c_uint = 1;
const WL_BUFFER_DESTROY: c_uint = 0;
const WL_SURFACE_DESTROY: c_uint = 0;
const WL_SURFACE_ATTACH: c_uint = 1;
const WL_SURFACE_COMMIT: c_uint = 6;
const WL_SURFACE_SET_BUFFER_SCALE: c_uint = 8;
const WL_SURFACE_DAMAGE_BUFFER: c_uint = 9;
const WL_SEAT_GET_POINTER: c_uint = 0;
const XDG_WM_BASE_DESTROY: c_uint = 0;
const XDG_WM_BASE_GET_XDG_SURFACE: c_uint = 2;
const XDG_WM_BASE_PONG: c_uint = 3;
const XDG_SURFACE_DESTROY: c_uint = 0;
const XDG_SURFACE_GET_TOPLEVEL: c_uint = 1;
const XDG_SURFACE_ACK_CONFIGURE: c_uint = 4;
const XDG_TOPLEVEL_DESTROY: c_uint = 0;
const XDG_TOPLEVEL_SET_TITLE: c_uint = 2;
const XDG_TOPLEVEL_SET_APP_ID: c_uint = 3;

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
    pub fn wl_proxy_get_version(proxy: *mut wl_proxy) -> c_uint;
    pub fn wl_proxy_marshal_flags(
        proxy: *mut wl_proxy,
        opcode: c_uint,
        interface: *const wl_interface,
        version: c_uint,
        flags: c_uint,
        ...
    ) -> *mut wl_proxy;
}

unsafe extern "C" {
    static wl_registry_interface: wl_interface;
    static wl_compositor_interface: wl_interface;
    static wl_output_interface: wl_interface;
    static wl_shm_interface: wl_interface;
    static wl_shm_pool_interface: wl_interface;
    static wl_buffer_interface: wl_interface;
    static wl_surface_interface: wl_interface;
    static wl_seat_interface: wl_interface;
    static wl_pointer_interface: wl_interface;
}

static XDG_WM_BASE_CREATE_POSITIONER_TYPES: InterfaceTypeList<1> =
    InterfaceTypeList([&XDG_POSITIONER_INTERFACE]);
static XDG_WM_BASE_GET_XDG_SURFACE_TYPES: InterfaceTypeList<2> = InterfaceTypeList([
    &XDG_SURFACE_INTERFACE,
    unsafe { &wl_surface_interface },
]);
static XDG_SURFACE_GET_TOPLEVEL_TYPES: InterfaceTypeList<1> =
    InterfaceTypeList([&XDG_TOPLEVEL_INTERFACE]);
static XDG_SURFACE_GET_POPUP_TYPES: InterfaceTypeList<3> = InterfaceTypeList([
    &XDG_POPUP_INTERFACE,
    &XDG_SURFACE_INTERFACE,
    &XDG_POSITIONER_INTERFACE,
]);
static XDG_TOPLEVEL_SET_PARENT_TYPES: InterfaceTypeList<1> =
    InterfaceTypeList([&XDG_TOPLEVEL_INTERFACE]);
static XDG_TOPLEVEL_SHOW_WINDOW_MENU_TYPES: InterfaceTypeList<1> =
    InterfaceTypeList([unsafe { &wl_seat_interface }]);
static XDG_TOPLEVEL_MOVE_TYPES: InterfaceTypeList<1> =
    InterfaceTypeList([unsafe { &wl_seat_interface }]);
static XDG_TOPLEVEL_RESIZE_TYPES: InterfaceTypeList<1> =
    InterfaceTypeList([unsafe { &wl_seat_interface }]);
static XDG_TOPLEVEL_SET_FULLSCREEN_TYPES: InterfaceTypeList<1> =
    InterfaceTypeList([unsafe { &wl_output_interface }]);
static XDG_POPUP_GRAB_TYPES: InterfaceTypeList<1> =
    InterfaceTypeList([unsafe { &wl_seat_interface }]);
static XDG_POPUP_REPOSITION_TYPES: InterfaceTypeList<2> =
    InterfaceTypeList([&XDG_POSITIONER_INTERFACE, unsafe { &wl_seat_interface }]);

static XDG_WM_BASE_REQUESTS: [wl_message; 4] = [
    wl_message {
        name: b"destroy\0".as_ptr().cast::<c_char>(),
        signature: b"\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"create_positioner\0".as_ptr().cast::<c_char>(),
        signature: b"n\0".as_ptr().cast::<c_char>(),
        types: XDG_WM_BASE_CREATE_POSITIONER_TYPES.as_ptr(),
    },
    wl_message {
        name: b"get_xdg_surface\0".as_ptr().cast::<c_char>(),
        signature: b"no\0".as_ptr().cast::<c_char>(),
        types: XDG_WM_BASE_GET_XDG_SURFACE_TYPES.as_ptr(),
    },
    wl_message {
        name: b"pong\0".as_ptr().cast::<c_char>(),
        signature: b"u\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
];

static XDG_WM_BASE_EVENTS: [wl_message; 1] = [wl_message {
    name: b"ping\0".as_ptr().cast::<c_char>(),
    signature: b"u\0".as_ptr().cast::<c_char>(),
    types: std::ptr::null(),
}];

static XDG_POSITIONER_REQUESTS: [wl_message; 10] = [
    wl_message {
        name: b"destroy\0".as_ptr().cast::<c_char>(),
        signature: b"\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"set_size\0".as_ptr().cast::<c_char>(),
        signature: b"ii\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"set_anchor_rect\0".as_ptr().cast::<c_char>(),
        signature: b"iiii\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"set_anchor\0".as_ptr().cast::<c_char>(),
        signature: b"u\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"set_gravity\0".as_ptr().cast::<c_char>(),
        signature: b"u\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"set_constraint_adjustment\0".as_ptr().cast::<c_char>(),
        signature: b"u\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"set_offset\0".as_ptr().cast::<c_char>(),
        signature: b"ii\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"set_reactive\0".as_ptr().cast::<c_char>(),
        signature: b"3\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"set_parent_size\0".as_ptr().cast::<c_char>(),
        signature: b"3ii\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"set_parent_configure\0".as_ptr().cast::<c_char>(),
        signature: b"3u\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
];

static XDG_SURFACE_REQUESTS: [wl_message; 5] = [
    wl_message {
        name: b"destroy\0".as_ptr().cast::<c_char>(),
        signature: b"\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"get_toplevel\0".as_ptr().cast::<c_char>(),
        signature: b"n\0".as_ptr().cast::<c_char>(),
        types: XDG_SURFACE_GET_TOPLEVEL_TYPES.as_ptr(),
    },
    wl_message {
        name: b"get_popup\0".as_ptr().cast::<c_char>(),
        signature: b"n?oo\0".as_ptr().cast::<c_char>(),
        types: XDG_SURFACE_GET_POPUP_TYPES.as_ptr(),
    },
    wl_message {
        name: b"set_window_geometry\0".as_ptr().cast::<c_char>(),
        signature: b"iiii\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"ack_configure\0".as_ptr().cast::<c_char>(),
        signature: b"u\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
];

static XDG_SURFACE_EVENTS: [wl_message; 1] = [wl_message {
    name: b"configure\0".as_ptr().cast::<c_char>(),
    signature: b"u\0".as_ptr().cast::<c_char>(),
    types: std::ptr::null(),
}];

static XDG_TOPLEVEL_REQUESTS: [wl_message; 14] = [
    wl_message {
        name: b"destroy\0".as_ptr().cast::<c_char>(),
        signature: b"\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"set_parent\0".as_ptr().cast::<c_char>(),
        signature: b"?o\0".as_ptr().cast::<c_char>(),
        types: XDG_TOPLEVEL_SET_PARENT_TYPES.as_ptr(),
    },
    wl_message {
        name: b"set_title\0".as_ptr().cast::<c_char>(),
        signature: b"s\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"set_app_id\0".as_ptr().cast::<c_char>(),
        signature: b"s\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"show_window_menu\0".as_ptr().cast::<c_char>(),
        signature: b"ouii\0".as_ptr().cast::<c_char>(),
        types: XDG_TOPLEVEL_SHOW_WINDOW_MENU_TYPES.as_ptr(),
    },
    wl_message {
        name: b"move\0".as_ptr().cast::<c_char>(),
        signature: b"ou\0".as_ptr().cast::<c_char>(),
        types: XDG_TOPLEVEL_MOVE_TYPES.as_ptr(),
    },
    wl_message {
        name: b"resize\0".as_ptr().cast::<c_char>(),
        signature: b"ouu\0".as_ptr().cast::<c_char>(),
        types: XDG_TOPLEVEL_RESIZE_TYPES.as_ptr(),
    },
    wl_message {
        name: b"set_max_size\0".as_ptr().cast::<c_char>(),
        signature: b"ii\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"set_min_size\0".as_ptr().cast::<c_char>(),
        signature: b"ii\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"set_maximized\0".as_ptr().cast::<c_char>(),
        signature: b"\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"unset_maximized\0".as_ptr().cast::<c_char>(),
        signature: b"\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"set_fullscreen\0".as_ptr().cast::<c_char>(),
        signature: b"?o\0".as_ptr().cast::<c_char>(),
        types: XDG_TOPLEVEL_SET_FULLSCREEN_TYPES.as_ptr(),
    },
    wl_message {
        name: b"unset_fullscreen\0".as_ptr().cast::<c_char>(),
        signature: b"\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"set_minimized\0".as_ptr().cast::<c_char>(),
        signature: b"\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
];

static XDG_TOPLEVEL_EVENTS: [wl_message; 4] = [
    wl_message {
        name: b"configure\0".as_ptr().cast::<c_char>(),
        signature: b"iia\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"close\0".as_ptr().cast::<c_char>(),
        signature: b"\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"configure_bounds\0".as_ptr().cast::<c_char>(),
        signature: b"4ii\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"wm_capabilities\0".as_ptr().cast::<c_char>(),
        signature: b"5a\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
];

static XDG_POPUP_REQUESTS: [wl_message; 3] = [
    wl_message {
        name: b"destroy\0".as_ptr().cast::<c_char>(),
        signature: b"\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"grab\0".as_ptr().cast::<c_char>(),
        signature: b"ou\0".as_ptr().cast::<c_char>(),
        types: XDG_POPUP_GRAB_TYPES.as_ptr(),
    },
    wl_message {
        name: b"reposition\0".as_ptr().cast::<c_char>(),
        signature: b"3ou\0".as_ptr().cast::<c_char>(),
        types: XDG_POPUP_REPOSITION_TYPES.as_ptr(),
    },
];

static XDG_POPUP_EVENTS: [wl_message; 3] = [
    wl_message {
        name: b"configure\0".as_ptr().cast::<c_char>(),
        signature: b"iiii\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"popup_done\0".as_ptr().cast::<c_char>(),
        signature: b"\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
    wl_message {
        name: b"repositioned\0".as_ptr().cast::<c_char>(),
        signature: b"3u\0".as_ptr().cast::<c_char>(),
        types: std::ptr::null(),
    },
];

static XDG_WM_BASE_INTERFACE: wl_interface = wl_interface {
    name: b"xdg_wm_base\0".as_ptr().cast::<c_char>(),
    version: 6,
    method_count: XDG_WM_BASE_REQUESTS.len() as c_int,
    methods: XDG_WM_BASE_REQUESTS.as_ptr(),
    event_count: XDG_WM_BASE_EVENTS.len() as c_int,
    events: XDG_WM_BASE_EVENTS.as_ptr(),
};

static XDG_POSITIONER_INTERFACE: wl_interface = wl_interface {
    name: b"xdg_positioner\0".as_ptr().cast::<c_char>(),
    version: 6,
    method_count: XDG_POSITIONER_REQUESTS.len() as c_int,
    methods: XDG_POSITIONER_REQUESTS.as_ptr(),
    event_count: 0,
    events: std::ptr::null(),
};

static XDG_SURFACE_INTERFACE: wl_interface = wl_interface {
    name: b"xdg_surface\0".as_ptr().cast::<c_char>(),
    version: 6,
    method_count: XDG_SURFACE_REQUESTS.len() as c_int,
    methods: XDG_SURFACE_REQUESTS.as_ptr(),
    event_count: XDG_SURFACE_EVENTS.len() as c_int,
    events: XDG_SURFACE_EVENTS.as_ptr(),
};

static XDG_TOPLEVEL_INTERFACE: wl_interface = wl_interface {
    name: b"xdg_toplevel\0".as_ptr().cast::<c_char>(),
    version: 6,
    method_count: XDG_TOPLEVEL_REQUESTS.len() as c_int,
    methods: XDG_TOPLEVEL_REQUESTS.as_ptr(),
    event_count: XDG_TOPLEVEL_EVENTS.len() as c_int,
    events: XDG_TOPLEVEL_EVENTS.as_ptr(),
};

static XDG_POPUP_INTERFACE: wl_interface = wl_interface {
    name: b"xdg_popup\0".as_ptr().cast::<c_char>(),
    version: 6,
    method_count: XDG_POPUP_REQUESTS.len() as c_int,
    methods: XDG_POPUP_REQUESTS.as_ptr(),
    event_count: XDG_POPUP_EVENTS.len() as c_int,
    events: XDG_POPUP_EVENTS.as_ptr(),
};

pub unsafe fn oab_wl_display_get_registry(display: *mut wl_display) -> *mut wl_registry {
    let display_proxy = display.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(display_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            display_proxy,
            WL_DISPLAY_GET_REGISTRY,
            &wl_registry_interface,
            version,
            0,
            std::ptr::null_mut::<wl_proxy>(),
        )
    }
    .cast::<wl_registry>()
}

pub unsafe fn oab_wl_registry_bind_compositor(
    registry: *mut wl_registry,
    name: c_uint,
    version: c_uint,
) -> *mut wl_compositor {
    let interface = unsafe { &wl_compositor_interface };
    unsafe { bind_registry_interface(registry, name, version, interface, b"wl_compositor\0") }
        .cast::<wl_compositor>()
}

pub unsafe fn oab_wl_registry_bind_shm(
    registry: *mut wl_registry,
    name: c_uint,
    version: c_uint,
) -> *mut wl_shm {
    let interface = unsafe { &wl_shm_interface };
    unsafe { bind_registry_interface(registry, name, version, interface, b"wl_shm\0") }
        .cast::<wl_shm>()
}

pub unsafe fn oab_wl_registry_bind_seat(
    registry: *mut wl_registry,
    name: c_uint,
    version: c_uint,
) -> *mut wl_seat {
    let interface = unsafe { &wl_seat_interface };
    unsafe { bind_registry_interface(registry, name, version, interface, b"wl_seat\0") }
        .cast::<wl_seat>()
}

pub unsafe fn oab_wl_registry_bind_xdg_wm_base(
    registry: *mut wl_registry,
    name: c_uint,
    version: c_uint,
) -> *mut xdg_wm_base {
    let interface = &XDG_WM_BASE_INTERFACE;
    unsafe { bind_registry_interface(registry, name, version, interface, b"xdg_wm_base\0") }
        .cast::<xdg_wm_base>()
}

unsafe fn bind_registry_interface(
    registry: *mut wl_registry,
    name: c_uint,
    version: c_uint,
    interface: *const wl_interface,
    interface_name: &[u8],
) -> *mut wl_proxy {
    debug_assert!(interface_name.last().copied() == Some(0));
    let interface_name_ptr = interface_name.as_ptr().cast::<c_char>();
    let registry_proxy = registry.cast::<wl_proxy>();
    unsafe {
        wl_proxy_marshal_flags(
            registry_proxy,
            WL_REGISTRY_BIND,
            interface,
            version,
            0,
            name,
            interface_name_ptr,
            version,
            std::ptr::null_mut::<wl_proxy>(),
        )
    }
}

pub unsafe fn oab_wl_compositor_create_surface(compositor: *mut wl_compositor) -> *mut wl_surface {
    let compositor_proxy = compositor.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(compositor_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            compositor_proxy,
            WL_COMPOSITOR_CREATE_SURFACE,
            &wl_surface_interface,
            version,
            0,
            std::ptr::null_mut::<wl_proxy>(),
        )
    }
    .cast::<wl_surface>()
}

pub unsafe fn oab_wl_shm_create_pool(shm: *mut wl_shm, fd: c_int, size: c_int) -> *mut wl_shm_pool {
    let shm_proxy = shm.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(shm_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            shm_proxy,
            WL_SHM_CREATE_POOL,
            &wl_shm_pool_interface,
            version,
            0,
            std::ptr::null_mut::<wl_proxy>(),
            fd,
            size,
        )
    }
    .cast::<wl_shm_pool>()
}

pub unsafe fn oab_wl_shm_pool_create_buffer(
    pool: *mut wl_shm_pool,
    offset: c_int,
    width: c_int,
    height: c_int,
    stride: c_int,
    format: c_uint,
) -> *mut wl_buffer {
    let pool_proxy = pool.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(pool_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            pool_proxy,
            WL_SHM_POOL_CREATE_BUFFER,
            &wl_buffer_interface,
            version,
            0,
            std::ptr::null_mut::<wl_proxy>(),
            offset,
            width,
            height,
            stride,
            format,
        )
    }
    .cast::<wl_buffer>()
}

pub unsafe fn oab_wl_shm_pool_destroy(pool: *mut wl_shm_pool) {
    let pool_proxy = pool.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(pool_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            pool_proxy,
            WL_SHM_POOL_DESTROY,
            std::ptr::null(),
            version,
            WL_MARSHAL_FLAG_DESTROY,
        );
    }
}

pub unsafe fn oab_wl_buffer_destroy(buffer: *mut wl_buffer) {
    let buffer_proxy = buffer.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(buffer_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            buffer_proxy,
            WL_BUFFER_DESTROY,
            std::ptr::null(),
            version,
            WL_MARSHAL_FLAG_DESTROY,
        );
    }
}

pub unsafe fn oab_wl_surface_attach(
    surface: *mut wl_surface,
    buffer: *mut wl_buffer,
    x: c_int,
    y: c_int,
) {
    let surface_proxy = surface.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(surface_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            surface_proxy,
            WL_SURFACE_ATTACH,
            std::ptr::null(),
            version,
            0,
            buffer,
            x,
            y,
        );
    }
}

pub unsafe fn oab_wl_surface_damage_buffer(
    surface: *mut wl_surface,
    x: c_int,
    y: c_int,
    width: c_int,
    height: c_int,
) {
    let surface_proxy = surface.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(surface_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            surface_proxy,
            WL_SURFACE_DAMAGE_BUFFER,
            std::ptr::null(),
            version,
            0,
            x,
            y,
            width,
            height,
        );
    }
}

pub unsafe fn oab_wl_surface_set_buffer_scale(surface: *mut wl_surface, scale: c_int) {
    let surface_proxy = surface.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(surface_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            surface_proxy,
            WL_SURFACE_SET_BUFFER_SCALE,
            std::ptr::null(),
            version,
            0,
            scale,
        );
    }
}

pub unsafe fn oab_wl_surface_commit(surface: *mut wl_surface) {
    let surface_proxy = surface.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(surface_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            surface_proxy,
            WL_SURFACE_COMMIT,
            std::ptr::null(),
            version,
            0,
        );
    }
}

pub unsafe fn oab_wl_surface_destroy(surface: *mut wl_surface) {
    let surface_proxy = surface.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(surface_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            surface_proxy,
            WL_SURFACE_DESTROY,
            std::ptr::null(),
            version,
            WL_MARSHAL_FLAG_DESTROY,
        );
    }
}

pub unsafe fn oab_wl_seat_get_pointer(seat: *mut wl_seat) -> *mut wl_pointer {
    let seat_proxy = seat.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(seat_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            seat_proxy,
            WL_SEAT_GET_POINTER,
            &wl_pointer_interface,
            version,
            0,
            std::ptr::null_mut::<wl_proxy>(),
        )
    }
    .cast::<wl_pointer>()
}

pub unsafe fn oab_wl_shm_release(shm: *mut wl_shm) {
    let shm_proxy = shm.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(shm_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            shm_proxy,
            WL_SHM_RELEASE,
            std::ptr::null(),
            version,
            WL_MARSHAL_FLAG_DESTROY,
        );
    }
}

pub unsafe fn oab_xdg_wm_base_get_xdg_surface(
    wm_base: *mut xdg_wm_base,
    surface: *mut wl_surface,
) -> *mut xdg_surface {
    let wm_base_proxy = wm_base.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(wm_base_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            wm_base_proxy,
            XDG_WM_BASE_GET_XDG_SURFACE,
            &XDG_SURFACE_INTERFACE,
            version,
            0,
            std::ptr::null_mut::<wl_proxy>(),
            surface,
        )
    }
    .cast::<xdg_surface>()
}

pub unsafe fn oab_xdg_wm_base_pong(wm_base: *mut xdg_wm_base, serial: c_uint) {
    let wm_base_proxy = wm_base.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(wm_base_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            wm_base_proxy,
            XDG_WM_BASE_PONG,
            std::ptr::null(),
            version,
            0,
            serial,
        );
    }
}

pub unsafe fn oab_xdg_wm_base_destroy(wm_base: *mut xdg_wm_base) {
    let wm_base_proxy = wm_base.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(wm_base_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            wm_base_proxy,
            XDG_WM_BASE_DESTROY,
            std::ptr::null(),
            version,
            WL_MARSHAL_FLAG_DESTROY,
        );
    }
}

pub unsafe fn oab_xdg_surface_get_toplevel(surface: *mut xdg_surface) -> *mut xdg_toplevel {
    let surface_proxy = surface.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(surface_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            surface_proxy,
            XDG_SURFACE_GET_TOPLEVEL,
            &XDG_TOPLEVEL_INTERFACE,
            version,
            0,
            std::ptr::null_mut::<wl_proxy>(),
        )
    }
    .cast::<xdg_toplevel>()
}

pub unsafe fn oab_xdg_surface_ack_configure(surface: *mut xdg_surface, serial: c_uint) {
    let surface_proxy = surface.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(surface_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            surface_proxy,
            XDG_SURFACE_ACK_CONFIGURE,
            std::ptr::null(),
            version,
            0,
            serial,
        );
    }
}

pub unsafe fn oab_xdg_surface_destroy(surface: *mut xdg_surface) {
    let surface_proxy = surface.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(surface_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            surface_proxy,
            XDG_SURFACE_DESTROY,
            std::ptr::null(),
            version,
            WL_MARSHAL_FLAG_DESTROY,
        );
    }
}

pub unsafe fn oab_xdg_toplevel_set_title(toplevel: *mut xdg_toplevel, title: *const c_char) {
    let toplevel_proxy = toplevel.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(toplevel_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            toplevel_proxy,
            XDG_TOPLEVEL_SET_TITLE,
            std::ptr::null(),
            version,
            0,
            title,
        );
    }
}

pub unsafe fn oab_xdg_toplevel_set_app_id(toplevel: *mut xdg_toplevel, app_id: *const c_char) {
    let toplevel_proxy = toplevel.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(toplevel_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            toplevel_proxy,
            XDG_TOPLEVEL_SET_APP_ID,
            std::ptr::null(),
            version,
            0,
            app_id,
        );
    }
}

pub unsafe fn oab_xdg_toplevel_destroy(toplevel: *mut xdg_toplevel) {
    let toplevel_proxy = toplevel.cast::<wl_proxy>();
    let version = unsafe { wl_proxy_get_version(toplevel_proxy) };
    unsafe {
        wl_proxy_marshal_flags(
            toplevel_proxy,
            XDG_TOPLEVEL_DESTROY,
            std::ptr::null(),
            version,
            WL_MARSHAL_FLAG_DESTROY,
        );
    }
}

#[inline]
pub fn wl_fixed_to_f64(value: wl_fixed_t) -> f64 {
    f64::from(value) / 256.0
}
