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
    fn intersects(&self, other: &RECT) -> bool;
}

impl RECTExt for RECT {
    fn contains(&self, other: &RECT) -> bool {
        // Check if the taskbar (other) belongs to this monitor (self)
        // Use intersection-based approach which is more robust than strict containment
        if !self.intersects(other) {
            return false;
        }
        
        // Calculate overlap area to ensure significant overlap
        let overlap_left = self.left.max(other.left);
        let overlap_top = self.top.max(other.top);
        let overlap_right = self.right.min(other.right);
        let overlap_bottom = self.bottom.min(other.bottom);
        
        let overlap_width = (overlap_right - overlap_left).max(0) as f32;
        let overlap_height = (overlap_bottom - overlap_top).max(0) as f32;
        let overlap_area = overlap_width * overlap_height;
        
        let other_width = (other.right - other.left).max(0) as f32;
        let other_height = (other.bottom - other.top).max(0) as f32;
        let other_area = other_width * other_height;
        
        // Require at least 80% overlap to consider the taskbar as belonging to this monitor
        let overlap_ratio = if other_area > 0.0 {
            overlap_area / other_area
        } else {
            0.0
        };
        
        let matches = overlap_ratio >= 0.8;
        
        tracing::debug!(
            "Monitor {:?} vs Taskbar {:?}: overlap_ratio={:.2}, matches={}",
            self, other, overlap_ratio, matches
        );
        
        matches
    }
    
    fn intersects(&self, other: &RECT) -> bool {
        // Check if rectangles intersect at all
        !(other.left >= self.right 
            || other.right <= self.left 
            || other.top >= self.bottom 
            || other.bottom <= self.top)
    }
}
