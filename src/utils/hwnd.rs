use windows::Win32::Foundation::HWND;

/// A simple wrapper around HWND that destroys the window when dropped.
pub struct HwndWithDrop(pub HWND);

impl Drop for HwndWithDrop {
    fn drop(&mut self) {
        unsafe {
            let _ = windows::Win32::UI::WindowsAndMessaging::DestroyWindow(self.0);
        }
    }
}
