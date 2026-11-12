use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::config::MonitorConfig;
use crate::windows::utils;

#[cfg(debug_assertions)]
const HOST_CLASSNAME: PCWSTR = w!("komorebi-switcher-debug::host");
#[cfg(not(debug_assertions))]
const HOST_CLASSNAME: PCWSTR = w!("komorebi-switcher::host");

pub unsafe fn create(
	taskbar_hwnd: HWND,
	monitor_config: &MonitorConfig,
) -> anyhow::Result<utils::OwnedHWND> {
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

	let _ = unsafe { RegisterClassW(&wc) };

	let height = if monitor_config.auto_height {
		rect.bottom - rect.top
	} else {
		monitor_config.height
	};

	let hwnd = utils::OwnedHWND(CreateWindowExW(
		WS_EX_NOACTIVATE | WS_EX_NOREDIRECTIONBITMAP,
		HOST_CLASSNAME,
		PCWSTR::null(),
		WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS,
		monitor_config.x,
		monitor_config.y,
		monitor_config.width,
		height,
		Some(taskbar_hwnd),
		None,
		None,
		None,
	)?);

	SetWindowPos(
		hwnd.0,
		Some(HWND_TOP),
		0,
		0,
		0,
		0,
		SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
	)?;

	Ok(hwnd)
}

unsafe extern "system" fn wndproc_host(
	hwnd: HWND,
	msg: u32,
	wparam: WPARAM,
	lparam: LPARAM,
) -> LRESULT {
	match msg {
		WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as _),
		WM_POINTERACTIVATE => return LRESULT(PA_NOACTIVATE as _),
		// WinUI owns context menus; do not forward requests to Explorer.
		WM_CONTEXTMENU => return LRESULT(0),
		// Resize children when this host is resized
		WM_SIZE => {
			let mut rect = RECT::default();

			if GetClientRect(hwnd, &mut rect).is_ok() {
				let width = rect.right - rect.left;
				let height = rect.bottom - rect.top;

				for child in utils::direct_child_windows(hwnd) {
					if let Err(e) = SetWindowPos(
						child,
						None,
						0,
						0,
						width,
						height,
						SWP_NOMOVE | SWP_FRAMECHANGED | SWP_NOACTIVATE,
					) {
						tracing::error!("Failed to resize child to match host: {e}")
					}
				}
			}
		}

		_ => {}
	}

	DefWindowProcW(hwnd, msg, wparam, lparam)
}
