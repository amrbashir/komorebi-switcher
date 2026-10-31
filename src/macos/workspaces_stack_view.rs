use std::cell::OnceCell;
use std::ptr::NonNull;

use block2::RcBlock;
use objc2::rc::{Retained, Weak};
use objc2::runtime::AnyObject;
use objc2::{define_class, msg_send, sel, AnyThread, DefinedClass, MainThreadOnly, Message};
use objc2_app_kit::{
	NSApp, NSBackingStoreType, NSButton, NSColor, NSEvent, NSEventMask, NSEventModifierFlags,
	NSMenu, NSMenuItem, NSPanel, NSStackView, NSStatusWindowLevel, NSTrackingArea,
	NSTrackingAreaOptions, NSView, NSViewFrameDidChangeNotification, NSWindowCollectionBehavior,
	NSWindowDidMoveNotification, NSWindowDidResizeNotification, NSWindowStyleMask,
};
use objc2_foundation::{
	MainThreadMarker, NSNotification, NSNotificationCenter, NSObjectProtocol, NSRect, NSString,
};

use super::AppDelegate;

#[derive(Default)]
pub struct WorkspacesStackViewIvars {
	context_menu: OnceCell<Retained<NSMenu>>,
	input_panel: OnceCell<Retained<StatusBarInputPanel>>,
	modifier_monitors: OnceCell<Vec<Retained<AnyObject>>>,
}

impl Drop for WorkspacesStackViewIvars {
	fn drop(&mut self) {
		if let Some(monitors) = self.modifier_monitors.get() {
			for monitor in monitors {
				unsafe { NSEvent::removeMonitor(monitor) };
			}
		}

		let Some(panel) = self.input_panel.get() else {
			return;
		};

		if let Some(surface) = panel.contentView() {
			unsafe { NSNotificationCenter::defaultCenter().removeObserver(&surface) };
		}

		panel.close();
	}
}

define_class!(
	#[unsafe(super = NSStackView)]
	#[thread_kind = MainThreadOnly]
	#[ivars = WorkspacesStackViewIvars]
	pub struct WorkspacesStackView;

	impl WorkspacesStackView {
		#[unsafe(method(viewDidMoveToWindow))]
		fn view_did_move_to_window(&self) {
			let _: () = unsafe { msg_send![super(self), viewDidMoveToWindow] };

			// Create the panel if not created already.
			if self.window().is_some() && self.ivars().input_panel.get().is_none() {
				let panel = StatusBarInputPanel::new(self);
				self.observe_modifier_flags(&panel);
				let _ = self.ivars().input_panel.set(panel);
			}

			self.observe_status_item_geometry();
			self.synchronize_input_panel();
		}

		#[unsafe(method(mouseDown:))]
		fn mouse_down(&self, _event: &NSEvent) {
			if let Some(button) = self.button_under_cursor() {
				unsafe { button.performClick(None) };
			}
		}

		#[unsafe(method(rightMouseDown:))]
		fn right_mouse_down(&self, event: &NSEvent) {
			let view = event.window(self.mtm()).and_then(|window| window.contentView());
			let view = view.as_deref().unwrap_or(self);
			self.show_or_create_context_menu(view, event);
		}

		#[unsafe(method(mouseMoved:))]
		fn mouse_moved(&self, event: &NSEvent) {
			// The input view receives tracking events instead of the buttons.
			// Clear the previous hover first, then apply it to the button now
			// under the cursor so moving between buttons doesn't leave stale highlights.
			self.mouseExited(event);
			self.mouseEntered(event);
		}

		#[unsafe(method(mouseEntered:))]
		fn mouse_entered(&self, event: &NSEvent) {
			if let Some(button) = self.button_under_cursor() {
				button.mouseEntered(event);
			}
		}

		#[unsafe(method(mouseExited:))]
		fn mouse_exited(&self, event: &NSEvent) {
			for view in self.subviews() {
				if let Ok(button) = view.downcast::<NSButton>() {
					button.mouseExited(event);
				}
			}
		}

		#[unsafe(method(showSettingsWindow:))]
		fn show_settings_window(&self, _sender: &NSMenuItem) {
			let mtm = self.mtm();
			let app = NSApp(mtm);

			let Some(delegate) = app.delegate() else {
				return;
			};

			let Ok(app_delegate) = delegate.downcast::<AppDelegate>() else {
				return;
			};

			app_delegate.show_or_create_settings_window();
		}
	}
);

