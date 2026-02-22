use super::WindowOptions;
use super::painter::WinPainter;
use super::scale::ScaleFactor;
use super::scaled::ScaledPainter;
use super::wstr;
use crate::app::App;
use crate::render::Viewport;
use core::ffi::c_void;
use std::time::{Duration, Instant};

const MAX_EVENTS_PER_TICK: usize = 512;
const SCREENSHOT_RESOURCE_WAIT_TIMEOUT: Duration = Duration::from_secs(5);
const WHEEL_SCROLL_STEP_PX: i32 = 48;

type BOOL = i32;
type DWORD = u32;
type HBRUSH = *mut c_void;
type HCURSOR = *mut c_void;
type HICON = *mut c_void;
type HINSTANCE = *mut c_void;
type HWND = *mut c_void;
type LPARAM = isize;
type LRESULT = isize;
type UINT = u32;
type WPARAM = usize;

const CS_HREDRAW: UINT = 0x0002;
const CS_VREDRAW: UINT = 0x0001;

const CW_USEDEFAULT: i32 = 0x8000_0000u32 as i32;

const GWLP_USERDATA: i32 = -21;

const IDC_ARROW: *const u16 = 32512usize as *const u16;

const PM_REMOVE: UINT = 0x0001;

const SW_SHOW: i32 = 5;

const VK_ESCAPE: WPARAM = 0x1b;

const WM_NCCREATE: UINT = 0x0081;
const WM_DESTROY: UINT = 0x0002;
const WM_CLOSE: UINT = 0x0010;
const WM_PAINT: UINT = 0x000f;
const WM_ERASEBKGND: UINT = 0x0014;
const WM_SIZE: UINT = 0x0005;
const WM_KEYDOWN: UINT = 0x0100;
const WM_LBUTTONDOWN: UINT = 0x0201;
const WM_MOUSEWHEEL: UINT = 0x020a;
const WM_XBUTTONDOWN: UINT = 0x020b;
const WM_DPICHANGED: UINT = 0x02e0;
const WM_QUIT: UINT = 0x0012;

const WHEEL_DELTA: i32 = 120;
const XBUTTON1: u16 = 0x0001;

const WS_OVERLAPPEDWINDOW: DWORD = 0x00cf_0000;
const WS_VISIBLE: DWORD = 0x1000_0000;

type DpiAwarenessContext = *mut c_void;
const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: DpiAwarenessContext =
    (-4isize) as DpiAwarenessContext;

#[repr(C)]
struct POINT {
    x: i32,
    y: i32,
}

#[repr(C)]
struct MSG {
    hwnd: HWND,
    message: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
    time: DWORD,
    pt: POINT,
}

#[repr(C)]
struct WNDCLASSW {
    style: UINT,
    wnd_proc: unsafe extern "system" fn(HWND, UINT, WPARAM, LPARAM) -> LRESULT,
    cls_extra: i32,
    wnd_extra: i32,
    instance: HINSTANCE,
    icon: HICON,
    cursor: HCURSOR,
    background: HBRUSH,
    menu_name: *const u16,
    class_name: *const u16,
}

#[repr(C)]
struct RECT {
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
}

#[repr(C)]
struct PAINTSTRUCT {
    hdc: *mut c_void,
    f_erase: BOOL,
    rc_paint: RECT,
    f_restore: BOOL,
    f_inc_update: BOOL,
    rgb_reserved: [u8; 32],
}

