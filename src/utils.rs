use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

pub fn enum_child_windows(hwnd: HWND) -> Vec<HWND> {
    let mut children = Vec::new();

    let children_ptr = &mut children as *mut Vec<HWND>;
    let children_ptr = LPARAM(children_ptr as _);

    unsafe extern "system" fn proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let windows = &mut *(lparam.0 as *mut Vec<HWND>);
        windows.push(hwnd);
        true.into()
    }

    let _ = unsafe { EnumChildWindows(Some(hwnd), Some(proc), children_ptr) };

    children
}

pub fn get_class_name(hwnd: HWND) -> String {
    let mut buffer: [u16; 256] = [0; 256];
    let len = unsafe { GetClassNameW(hwnd, &mut buffer) };
    String::from_utf16_lossy(&buffer[..len as usize])
}

pub struct TopLevelWindowsIterator {
    current: HWND,
}

impl TopLevelWindowsIterator {
    pub fn new() -> Self {
        Self {
            current: unsafe { GetTopWindow(None).unwrap_or_default() },
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = windows::core::Result<HWND>> {
        TopLevelWindowsIterator {
            current: self.current,
        }
    }
}

impl Iterator for TopLevelWindowsIterator {
    type Item = windows::core::Result<HWND>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(hwnd) = unsafe { GetWindow(self.current, GW_HWNDNEXT) } {
            self.current = hwnd;
            Some(Ok(hwnd))
        } else {
            None
        }
    }
}

pub trait RECTExt {
    fn contains(&self, other: &RECT) -> bool;
}

impl RECTExt for RECT {
    fn contains(&self, other: &RECT) -> bool {
        self.left <= other.left && self.top <= other.top
    }
}