impl WorkspacesStackView {
	fn status_item_window_changed(&self, _notification: &NSNotification) {
		self.synchronize_input_panel();
	}

	pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
		let this = Self::alloc(mtm).set_ivars(WorkspacesStackViewIvars::default());
		unsafe { msg_send![super(this), init] }
	}

	fn observe_modifier_flags(&self, panel: &StatusBarInputPanel) {
		let weak_panel = Weak::new(panel);
		let update_passthrough = move |event: NonNull<NSEvent>| {
			if let Some(panel) = weak_panel.load() {
				// AppKit keeps the event alive for the duration of the monitor callback.
				let event = unsafe { event.as_ref() };
				panel.update_click_passthrough(event.modifierFlags());
			}
		};

		// Global monitors receive other apps' events; local monitors receive ours.
		// Observe modifier changes only, so Command-drag needs no keyboard capture.
		let global_handler = RcBlock::new(update_passthrough.clone());
		let global = NSEvent::addGlobalMonitorForEventsMatchingMask_handler(
			NSEventMask::FlagsChanged,
			&global_handler,
		);
		let local_handler = RcBlock::new(move |event: NonNull<NSEvent>| {
			update_passthrough(event);
			event.as_ptr()
		});
		// Return the original event unchanged so normal event dispatch continues.
		let local = unsafe {
			NSEvent::addLocalMonitorForEventsMatchingMask_handler(
				NSEventMask::FlagsChanged,
				&local_handler,
			)
		};
		let monitors = [global, local].into_iter().flatten().collect();
		let _ = self.ivars().modifier_monitors.set(monitors);
	}

	fn observe_status_item_geometry(&self) {
		let Some(panel) = self.ivars().input_panel.get() else {
			return;
		};
		let Some(surface) = panel.contentView() else {
			return;
		};

		let Some(btn) = (unsafe { self.superview() }) else {
			return;
		};
		let Some(source_window) = self.window() else {
			return;
		};

		let center = NSNotificationCenter::defaultCenter();

		// viewDidMoveToWindow can run again when the stack is reattached or moves
		// to another window. Remove old subscriptions before registering the current
		// host to avoid duplicate callbacks and notifications from the previous host.
		unsafe { center.removeObserver(&surface) };

		unsafe {
			center.addObserver_selector_name_object(
				&surface,
				sel!(statusItemWindowChanged:),
				Some(NSViewFrameDidChangeNotification),
				Some(&btn),
			);
		}

		// Keep the input panel aligned when the hosting window moves or resizes.
		unsafe {
			for name in [NSWindowDidMoveNotification, NSWindowDidResizeNotification] {
				center.addObserver_selector_name_object(
					&surface,
					sel!(statusItemWindowChanged:),
					Some(name),
					Some(&source_window),
				);
			}
		}
	}

	fn synchronize_input_panel(&self) {
		let Some(panel) = self.ivars().input_panel.get() else {
			return;
		};

		let Some(btn) = (unsafe { self.superview() }) else {
			panel.orderOut(None);
			return;
		};
		let Some(source_window) = self.window() else {
			panel.orderOut(None);
			return;
		};

		// Convert the button's bounds to window coordinates, then screen coordinates,
		// so the separate input panel can cover the native status button.
		let rect = btn.convertRect_toView(btn.bounds(), None);
		let mut rect = source_window.convertRectToScreen(rect);

		// Testing showed the native status item receives clicks one point beyond
		// the button's bounds. Cover this invisible clickable margin so edge clicks
		// cannot reach the native item and trigger a highlight of the entire strip.
		rect.origin.x -= 1.0;
		rect.origin.y -= 1.0;
		rect.size.width += 2.0;
		rect.size.height += 2.0;

		if panel.frame() != rect {
			panel.setFrame_display(rect, true);
		}
		if !panel.isVisible() {
			panel.orderFrontRegardless();
		}
	}

	/// The button the mouse cursor is currently over.
	///
	/// Use the cursor position to identify the button. Returns `None`
	/// when the cursor is over the spacing between two buttons.
	fn button_under_cursor(&self) -> Option<Retained<NSButton>> {
		let window = self.window()?;
		let cursor = NSEvent::mouseLocation();
		let point = self.convertPoint_fromView(window.convertPointFromScreen(cursor), None);

		// The strip is laid out horizontally and every button spans its full
		// height, so the x coordinate alone identifies the button.
		self.subviews().iter().find_map(|view| {
			let frame = view.frame();
			let inside = point.x >= frame.origin.x && point.x < frame.origin.x + frame.size.width;

			inside
				.then(|| view.retain().downcast::<NSButton>().ok())
				.flatten()
		})
	}

	fn show_or_create_context_menu(&self, view: &NSView, event: &NSEvent) {
		if self.ivars().context_menu.get().is_none() {
			self.create_context_menu();
		}

		let Some(menu) = self.ivars().context_menu.get() else {
			return;
		};

		let location = view.convertPoint_fromView(event.locationInWindow(), None);
		menu.popUpMenuPositioningItem_atLocation_inView(None, location, Some(view));
	}

	fn create_context_menu(&self) {
		let mtm = self.mtm();
		let menu = NSMenu::new(mtm);

		let settings_item = self.create_menu_item("Settings", Some(sel!(showSettingsWindow:)));
		unsafe { settings_item.setTarget(Some(self)) };
		menu.addItem(&settings_item);

		menu.addItem(&NSMenuItem::separatorItem(mtm));

		#[cfg(debug_assertions)]
		let title = concat!(env!("CARGO_PKG_NAME"), " (debug)");
		#[cfg(not(debug_assertions))]
		let title = env!("CARGO_PKG_NAME");
		let title_item = self.create_menu_item(title, None);
		menu.addItem(&title_item);

		let version_item = self.create_menu_item(concat!("v", env!("CARGO_PKG_VERSION")), None);
		menu.addItem(&version_item);

		menu.addItem(&NSMenuItem::separatorItem(mtm));

		let quit_item = self.create_menu_item("Quit", Some(sel!(terminate:)));
		menu.addItem(&quit_item);

		let _ = self.ivars().context_menu.set(menu);
	}

	fn create_menu_item(
		&self,
		title: &str,
		action: Option<objc2::runtime::Sel>,
	) -> Retained<NSMenuItem> {
		let mtm = self.mtm();
		unsafe {
			NSMenuItem::initWithTitle_action_keyEquivalent(
				NSMenuItem::alloc(mtm),
				&NSString::from_str(title),
				action,
				&NSString::from_str(""),
			)
		}
	}
}

