use std::sync::LazyLock;

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows_reactor::ThreadSender;

use super::app::AppMessage;
use super::utils::OwnedHWND;

#[cfg(debug_assertions)]
const MESSAGE_WINDOW_CLASSNAME: PCWSTR = w!("komorebi-switcher-debug::message-window");
#[cfg(not(debug_assertions))]
const MESSAGE_WINDOW_CLASSNAME: PCWSTR = w!("komorebi-switcher::message-window");
static WM_TASKBARCREATED: LazyLock<u32> =
	LazyLock::new(|| unsafe { RegisterWindowMessageA(s!("TaskbarCreated")) });

pub unsafe fn create(sender: ThreadSender<AppMessage>) -> anyhow::Result<OwnedHWND> {
	let instance = GetModuleHandleW(None)?;
	let class = WNDCLASSW {
		hInstance: instance.into(),
		lpszClassName: MESSAGE_WINDOW_CLASSNAME,
		lpfnWndProc: Some(wndproc),
		..Default::default()
	};

	RegisterClassW(&class);

	let hwnd = CreateWindowExW(
		// WS_EX_TOOLWINDOW keeps this hidden receiver out of the taskbar, including
		// after Explorer restarts. Without it, a taskbar entry can appear later even
		// though the window was initially hidden.
		WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
		MESSAGE_WINDOW_CLASSNAME,
		None,
		WS_OVERLAPPED,
		0,
		0,
		0,
		0,
		None,
		None,
		Some(instance.into()),
		Some((&sender as *const ThreadSender<AppMessage>).cast()),
	)?;

	Ok(OwnedHWND(hwnd))
}

unsafe extern "system" fn wndproc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
	if msg == WM_NCCREATE {
		let create = &*(lparam.0 as *const CREATESTRUCTW);
		let sender = &*(create.lpCreateParams as *const ThreadSender<AppMessage>);
		let data = Box::into_raw(Box::new(sender.clone()));
		SetWindowLongPtrW(hwnd, GWLP_USERDATA, data as _);

		return LRESULT(1);
	}

	let data = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ThreadSender<AppMessage>;

	if !data.is_null() {
		match msg {
			msg if msg == *WM_TASKBARCREATED => {
				let _ = (*data).send(AppMessage::TaskbarRecreated);
			}

			WM_NCDESTROY => {
				SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
				drop(Box::from_raw(data));
			}

			_ => {}
		}
	}

	DefWindowProcW(hwnd, msg, wparam, lparam)
}
