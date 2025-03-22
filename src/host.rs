use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows_registry::CURRENT_USER;
use winit::event_loop::EventLoopProxy;

use crate::app::AppMessage;

#[cfg(debug_assertions)]
const APP_REG_KEY: &str = "SOFTWARE\\amrbashir\\komorebi-switcher-debug";
#[cfg(not(debug_assertions))]
const APP_REG_KEY: &str = "SOFTWARE\\amrbashir\\komorebi-switcher";

const WINDOW_POS_X_KEY: &str = "window-pos-x";
const WINDOW_POS_Y_KEY: &str = "window-pos-y";

#[cfg(debug_assertions)]
const HOST_CLASSNAME: PCWSTR = w!("komorebi-switcher-debug::host");
#[cfg(not(debug_assertions))]
const HOST_CLASSNAME: PCWSTR = w!("komorebi-switcher::host");

struct WndProcUserData {
    proxy: EventLoopProxy<AppMessage>,
    taskbar_hwnd: HWND,
}

impl WndProcUserData {
    unsafe fn from_hwnd(hwnd: HWND) -> &'static mut Self {
        &mut *(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Self)
    }
}

unsafe extern "system" fn enum_child_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let children = &mut *(lparam.0 as *mut Vec<HWND>);
    children.push(hwnd);
    true.into()
}

fn enum_child_windows(hwnd: HWND) -> Vec<HWND> {
    let mut children = Vec::new();

    let children_ptr = &mut children as *mut Vec<HWND>;
    let children_ptr = LPARAM(children_ptr as _);

    let _ = unsafe { EnumChildWindows(Some(hwnd), Some(enum_child_proc), children_ptr) };

    children
}

unsafe extern "system" fn wndproc_host(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        // Initialize GWLP_USERDATA
        WM_CREATE => {
            let create_struct = &*(lparam.0 as *const CREATESTRUCTW);
            let userdata = create_struct.lpCreateParams as *const WndProcUserData;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, userdata as _);
        }

        // Notify app to update DPI
        WM_DPICHANGED_AFTERPARENT => {
            let userdata = WndProcUserData::from_hwnd(hwnd);
            if let Err(e) = userdata.proxy.send_event(AppMessage::DpiChanged) {
                tracing::error!("Failed to send `AppMessage::DpiChanged`: {e}")
            }
        }

        // Disable position changes in y direction
        // and clamp x direction to stay visible in taskbar
        WM_WINDOWPOSCHANGING => {
            let window_pos = &mut *(lparam.0 as *mut WINDOWPOS);
            window_pos.y = 0;

            let userdata = WndProcUserData::from_hwnd(hwnd);

            let mut rect = RECT::default();
            if GetClientRect(userdata.taskbar_hwnd, &mut rect).is_ok() {
                window_pos.x = window_pos.x.max(0).min(rect.right - window_pos.cx);
            }
        }

        // Save host position to be loaded on startup
        WM_WINDOWPOSCHANGED => {
            let window_pos = &*(lparam.0 as *const WINDOWPOS);

            let key = CURRENT_USER.create(APP_REG_KEY);
            if let Ok(key) = key {
                let x = window_pos.x;
                let y = window_pos.y;

                tracing::debug!("Storing window position into registry {x},{y}");

                if let Err(e) = key.set_string(WINDOW_POS_X_KEY, &x.to_string()) {
                    tracing::error!("Failed to store window pos x into registry: {e}")
                }

                if let Err(e) = key.set_string(WINDOW_POS_Y_KEY, &y.to_string()) {
                    tracing::error!("Failed to store window pos y into registry: {e}")
                }
            }
        }

        // Resize children when this host is resized
        WM_SIZE => {
            let mut rect = RECT::default();
            if GetClientRect(hwnd, &mut rect).is_ok() {
                let width = rect.right - rect.left;
                let height = rect.bottom - rect.top;

                for child in enum_child_windows(hwnd) {
                    if let Err(e) = SetWindowPos(
                        child,
                        None,
                        0,
                        0,
                        width,
                        height,
                        SWP_NOMOVE | SWP_FRAMECHANGED,
                    ) {
                        tracing::error!("Failed to resize child to match host: {e}")
                    }
                }
            }
        }

        // Notify app to update system settings like accent colors
        WM_SETTINGCHANGE => {
            let userdata = WndProcUserData::from_hwnd(hwnd);
            if let Err(e) = userdata.proxy.send_event(AppMessage::SystemSettingsChanged) {
                tracing::error!("Failed to send `AppMessage::SystemSettingsChanged`: {e}")
            }
        }

        // Close children when this host is closed
        WM_CLOSE => {
            for child in enum_child_windows(hwnd) {
                let _ = SendMessageW(child, WM_CLOSE, None, None);
            }

            // Drop userdata
            let userdata = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            let userdata = userdata as *mut WndProcUserData;
            drop(Box::from_raw(userdata));
        }

        _ => {}
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}

pub unsafe fn create_host(
    taskbar_hwnd: HWND,
    proxy: EventLoopProxy<AppMessage>,
) -> anyhow::Result<HWND> {
    let hinstance = unsafe { GetModuleHandleW(None) }?;

    let mut rect = RECT::default();
    GetClientRect(taskbar_hwnd, &mut rect)?;

    let wc = WNDCLASSW {
        hInstance: hinstance.into(),
        lpszClassName: HOST_CLASSNAME,
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(wndproc_host),
        ..Default::default()
    };

    let atom = RegisterClassW(&wc);
    debug_assert!(atom != 0);

    tracing::debug!("Loading window position from registry");
    let key = CURRENT_USER.create(APP_REG_KEY)?;
    let window_pos_x = key.get_string(WINDOW_POS_X_KEY).ok();
    let window_pos_y = key.get_string(WINDOW_POS_Y_KEY).ok();
    let window_pos_x = window_pos_x.and_then(|s| s.parse().ok());
    let window_pos_y = window_pos_y.and_then(|s| s.parse().ok());

    let userdata = WndProcUserData {
        proxy,
        taskbar_hwnd,
    };

    let hwnd = CreateWindowExW(
        WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP,
        HOST_CLASSNAME,
        PCWSTR::null(),
        WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
        window_pos_x.unwrap_or(16),
        window_pos_y.unwrap_or(0),
        200,
        rect.bottom - rect.top,
        Some(taskbar_hwnd),
        None,
        None,
        Some(Box::into_raw(Box::new(userdata)) as _),
    )?;

    SetWindowPos(
        hwnd,
        Some(HWND_TOP),
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
    )?;

    Ok(hwnd)
}
