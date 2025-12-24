use std::sync::LazyLock;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use winit::event_loop::EventLoopProxy;

use crate::app::AppMessage;

#[cfg(debug_assertions)]
const MESSAGE_WINDOW_CLASSNAME: PCWSTR = w!("komorebi-switcher-debug::message-window");
#[cfg(not(debug_assertions))]
const MESSAGE_WINDOW_CLASSNAME: PCWSTR = w!("komorebi-switcher::message-window");

static WM_TASKBARCREATED: LazyLock<u32> =
    LazyLock::new(|| unsafe { RegisterWindowMessageA(s!("TaskbarCreated")) });

struct WndProcUserData {
    proxy: EventLoopProxy<AppMessage>,
}

impl WndProcUserData {
    unsafe fn from_hwnd(hwnd: HWND) -> &'static mut Self {
        &mut *(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Self)
    }
}

pub unsafe fn create(proxy: EventLoopProxy<AppMessage>) -> anyhow::Result<HWND> {
    let hinstance = GetModuleHandleW(None)?;

    let wc = WNDCLASSW {
        hInstance: hinstance.into(),
        lpszClassName: MESSAGE_WINDOW_CLASSNAME,
        lpfnWndProc: Some(wndproc_message_window),
        ..Default::default()
    };

    RegisterClassW(&wc);

    let userdata = WndProcUserData { proxy };

    let hwnd = CreateWindowExW(
        WS_EX_NOACTIVATE | WS_EX_TRANSPARENT | WS_EX_LAYERED |
            // WS_EX_TOOLWINDOW prevents this window from ever showing up in the taskbar, which
            // we want to avoid. If you remove this style, this window won't show up in the
            // taskbar *initially*, but it can show up at some later point. This can sometimes
            // happen on its own after several hours have passed, although this has proven
            // difficult to reproduce. Alternatively, it can be manually triggered by killing
            // `explorer.exe` and then starting the process back up.
            // It is unclear why the bug is triggered by waiting for several hours.
            WS_EX_TOOLWINDOW,
        MESSAGE_WINDOW_CLASSNAME,
        None,
        WS_OVERLAPPED,
        CW_USEDEFAULT,
        0,
        CW_USEDEFAULT,
        0,
        None,
        None,
        Some(hinstance.into()),
        Some(Box::into_raw(Box::new(userdata)) as _),
    )?;

    Ok(hwnd)
}

unsafe extern "system" fn wndproc_message_window(
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

        // Handle taskbar recreation (explorer.exe restart)
        msg if msg == *WM_TASKBARCREATED => {
            let userdata = WndProcUserData::from_hwnd(hwnd);
            if let Err(e) = userdata.proxy.send_event(AppMessage::TaskbarRecreated) {
                tracing::error!("Failed to send `AppMessage::TaskbarRecreated`: {e}")
            }
        }

        WM_DESTROY => {
            // Drop userdata
            let userdata = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            let userdata = userdata as *mut WndProcUserData;
            if !userdata.is_null() {
                drop(Box::from_raw(userdata));
            }
        }

        _ => {}
    }

    DefWindowProcW(hwnd, msg, wparam, lparam)
}