#[repr(C)]
struct CREATESTRUCTW {
    create_params: *mut c_void,
    instance: HINSTANCE,
    menu: *mut c_void,
    parent: HWND,
    cy: i32,
    cx: i32,
    y: i32,
    x: i32,
    style: i32,
    name: *const u16,
    class_name: *const u16,
    ex_style: DWORD,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterClassW(wnd_class: *const WNDCLASSW) -> u16;
    fn CreateWindowExW(
        ex_style: DWORD,
        class_name: *const u16,
        window_name: *const u16,
        style: DWORD,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        parent: HWND,
        menu: *mut c_void,
        instance: HINSTANCE,
        param: *mut c_void,
    ) -> HWND;
    fn DefWindowProcW(hwnd: HWND, msg: UINT, w_param: WPARAM, l_param: LPARAM) -> LRESULT;
    fn DestroyWindow(hwnd: HWND) -> BOOL;
    fn ShowWindow(hwnd: HWND, cmd_show: i32) -> BOOL;
    fn UpdateWindow(hwnd: HWND) -> BOOL;
    fn PeekMessageW(msg: *mut MSG, hwnd: HWND, min: UINT, max: UINT, remove: UINT) -> BOOL;
    fn TranslateMessage(msg: *const MSG) -> BOOL;
    fn DispatchMessageW(msg: *const MSG) -> LRESULT;
    fn PostQuitMessage(exit_code: i32);
    fn AdjustWindowRectEx(rect: *mut RECT, style: DWORD, menu: BOOL, ex_style: DWORD) -> BOOL;
    fn GetClientRect(hwnd: HWND, rect: *mut RECT) -> BOOL;
    fn GetModuleHandleW(name: *const u16) -> HINSTANCE;
    fn LoadCursorW(instance: HINSTANCE, cursor_name: *const u16) -> HCURSOR;
    fn SetWindowLongPtrW(hwnd: HWND, index: i32, value: isize) -> isize;
    fn GetWindowLongPtrW(hwnd: HWND, index: i32) -> isize;
    fn BeginPaint(hwnd: HWND, ps: *mut PAINTSTRUCT) -> *mut c_void;
    fn EndPaint(hwnd: HWND, ps: *const PAINTSTRUCT) -> BOOL;
    fn SetProcessDpiAwarenessContext(value: DpiAwarenessContext) -> BOOL;
    fn SetWindowPos(
        hwnd: HWND,
        insert_after: HWND,
        x: i32,
        y: i32,
        cx: i32,
        cy: i32,
        flags: UINT,
    ) -> BOOL;
}

const SWP_NOZORDER: UINT = 0x0004;
const SWP_NOACTIVATE: UINT = 0x0010;

#[derive(Clone, Copy, Debug)]
enum WindowEvent {
    MouseDown { x_px: i32, y_px: i32 },
    MouseWheel { wheel_delta: i32 },
    NavigateBack,
}

#[derive(Debug)]
struct WindowState {
    should_close: bool,
    needs_redraw: bool,
    dpi_changed: bool,
    new_client_size: Option<(i32, i32)>,
    events: Vec<WindowEvent>,
}

impl WindowState {
    fn new() -> Self {
        Self {
            should_close: false,
            needs_redraw: false,
            dpi_changed: false,
            new_client_size: None,
            events: Vec::new(),
        }
    }
}