struct StatusBarInputViewIvars {
	stack: Weak<WorkspacesStackView>,
}

define_class!(
	/// Receives clicks and hover events in `StatusBarInputPanel` and manually
	/// forwards them to `WorkspacesStackView`, which stays inside the native
	/// status item to preserve its appearance.
	///
	/// Holds a weak reference because the stack owns the panel and its content view.
	#[unsafe(super = NSView)]
	#[thread_kind = MainThreadOnly]
	#[ivars = StatusBarInputViewIvars]
	struct StatusBarInputView;

	unsafe impl NSObjectProtocol for StatusBarInputView {}

	impl StatusBarInputView {
		#[unsafe(method(acceptsFirstMouse:))]
		fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool { true }

		#[unsafe(method(mouseDown:))]
		fn mouse_down(&self, event: &NSEvent) {
			if let Some(stack) = self.ivars().stack.load() { stack.mouseDown(event); }
		}

		#[unsafe(method(rightMouseDown:))]
		fn right_mouse_down(&self, event: &NSEvent) {
			if let Some(stack) = self.ivars().stack.load() { stack.rightMouseDown(event); }
		}

		#[unsafe(method(mouseMoved:))]
		fn mouse_moved(&self, event: &NSEvent) {
			if let Some(stack) = self.ivars().stack.load() { stack.mouseMoved(event); }
		}

		#[unsafe(method(mouseEntered:))]
		fn mouse_entered(&self, event: &NSEvent) {
			if let Some(stack) = self.ivars().stack.load() { stack.mouseEntered(event); }
		}

		#[unsafe(method(mouseExited:))]
		fn mouse_exited(&self, event: &NSEvent) {
			if let Some(stack) = self.ivars().stack.load() { stack.mouseExited(event); }
		}

		#[unsafe(method(statusItemWindowChanged:))]
		fn status_item_window_changed(&self, notification: &NSNotification) {
			if let Some(stack) = self.ivars().stack.load() {
				stack.status_item_window_changed(notification);
			}
		}
	}
);

