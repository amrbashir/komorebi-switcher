use objc2::rc::Retained;
use objc2::{define_class, msg_send, sel, MainThreadOnly};
use objc2_app_kit::{NSButton, NSColor, NSEvent, NSFont};
use objc2_foundation::{MainThreadMarker, NSObjectProtocol, NSString};

use crate::komorebi::CycleDirection;

define_class!(
	/// A status-bar button that cycles the focused workspace's layout.
	#[unsafe(super = NSButton)]
	#[thread_kind = MainThreadOnly]
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
			let layer = self.layer().unwrap();
			let hover_color = NSColor::colorWithWhite_alpha(1.0, 0.1).CGColor();
			let _: () = unsafe { msg_send![&layer, setBackgroundColor: &*hover_color] };
		}

		#[unsafe(method(mouseExited:))]
		fn mouse_exited(&self, _event: &NSEvent) {
			let layer = self.layer().unwrap();
			let clear_color = NSColor::clearColor().CGColor();
			let _: () = unsafe { msg_send![&layer, setBackgroundColor: &*clear_color] };
		}
	}
);

impl LayoutButton {
	pub fn new(
		mtm: MainThreadMarker,
		workspace: &crate::komorebi::Workspace,
		font: Option<&NSFont>,
	) -> Retained<Self> {
		// Create button
		// SAFETY: The signature of `NSButton`'s `init` method is correct.
		let this: Retained<Self> = unsafe { msg_send![Self::alloc(mtm), init] };

		// Configure button
		this.setTitle(&NSString::from_str(&workspace.layout));
		if let Some(font) = font {
			this.setFont(Some(font));
		}

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

		this
	}
}
