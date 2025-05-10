use windows::Win32::Foundation::*;
use windows::Win32::UI::WindowsAndMessaging::*;

use crate::utils;

#[derive(Debug, Clone, Copy)]
pub struct Taskbar {
    pub hwnd: HWND,
    pub rect: RECT,
}

pub const TASKBAR_CLASS_NAME: &str = "Shell_TrayWnd";
pub const TASKBAR_SECONDARY_CLASS_NAME: &str = "Shell_SecondaryTrayWnd";

fn is_taskbar(hwnd: HWND) -> bool {
    let class_name = utils::get_class_name(hwnd);
    class_name == TASKBAR_CLASS_NAME || class_name == TASKBAR_SECONDARY_CLASS_NAME
}

pub fn all() -> Vec<Taskbar> {
    utils::TopLevelWindowsIterator::new()
        .iter()
        .filter_map(|hwnd| {
            let hwnd = hwnd.ok()?;
            if is_taskbar(hwnd) {
                let mut rect = Default::default();
                unsafe { GetWindowRect(hwnd, &mut rect) }.ok()?;
                Some(Taskbar { hwnd, rect })
            } else {
                None
            }
        })
        .collect()
}
