use objc2::rc::Retained;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{NSButton, NSColor, NSEvent, NSFont, NSView};
use objc2_foundation::{MainThreadMarker, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};

#[derive(Debug)]
pub struct WorkspaceButtonIvars {
	focused: bool,
}

define_class!(
	/// A Custom button representing a workspace in the status bar.
	/// Displays an indicator for active and busy workspaces.
	#[unsafe(super = NSButton)]
	#[thread_kind = MainThreadOnly]
	#[ivars = WorkspaceButtonIvars]
	#[derive(Debug)]
	pub struct WorkspaceButton;

	unsafe impl NSObjectProtocol for WorkspaceButton {}

	impl WorkspaceButton {
		#[unsafe(method(buttonClicked:))]
		fn button_clicked(&self, _sender: &NSButton) {
			crate::komorebi::change_workspace(0, self.tag() as usize);
		}

		#[unsafe(method(mouseEntered:))]
		fn mouse_entered(&self, _event: &NSEvent) {
			let layer = self.layer().unwrap();
			let hover_color = NSColor::colorWithWhite_alpha(1.0, 0.1).CGColor();
			let _: () = unsafe { msg_send![&layer, setBackgroundColor: &*hover_color] };
		}

		#[unsafe(method(mouseExited:))]
		fn mouse_exited(&self, _event: &NSEvent) {
			if self.ivars().focused {
				return;
			}

			let layer = self.layer().unwrap();
			let clear_color = NSColor::clearColor().CGColor();
			let _: () = unsafe { msg_send![&layer, setBackgroundColor: &*clear_color] };
		}
	}
);

impl WorkspaceButton {
	const INDICATOR_SIZE: f64 = 4.0;
	pub const HEIGHT: f64 = 24.0;

	pub fn new(
		mtm: MainThreadMarker,
		workspace: &crate::komorebi::Workspace,
		font: Option<&NSFont>,
		active_indicator_color: Option<&str>,
		busy_indicator_color: Option<&str>,
	) -> Retained<Self> {
		// Create button
		let this = Self::alloc(mtm).set_ivars(WorkspaceButtonIvars {
			focused: workspace.focused,
		});
		// SAFETY: The signature of `NSButton`'s `init` method is correct.
		let this: Retained<Self> = unsafe { msg_send![super(this), init] };

		// Configure button
		this.setTitle(&NSString::from_str(&workspace.name));
		this.setTag(workspace.index as isize);
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
		let width_constraint = this
			.widthAnchor()
			.constraintGreaterThanOrEqualToConstant(32.0);
		let height_constraint = this
			.heightAnchor()
			.constraintGreaterThanOrEqualToConstant(Self::HEIGHT);
		width_constraint.setActive(true);
		height_constraint.setActive(true);

		// Set background color based on active state
		if workspace.focused {
			let bg_color = NSColor::colorWithWhite_alpha(1.0, 0.1).CGColor();
			let _: () = unsafe { msg_send![&layer, setBackgroundColor: &*bg_color] };
		}

		// Create indicator view
		let indicator = NSView::new(mtm);

		// Make it round
		indicator.setWantsLayer(true);
		let layer = indicator.layer().unwrap();
		let _: () = unsafe { msg_send![&layer, setCornerRadius: 2.0] };

		// Set indicator color and visibility based on workspace state
		if workspace.focused {
			let color = active_indicator_color
				.and_then(crate::macos::utils::ns_color_from_color)
				.unwrap_or_else(NSColor::systemBlueColor)
				.CGColor();
			let _: () = unsafe { msg_send![&layer, setBackgroundColor: &*color] };
		} else if !workspace.is_empty {
			let color = busy_indicator_color
				.and_then(crate::macos::utils::ns_color_from_color)
				.unwrap_or_else(NSColor::lightGrayColor)
				.CGColor();
			let _: () = unsafe { msg_send![&layer, setBackgroundColor: &*color] };
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

		this
	}
}
