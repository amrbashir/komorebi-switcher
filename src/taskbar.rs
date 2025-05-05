use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::utils;

#[derive(Debug, Clone, Copy)]
pub struct Taskbar {
    pub hwnd: HWND,
    pub x: i32,
    pub y: i32,
}

impl Taskbar {
    pub const TASKBAR_CLASS_NAME: &'static str = "Shell_TrayWnd";
    pub const TASKBAR_SECONDARY_CLASS_NAME: &'static str = "Shell_SecondaryTrayWnd";

    fn is_taskbar(hwnd: HWND) -> bool {
        let class_name = utils::get_class_name(hwnd);
        class_name == Self::TASKBAR_CLASS_NAME || class_name == Self::TASKBAR_SECONDARY_CLASS_NAME
    }

    pub fn all() -> Vec<Self> {
        utils::TopLevelWindowsIterator::new()
            .iter()
            .filter_map(|hwnd| {
                let hwnd = hwnd.ok()?;
                if Self::is_taskbar(hwnd) {
                    let mut rect = Default::default();
                    unsafe { GetWindowRect(hwnd, &mut rect) }.ok()?;
                    Some(Self {
                        hwnd,
                        x: rect.left,
                        y: rect.top,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}