pub(super) fn run<A: App>(title: &str, options: WindowOptions, app: &mut A) -> Result<(), String> {
    let initial_width_css = options.initial_width_px.unwrap_or(1024);
    let initial_height_css = options.initial_height_px.unwrap_or(768);
    if initial_width_css <= 0 || initial_height_css <= 0 {
        return Err(format!(
            "Invalid initial window size: {initial_width_css}x{initial_height_css}"
        ));
    }

    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    let scale_guess = ScaleFactor::detect(false, None);
    let initial_width_device = scale_guess.css_size_to_device_px(initial_width_css);
    let initial_height_device = scale_guess.css_size_to_device_px(initial_height_css);

    let mut state = Box::new(WindowState::new());
    let state_ptr: *mut WindowState = &mut *state;

    let class_name = wstr::utf16_nul("OneAgentOneBrowserWindow");
    let hwnd = create_window(
        title,
        initial_width_device,
        initial_height_device,
        &class_name,
        state_ptr,
    )?;

    let mut scale = ScaleFactor::detect(false, Some(hwnd));

    let mut viewport = client_viewport(hwnd)?;
    if viewport.width_px <= 0 || viewport.height_px <= 0 {
        viewport.width_px = viewport.width_px.max(1);
        viewport.height_px = viewport.height_px.max(1);
    }

    let mut css_viewport = Viewport {
        width_px: scale.device_size_to_css_px(viewport.width_px),
        height_px: scale.device_size_to_css_px(viewport.height_px),
    };

    let mut painter = WinPainter::new(viewport, Some(hwnd))?;

    let mut screenshot_path = options.screenshot_path;

    let mut needs_redraw = true;
    let mut should_exit = false;
    let mut has_rendered_ready_state = false;
    let mut resource_wait_started: Option<Instant> = None;
    let mut wheel_accum: i32 = 0;

    loop {
        let mut processed = 0usize;
        while processed < MAX_EVENTS_PER_TICK {
            let mut msg = MSG {
                hwnd: std::ptr::null_mut(),
                message: 0,
                w_param: 0,
                l_param: 0,
                time: 0,
                pt: POINT { x: 0, y: 0 },
            };
            let has = unsafe { PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) };
            if has == 0 {
                break;
            }
            if msg.message == WM_QUIT {
                should_exit = true;
                break;
            }
            unsafe {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            processed += 1;
        }

        if should_exit || state.should_close {
            break;
        }

        if state.dpi_changed {
            state.dpi_changed = false;
            let next_scale = ScaleFactor::detect(false, Some(hwnd));
            if next_scale != scale {
                scale = next_scale;
                needs_redraw = true;
                has_rendered_ready_state = false;
                resource_wait_started = None;
            }
            viewport = client_viewport(hwnd)?;
            css_viewport = Viewport {
                width_px: scale.device_size_to_css_px(viewport.width_px),
                height_px: scale.device_size_to_css_px(viewport.height_px),
            };
        }

        if let Some((w, h)) = state.new_client_size.take() {
            viewport = Viewport {
                width_px: w,
                height_px: h,
            };
            css_viewport = Viewport {
                width_px: scale.device_size_to_css_px(viewport.width_px),
                height_px: scale.device_size_to_css_px(viewport.height_px),
            };
            needs_redraw = true;
            has_rendered_ready_state = false;
            resource_wait_started = None;
        }

        if state.needs_redraw {
            state.needs_redraw = false;
            needs_redraw = true;
        }

        let events = std::mem::take(&mut state.events);
        for event in events {
            match event {
                WindowEvent::MouseDown { x_px, y_px } => {
                    let x_css = scale.device_coord_to_css_px(x_px);
                    let y_css = scale.device_coord_to_css_px(y_px);
                    let tick = app.mouse_down(x_css, y_css, css_viewport)?;
                    if tick.needs_redraw {
                        needs_redraw = true;
                    }
                }
                WindowEvent::MouseWheel { wheel_delta } => {
                    wheel_accum = wheel_accum.saturating_add(wheel_delta);
                    let steps = wheel_accum / WHEEL_DELTA;
                    if steps != 0 {
                        wheel_accum -= steps * WHEEL_DELTA;
                        let delta_y_device_px = (-steps).saturating_mul(WHEEL_SCROLL_STEP_PX);
                        let delta_y_css = scale.device_delta_to_css_px(delta_y_device_px);
                        let tick = app.mouse_wheel(delta_y_css, css_viewport)?;
                        if tick.needs_redraw {
                            needs_redraw = true;
                        }
                    }
                }
                WindowEvent::NavigateBack => {
                    let tick = app.navigate_back()?;
                    if tick.needs_redraw {
                        needs_redraw = true;
                    }
                }
            }
        }

        if should_exit || state.should_close {
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
        let should_complete_screenshot =
            wants_screenshot && ready_for_screenshot && has_rendered_ready_state;

        let mut capture_now = false;
        let mut capture_after_render = false;

        if ready_for_screenshot && wants_screenshot && !has_rendered_ready_state {
            needs_redraw = true;
        } else if ready_for_screenshot && should_wait_for_resources && has_rendered_ready_state {
            resource_wait_started.get_or_insert(Instant::now());
        } else if ready_for_screenshot && has_rendered_ready_state {
            resource_wait_started = None;
        }

        if ready_for_screenshot
            && has_rendered_ready_state
            && can_complete
            && should_complete_screenshot
        {
            if needs_redraw {
                capture_after_render = true;
            } else {
                capture_now = true;
            }
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
            if viewport.width_px > 0 && viewport.height_px > 0 {
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
                        let rgb = painter.capture_back_buffer_rgb()?;
                        crate::png::write_rgb_png(&path, &rgb)?;
                        break;
                    }
                }
            } else {
                needs_redraw = false;
            }
        }

        if !needs_redraw {
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    unsafe {
        let _ = DestroyWindow(hwnd);
    }

    Ok(())
}

fn create_window(
    title: &str,
    client_width_px: i32,
    client_height_px: i32,
    class_name: &[u16],
    state_ptr: *mut WindowState,
) -> Result<HWND, String> {
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    if instance.is_null() {
        return Err("GetModuleHandleW returned null".to_owned());
    }

    let cursor = unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW) };
    let wc = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        wnd_proc: wnd_proc,
        cls_extra: 0,
        wnd_extra: 0,
        instance,
        icon: std::ptr::null_mut(),
        cursor,
        background: std::ptr::null_mut(),
        menu_name: std::ptr::null(),
        class_name: class_name.as_ptr(),
    };

    unsafe {
        let _ = RegisterClassW(&wc);
    }

    let mut rect = RECT {
        left: 0,
        top: 0,
        right: client_width_px.max(1),
        bottom: client_height_px.max(1),
    };
    unsafe {
        let _ = AdjustWindowRectEx(&mut rect, WS_OVERLAPPEDWINDOW, 0, 0);
    }

    let width = rect.right.saturating_sub(rect.left).max(1);
    let height = rect.bottom.saturating_sub(rect.top).max(1);
    let title_w = wstr::utf16_nul(title);

    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            title_w.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            width,
            height,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            state_ptr.cast::<c_void>(),
        )
    };
    if hwnd.is_null() {
        return Err("CreateWindowExW returned null".to_owned());
    }

    unsafe {
        ShowWindow(hwnd, SW_SHOW);
        UpdateWindow(hwnd);
    }

    Ok(hwnd)
}

