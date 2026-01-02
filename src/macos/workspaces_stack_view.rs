use objc2::rc::Retained;
use objc2::{define_class, msg_send, sel, MainThreadOnly};
use objc2_app_kit::{NSEvent, NSMenu, NSMenuItem, NSStackView};
use objc2_foundation::{MainThreadMarker, NSString};

define_class!(
    #[unsafe(super = NSStackView)]
    #[thread_kind = MainThreadOnly]
    #[derive(Debug)]
    pub struct WorkspacesStackView;

    impl WorkspacesStackView {
        #[unsafe(method(rightMouseDown:))]
        fn right_mouse_down(&self, event: &NSEvent) {
            let mtm = self.mtm();

            // Create menu
            let menu = NSMenu::new(mtm);

            fn create_item(
                mtm: MainThreadMarker,
                title: &str,
                action: Option<objc2::runtime::Sel>,
            ) -> Retained<NSMenuItem> {
                let item = unsafe {
                    NSMenuItem::initWithTitle_action_keyEquivalent(
                        NSMenuItem::alloc(mtm),
                        &NSString::from_str(title),
                        action,
                        &NSString::from_str(""),
                    )
                };
                item
            }

            // Add title menu item
            let app_name = env!("CARGO_PKG_NAME");
            let title_item = create_item(mtm, app_name, None);
            title_item.setEnabled(false);
            menu.addItem(&title_item);

            // Add version menu item
            let version_item = create_item(mtm, concat!("v", env!("CARGO_PKG_VERSION")), None);
            version_item.setEnabled(false);
            menu.addItem(&version_item);

            // Add separator
            menu.addItem(&NSMenuItem::separatorItem(mtm));

            // Create quit menu item
            let quit_item = create_item(mtm, "Quit", Some(sel!(terminate:)));
            menu.addItem(&quit_item);

            // Show menu at click location
            let location = event.locationInWindow();
            menu.popUpMenuPositioningItem_atLocation_inView(
                None,
                location,
                Some(self),
            );
        }
    }
);

impl WorkspacesStackView {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm);
        // SAFETY: The signature of `NSStackView`'s `init` method is correct.
        unsafe { msg_send![this, init] }
    }
}
