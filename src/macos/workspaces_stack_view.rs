use objc2::rc::Retained;
use objc2::{define_class, msg_send, sel, MainThreadOnly};
use objc2_app_kit::{NSApp, NSEvent, NSLayoutAttribute, NSMenu, NSMenuItem, NSStackView};
use objc2_foundation::{MainThreadMarker, NSString};

use super::AppDelegate;

define_class!(
    #[unsafe(super = NSStackView)]
    #[thread_kind = MainThreadOnly]
    #[derive(Debug)]
    pub struct WorkspacesStackView;

    impl WorkspacesStackView {
        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, event: &NSEvent) {
            let mtm = self.mtm();

            let menu = NSMenu::new(mtm);

            fn create_item(
                mtm: MainThreadMarker,
                title: &str,
                action: Option<objc2::runtime::Sel>,
            ) -> Retained<NSMenuItem> {
                unsafe {
                    NSMenuItem::initWithTitle_action_keyEquivalent(
                        NSMenuItem::alloc(mtm),
                        &NSString::from_str(title),
                        action,
                        &NSString::from_str(""),
                    )
                }
            }

            let app_name = env!("CARGO_PKG_NAME");
            let title_item = create_item(mtm, app_name, None);
            title_item.setEnabled(false);
            menu.addItem(&title_item);

            let version_item = create_item(mtm, concat!("v", env!("CARGO_PKG_VERSION")), None);
            version_item.setEnabled(false);
            menu.addItem(&version_item);

            menu.addItem(&NSMenuItem::separatorItem(mtm));

            let settings_item = create_item(mtm, "Settings...", Some(sel!(showSettingsWindow:)));
            menu.addItem(&settings_item);

            menu.addItem(&NSMenuItem::separatorItem(mtm));

            let quit_item = create_item(mtm, "Quit", Some(sel!(terminate:)));
            menu.addItem(&quit_item);

            let location = event.locationInWindow();
            menu.popUpMenuPositioningItem_atLocation_inView(
                None,
                location,
                Some(self),
            );
        }

        #[unsafe(method(showSettingsWindow:))]
        fn show_settings_window(&self, _sender: &NSMenuItem) {
            let mtm = self.mtm();
            let app = NSApp(mtm);
            if let Some(delegate) = app.delegate() {
                if let Ok(app_delegate) = delegate.downcast::<AppDelegate>() {
                    app_delegate.show_settings_window();
                }
            }
        }
    }
);

impl WorkspacesStackView {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm);
        let this: Retained<Self> = unsafe { msg_send![this, init] };

        this.setAlignment(NSLayoutAttribute::CenterY);

        this
    }
}