fn client_viewport(hwnd: HWND) -> Result<Viewport, String> {
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let ok = unsafe { GetClientRect(hwnd, &mut rect) };
    if ok == 0 {
        return Err("GetClientRect failed".to_owned());
    }
    Ok(Viewport {
        width_px: rect.right.saturating_sub(rect.left),
        height_px: rect.bottom.saturating_sub(rect.top),
    })
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    unsafe {
        if msg == WM_NCCREATE {
            let cs = l_param as *const CREATESTRUCTW;
            if cs.is_null() {
                return 0;
            }
            let state_ptr = (*cs).create_params as *mut WindowState;
            let _ = SetWindowLongPtrW(hwnd, GWLP_USERDATA, state_ptr as isize);
            return 1;
        }

        let state_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowState;
        let state = state_ptr.as_mut();

        match msg {
            WM_DESTROY => {
                if let Some(state) = state {
                    state.should_close = true;
                }
                PostQuitMessage(0);
                return 0;
            }
            WM_CLOSE => {
                if let Some(state) = state {
                    state.should_close = true;
                }
                let _ = DestroyWindow(hwnd);
                return 0;
            }
            WM_KEYDOWN => {
                if w_param == VK_ESCAPE {
                    if let Some(state) = state {
                        state.should_close = true;
                    }
                    let _ = DestroyWindow(hwnd);
                    return 0;
                }
            }
            WM_LBUTTONDOWN => {
                if let Some(state) = state {
                    state.events.push(WindowEvent::MouseDown {
                        x_px: get_x_lparam(l_param),
                        y_px: get_y_lparam(l_param),
                    });
                }
                return 0;
            }
            WM_MOUSEWHEEL => {
                if let Some(state) = state {
                    state.events.push(WindowEvent::MouseWheel {
                        wheel_delta: get_wheel_delta_wparam(w_param),
                    });
                }
                return 0;
            }
            WM_XBUTTONDOWN => {
                let button = get_xbutton_wparam(w_param);
                if button == XBUTTON1 {
                    if let Some(state) = state {
                        state.events.push(WindowEvent::NavigateBack);
                    }
                }
                return 0;
            }
            WM_SIZE => {
                if let Some(state) = state {
                    state.new_client_size = Some((get_x_lparam(l_param), get_y_lparam(l_param)));
                    state.needs_redraw = true;
                }
                return 0;
            }
            WM_DPICHANGED => {
                if let Some(state) = state {
                    state.dpi_changed = true;
                    state.needs_redraw = true;
                }
                let rect = l_param as *const RECT;
                if !rect.is_null() {
                    let suggested = &*rect;
                    let _ = SetWindowPos(
                        hwnd,
                        std::ptr::null_mut(),
                        suggested.left,
                        suggested.top,
                        suggested.right.saturating_sub(suggested.left),
                        suggested.bottom.saturating_sub(suggested.top),
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
                return 0;
            }
            WM_PAINT => {
                if let Some(state) = state {
                    state.needs_redraw = true;
                }
                let mut ps = PAINTSTRUCT {
                    hdc: std::ptr::null_mut(),
                    f_erase: 0,
                    rc_paint: RECT {
                        left: 0,
                        top: 0,
                        right: 0,
                        bottom: 0,
                    },
                    f_restore: 0,
                    f_inc_update: 0,
                    rgb_reserved: [0; 32],
                };
                let _ = BeginPaint(hwnd, &mut ps);
                let _ = EndPaint(hwnd, &ps);
                return 0;
            }
            WM_ERASEBKGND => {
                return 1;
            }
            _ => {}
        }

        DefWindowProcW(hwnd, msg, w_param, l_param)
    }
}

fn get_x_lparam(l_param: LPARAM) -> i32 {
    let x = (l_param as u32 & 0xFFFF) as u16;
    (x as i16) as i32
}

fn get_y_lparam(l_param: LPARAM) -> i32 {
    let y = ((l_param as u32 >> 16) & 0xFFFF) as u16;
    (y as i16) as i32
}

fn get_wheel_delta_wparam(w_param: WPARAM) -> i32 {
    let delta = ((w_param as u32 >> 16) & 0xFFFF) as u16;
    (delta as i16) as i32
}

fn get_xbutton_wparam(w_param: WPARAM) -> u16 {
    ((w_param as u32 >> 16) & 0xFFFF) as u16
}
