use std::cell::OnceCell;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSBackingStoreType, NSWindow, NSWindowController, NSWindowDelegate, NSWindowStyleMask,
};
use objc2_foundation::{MainThreadMarker, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};

use super::SettingsViewController;

define_class!(
    #[unsafe(super = NSWindowController)]
    #[thread_kind = MainThreadOnly]
    #[ivars = SettingsWindowControllerIvars]
    #[derive(Debug)]
    pub struct SettingsWindowController;

    unsafe impl NSObjectProtocol for SettingsWindowController {}

    unsafe impl NSWindowDelegate for SettingsWindowController {}
);

#[derive(Debug, Default)]
pub struct SettingsWindowControllerIvars {
    view_controller: OnceCell<Retained<SettingsViewController>>,
}

impl SettingsWindowController {
    pub fn new(mtm: MainThreadMarker, config: crate::config::Config) -> Retained<Self> {
        let content_rect = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(450.0, 600.0));

        let style_mask =
            NSWindowStyleMask::Titled | NSWindowStyleMask::Closable | NSWindowStyleMask::Resizable;

        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                NSWindow::alloc(mtm),
                content_rect,
                style_mask,
                NSBackingStoreType::Buffered,
                false,
            )
        };

        window.setTitle(&NSString::from_str("Settings"));

        let view_controller = SettingsViewController::new(mtm, config);
        window.setContentViewController(Some(&view_controller));

        let this = Self::alloc(mtm).set_ivars(SettingsWindowControllerIvars::default());
        let window_ref: &NSWindow = &window;
        let this: Retained<Self> =
            unsafe { msg_send![super(this), initWithWindow: Some(window_ref)] };

        let _ = this.ivars().view_controller.set(view_controller);

        window.setDelegate(Some(ProtocolObject::from_ref(&*this)));

        this
    }

    pub fn show(&self) {
        if let Some(window) = self.window() {
            window.makeKeyAndOrderFront(None);
        }
    }
}
