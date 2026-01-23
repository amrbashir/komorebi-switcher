use std::cell::{Cell, RefCell};

use objc2::rc::Retained;
use objc2::{define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSButton, NSColor, NSEvent, NSTrackingArea, NSTrackingAreaOptions};
use objc2_foundation::{MainThreadMarker, NSObjectProtocol, NSString};

use crate::komorebi::CycleDirection;

#[derive(Debug)]
pub struct LayoutButtonIvars {
    workspace: crate::komorebi::Workspace,
    is_hovering: Cell<bool>,
    tracking_area: RefCell<Option<Retained<NSTrackingArea>>>,
}

impl LayoutButtonIvars {
    fn new(workspace: crate::komorebi::Workspace) -> Self {
        Self {
            workspace,
            is_hovering: Cell::new(false),
            tracking_area: RefCell::new(None),
        }
    }
}

define_class!(
    /// A Custom button representing a workspace in the status bar.
    /// Displays an indicator for focused and non-empty workspaces.
    #[unsafe(super = NSButton)]
    #[thread_kind = MainThreadOnly]
    #[ivars = LayoutButtonIvars]
    #[derive(Debug)]
    pub struct LayoutButton;

    unsafe impl NSObjectProtocol for LayoutButton {}

    impl LayoutButton {
        #[unsafe(method(buttonClicked:))]
        fn button_clicked(&self, _sender: &NSButton) {
            crate::komorebi::cycle_layout(CycleDirection::Next);
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            self.ivars().is_hovering.set(true);
            let layer = self.layer().unwrap();
            let hover_color = NSColor::colorWithWhite_alpha(1.0, 0.1).CGColor();
            let _: () = unsafe { msg_send![&layer, setBackgroundColor: &*hover_color] };
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            if self.ivars().workspace.focused {
                return;
            }

            self.ivars().is_hovering.set(false);
            let layer = self.layer().unwrap();
            let clear_color = NSColor::clearColor().CGColor();
            let _: () = unsafe { msg_send![&layer, setBackgroundColor: &*clear_color] };
        }

        #[unsafe(method(updateTrackingAreas))]
        fn update_tracking_areas(&self) {
            // Remove old tracking area if it exists
            if let Some(old_tracking_area) = self.ivars().tracking_area.borrow().as_ref() {
                self.removeTrackingArea(old_tracking_area);
            }

            // Create new tracking area
            let options = NSTrackingAreaOptions::MouseEnteredAndExited
                | NSTrackingAreaOptions::ActiveAlways;
            let tracking_area = unsafe {
                NSTrackingArea::initWithRect_options_owner_userInfo(
                    NSTrackingArea::alloc(),
                    self.bounds(),
                    options,
                    Some(self),
                    None,
                )
            };
            self.addTrackingArea(&tracking_area);

            // Store tracking area
            *self.ivars().tracking_area.borrow_mut() = Some(tracking_area);
        }
    }
);

impl LayoutButton {
    pub fn new(mtm: MainThreadMarker, workspace: &crate::komorebi::Workspace) -> Retained<Self> {
        // Create button
        let this = Self::alloc(mtm).set_ivars(LayoutButtonIvars::new(workspace.clone()));
        // SAFETY: The signature of `NSButton`'s `init` method is correct.
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };

        // Configure button
        this.setTitle(&NSString::from_str(&workspace.layout));

        // Set up action handler
        unsafe { this.setTarget(Some(&this)) };
        unsafe { this.setAction(Some(sel!(buttonClicked:))) };

        // Make button transparent
        this.setBordered(false);
        this.setWantsLayer(true);
        let layer = this.layer().unwrap();
        let _: () = unsafe { msg_send![&layer, setCornerRadius: 4.0] };

        // Add size constraints for padding (since bezel is removed)
        let height_constraint = this
            .heightAnchor()
            .constraintGreaterThanOrEqualToConstant(24.0);
        height_constraint.setActive(true);

        // Set background color based on focus state
        let bg_color = NSColor::clearColor().CGColor();
        let _: () = unsafe { msg_send![&layer, setBackgroundColor: &*bg_color] };

        this
    }
}
