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
               unsafe {
                    NSMenuItem::initWithTitle_action_keyEquivalent(
                        NSMenuItem::alloc(mtm),
                        &NSString::from_str(title),
                        action,
                        &NSString::from_str(""),
                    )
                }
            }

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