impl StatusBarInputView {
	fn new(stack: &WorkspacesStackView) -> Retained<Self> {
		let ivars = StatusBarInputViewIvars {
			stack: Weak::new(stack),
		};

		let surface = Self::alloc(stack.mtm()).set_ivars(ivars);
		let surface: Retained<Self> = unsafe { msg_send![super(surface), init] };

		let tracking_area = unsafe {
			NSTrackingArea::initWithRect_options_owner_userInfo(
				NSTrackingArea::alloc(),
				NSRect::ZERO,
				NSTrackingAreaOptions::MouseEnteredAndExited
					| NSTrackingAreaOptions::MouseMoved
					| NSTrackingAreaOptions::ActiveAlways
					| NSTrackingAreaOptions::InVisibleRect,
				Some(&surface),
				None,
			)
		};
		surface.addTrackingArea(&tracking_area);

		surface
	}
}

define_class!(
	/// Transparent, nonactivating panel positioned over the workspace strip.
	///
	/// Stops clicks from reaching the native status item's window, preventing
	/// macOS from starting highlight tracking for the entire strip. Its
	/// `StatusBarInputView` manually forwards those events to the stack view.
	///
	/// The panel never becomes the key or main window, preserving keyboard focus.
	///
	/// Command-drag passes through for native status-item reordering.
	#[unsafe(super = NSPanel)]
	#[thread_kind = MainThreadOnly]
	struct StatusBarInputPanel;

	unsafe impl NSObjectProtocol for StatusBarInputPanel {}

	impl StatusBarInputPanel {
		#[unsafe(method(canBecomeKeyWindow))]
		fn can_become_key_window(&self) -> bool { false }

		#[unsafe(method(canBecomeMainWindow))]
		fn can_become_main_window(&self) -> bool { false }
	}
);

impl StatusBarInputPanel {
	fn update_click_passthrough(&self, flags: NSEventModifierFlags) {
		let command_pressed = flags.contains(NSEventModifierFlags::Command);
		self.setIgnoresMouseEvents(command_pressed);
	}

	/// macOS highlights the native status item before delivering its click.
	/// Receive input in a transparent, nonactivating panel while keeping the
	/// entire button strip inside the original status item.
	fn new(stack: &WorkspacesStackView) -> Retained<Self> {
		let mtm = stack.mtm();
		let panel = Self::alloc(mtm);
		let style_mask = NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel;
		let panel: Retained<Self> = unsafe {
			msg_send![
				panel,
				initWithContentRect: NSRect::ZERO,
				styleMask: style_mask,
				backing: NSBackingStoreType::Buffered,
				defer: false,
			]
		};
		panel.setOpaque(false);
		// A completely clear panel passes clicks through, including clicks
		// in the gaps. Keep this surface just opaque enough to receive input.
		panel.setBackgroundColor(Some(&NSColor::colorWithWhite_alpha(1.0, 0.001)));
		panel.setHasShadow(false);
		panel.setLevel(NSStatusWindowLevel + 1);
		panel.setHidesOnDeactivate(false);
		panel.setAcceptsMouseMovedEvents(true);
		panel.setCollectionBehavior(
			NSWindowCollectionBehavior::CanJoinAllSpaces
				| NSWindowCollectionBehavior::Stationary
				| NSWindowCollectionBehavior::FullScreenAuxiliary
				| NSWindowCollectionBehavior::IgnoresCycle,
		);

		let surface = StatusBarInputView::new(stack);
		panel.setContentView(Some(&surface));

		panel
	}
}
