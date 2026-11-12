use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub(super) const TRANSPARENT: windows_reactor::Color = windows_reactor::Color {
	a: 0,
	r: 0,
	g: 0,
	b: 0,
};

pub(super) const TRANSPARENT_BRUSH: windows_reactor::Brush =
	windows_reactor::Brush::Solid(TRANSPARENT);

/// A simple wrapper around HWND that destroys the window when dropped.
pub struct OwnedHWND(pub HWND);

impl Drop for OwnedHWND {
	fn drop(&mut self) {
		unsafe {
			let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(self.0);
		}
	}
}

/// Returns immediate child windows in sibling order, excluding nested descendants.
pub fn direct_child_windows(hwnd: HWND) -> impl Iterator<Item = HWND> {
	let mut next = unsafe { GetWindow(hwnd, GW_CHILD) }.ok();

	std::iter::from_fn(move || {
		let hwnd = next?;
		next = unsafe { GetWindow(hwnd, GW_HWNDNEXT) }.ok();

		Some(hwnd)
	})
}

pub fn get_class_name(hwnd: HWND) -> String {
	let mut buffer: [u16; 256] = [0; 256];
	let len = unsafe { GetClassNameW(hwnd, &mut buffer) };

	String::from_utf16_lossy(&buffer[..len as usize])
}

/// Returns top-level windows in sibling order, including the first window.
pub fn top_level_windows() -> impl Iterator<Item = HWND> {
	let mut next = unsafe { GetTopWindow(None) }.ok();

	std::iter::from_fn(move || {
		let hwnd = next?;
		next = unsafe { GetWindow(hwnd, GW_HWNDNEXT) }.ok();

		Some(hwnd)
	})
}

pub fn to_reactor_color(color: &str) -> Option<windows_reactor::Color> {
	let rgba = color::parse_color(color)
		.ok()?
		.to_alpha_color::<color::Srgb>()
		.to_rgba8();

	Some(windows_reactor::Color {
		r: rgba.r,
		g: rgba.g,
		b: rgba.b,
		a: rgba.a,
	})
}
