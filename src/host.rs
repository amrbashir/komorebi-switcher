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

struct WndProcUserData {
    proxy: EventLoopProxy<AppMessage>,
    taskbar_hwnd: HWND,
}

unsafe extern "system" fn enum_child_resize(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let rect = lparam.0 as *const RECT;
    let rect = *rect;

    let width = rect.right - rect.left;
    let height = rect.bottom - rect.top;

    if let Err(e) = SetWindowPos(
        hwnd,
        None,
        0,
        0,
        width,
        height,
        SWP_NOMOVE | SWP_FRAMECHANGED,
    ) {
        tracing::error!("Failed to resize child to match host: {e}")
    }

    true.into()
}

unsafe extern "system" fn enum_child_close(hwnd: HWND, _lparam: LPARAM) -> BOOL {
    let _ = SendMessageW(hwnd, WM_CLOSE, None, None);
    true.into()
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

        // Disable position changes in y direction
        // and clamp x direction to stay visible in taskbar
        WM_WINDOWPOSCHANGING => {
            let window_pos = &mut *(lparam.0 as *mut WINDOWPOS);
            window_pos.y = 0;

            let userdata = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            let userdata = &*(userdata as *const WndProcUserData);

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
                let _ = EnumChildWindows(
                    Some(hwnd),
                    Some(enum_child_resize),
                    LPARAM(&rect as *const _ as _),
                );
            }
        }

        // Notify winit app to update system settings
        WM_SETTINGCHANGE => {
            let userdata = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            let userdata = &*(userdata as *const WndProcUserData);
            if let Err(e) = userdata.proxy.send_event(AppMessage::SystemSettingsChanged) {
                tracing::error!("Failed to send `AppMessage::SystemSettingsChanged`: {e}")
            }
        }

        // Close children when this host is closed
        WM_CLOSE => {
            // Drop userdata
            let userdata = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            let userdata = userdata as *mut WndProcUserData;
            drop(Box::from_raw(userdata));

            let _ = EnumChildWindows(Some(hwnd), Some(enum_child_close), LPARAM::default());
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

    #[cfg(debug_assertions)]
    let window_class = w!("komorebi-switcher-debug::host");
    #[cfg(not(debug_assertions))]
    let window_class = w!("komorebi-switcher::host");

    let wc = WNDCLASSW {
        hInstance: hinstance.into(),
        lpszClassName: window_class,
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
        window_class,
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
