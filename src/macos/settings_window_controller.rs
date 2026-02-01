use std::cell::OnceCell;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, ClassType, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSBackingStoreType, NSWindow, NSWindowController, NSWindowStyleMask};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSRect, NSSize, NSString};

use super::settings_view_controller::SettingsViewController;

define_class!(
    #[unsafe(super = NSWindowController)]
    #[thread_kind = MainThreadOnly]
    #[ivars = SettingsWindowControllerIvars]
    pub struct SettingsWindowController;

    unsafe impl NSObjectProtocol for SettingsWindowController {}

    unsafe impl NSWindowController for SettingsWindowController {
        #[unsafe(method(windowWillClose:))]
        fn window_will_close(&self, _notification: &NSObject) {
            let mtm = self.mtm();
            if let Some(view_controller) = self.ivars().settings_view_controller.take() {
                let _: Retained<SettingsViewController> =
                    unsafe { msg_send![&view_controller, release] };
            }
        }
    }
);

#[derive(Debug)]
pub struct SettingsWindowControllerIvars {
    settings_view_controller: OnceCell<Retained<SettingsViewController>>,
}

impl Default for SettingsWindowControllerIvars {
    fn default() -> Self {
        Self {
            settings_view_controller: OnceCell::new(),
        }
    }
}

impl SettingsWindowController {
    pub fn new(mtm: MainThreadMarker, config: crate::config::Config) -> Retained<Self> {
        let content_rect = NSRect::new(super::NSPoint::new(0.0, 0.0), NSSize::new(350.0, 150.0));

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
        window.setContentViewController(Some(ProtocolObject::from_ref(&*view_controller)));

        let this = Self::alloc(mtm).set_ivars(SettingsWindowControllerIvars::default());
        let this: Retained<Self> = unsafe { msg_send![super(this), initWithWindow: &window] };

        let _ = this.ivars().settings_view_controller.set(view_controller);

        window.setDelegate(Some(ProtocolObject::from_ref(&*this)));

        this
    }

    pub fn show(&self) {
        self.window().makeKeyAndOrderFront(None);
    }
}
