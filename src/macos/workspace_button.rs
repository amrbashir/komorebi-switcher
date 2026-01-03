use std::cell::{Cell, OnceCell, RefCell};

use objc2::rc::Retained;
use objc2::{define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSButton, NSColor, NSEvent, NSTrackingArea, NSTrackingAreaOptions, NSView};
use objc2_foundation::{MainThreadMarker, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};

#[derive(Debug)]
pub struct WorkspaceButtonIvars {
    workspace: crate::komorebi::Workspace,
    indicator: OnceCell<Retained<NSView>>,
    is_hovering: Cell<bool>,
    tracking_area: RefCell<Option<Retained<NSTrackingArea>>>,
}

impl WorkspaceButtonIvars {
    fn new(workspace: crate::komorebi::Workspace) -> Self {
        Self {
            workspace,
            indicator: OnceCell::new(),
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
    #[ivars = WorkspaceButtonIvars]
    #[derive(Debug)]
    pub struct WorkspaceButton;

    unsafe impl NSObjectProtocol for WorkspaceButton {}

    impl WorkspaceButton {
        #[unsafe(method(buttonClicked:))]
        fn button_clicked(&self, _sender: &NSButton) {
            crate::komorebi::change_workspace(0, self.ivars().workspace.index);
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            self.ivars().is_hovering.set(true);
            self.setWantsLayer(true);
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

impl WorkspaceButton {
    const INDICATOR_SIZE: f64 = 4.0;

    pub fn new(mtm: MainThreadMarker, workspace: &crate::komorebi::Workspace) -> Retained<Self> {
        // Create button
        let this = Self::alloc(mtm).set_ivars(WorkspaceButtonIvars::new(workspace.clone()));
        // SAFETY: The signature of `NSButton`'s `init` method is correct.
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };

        // Configure button
        this.setTitle(&NSString::from_str(&workspace.name));
        this.setTag(workspace.index as isize);

        // Set up action handler
        unsafe { this.setTarget(Some(&this)) };
        unsafe { this.setAction(Some(sel!(buttonClicked:))) };

        // Make button transparent
        this.setBordered(false);
        this.setWantsLayer(true);
        let layer = this.layer().unwrap();
        let _: () = unsafe { msg_send![&layer, setCornerRadius: 4.0] };

        // Add size constraints for padding (since bezel is removed)
        let width_constraint = this
            .widthAnchor()
            .constraintGreaterThanOrEqualToConstant(32.0);
        let height_constraint = this
            .heightAnchor()
            .constraintGreaterThanOrEqualToConstant(24.0);
        width_constraint.setActive(true);
        height_constraint.setActive(true);

        // Set background color based on focus state
        let bg_color = if workspace.focused {
            NSColor::colorWithWhite_alpha(1.0, 0.1).CGColor()
        } else {
            NSColor::clearColor().CGColor()
        };
        let _: () = unsafe { msg_send![&layer, setBackgroundColor: &*bg_color] };

        // Create indicator view
        let indicator = NSView::new(mtm);

        // Make it round
        indicator.setWantsLayer(true);
        let layer = indicator.layer().unwrap();
        let _: () = unsafe { msg_send![&layer, setCornerRadius: 2.0] };

        // Set indicator color and visibility based on workspace state
        if workspace.focused {
            let blue = NSColor::systemBlueColor().CGColor();
            let _: () = unsafe { msg_send![&layer, setBackgroundColor: &*blue] };
            indicator.setHidden(false);
        } else if !workspace.is_empty {
            let light_gray = NSColor::lightGrayColor().CGColor();
            let _: () = unsafe { msg_send![&layer, setBackgroundColor: &*light_gray] };
            indicator.setHidden(false);
        } else {
            indicator.setHidden(true);
        }

        // Position indicator at bottom center of button
        let btn_fitting_size = this.fittingSize();
        let x = (btn_fitting_size.width - Self::INDICATOR_SIZE) / 2.0;
        let y = btn_fitting_size.height - Self::INDICATOR_SIZE;
        let position = NSPoint::new(x, y);
        let size = NSSize::new(Self::INDICATOR_SIZE, Self::INDICATOR_SIZE);
        indicator.setFrame(NSRect::new(position, size));

        // Add indicator to button
        this.addSubview(&indicator);

        // Store indicator in ivars
        let _ = this.ivars().indicator.set(indicator);

        this
    }
}
