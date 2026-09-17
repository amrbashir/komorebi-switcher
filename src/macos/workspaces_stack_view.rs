use std::cell::OnceCell;

use objc2::rc::Retained;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly, Message};
use objc2_app_kit::{
	NSApp, NSButton, NSEvent, NSLayoutAttribute, NSMenu, NSMenuItem, NSStackView, NSView,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSString};

use super::AppDelegate;

#[derive(Debug, Default)]
pub struct WorkspacesStackViewIvars {
	context_menu: OnceCell<Retained<NSMenu>>,
}

define_class!(
	#[unsafe(super = NSStackView)]
	#[thread_kind = MainThreadOnly]
	#[ivars = WorkspacesStackViewIvars]
	pub struct WorkspacesStackView;

	impl WorkspacesStackView {
		/// macOS delivers clicks on a status item at the center of the item
		/// instead of where the click actually happened, so the event location
		/// can't tell us which of our buttons was pressed. Claim the whole
		/// strip for ourselves and route clicks in `mouseDown:` instead, so no
		/// button ever acts on the bogus location.
		#[unsafe(method_id(hitTest:))]
		fn hit_test(&self, point: NSPoint) -> Option<Retained<NSView>> {
			let frame = self.frame();

			let inside = point.x >= frame.origin.x
				&& point.x < frame.origin.x + frame.size.width
				&& point.y >= frame.origin.y
				&& point.y < frame.origin.y + frame.size.height;

			let this: &NSView = self;
			inside.then(|| this.retain())
		}

		#[unsafe(method(mouseDown:))]
		fn mouse_down(&self, event: &NSEvent) {
			if let Some(button) = self.button_under_cursor() {
				unsafe { button.performClick(None) };
			}
		}

		#[unsafe(method(rightMouseDown:))]
		fn right_mouse_down(&self, event: &NSEvent) {
			self.show_or_create_context_menu(event);
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
	pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
		let this = Self::alloc(mtm).set_ivars(WorkspacesStackViewIvars::default());
		let this: Retained<Self> = unsafe { msg_send![super(this), init] };

		this.setAlignment(NSLayoutAttribute::CenterY);

		this
	}

	/// The button the mouse cursor is currently over.
	///
	/// The cursor is still at the real click location even though the event
	/// isn't, so it is what decides which button was pressed. Returns `None`
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

	fn show_or_create_context_menu(&self, event: &NSEvent) {
		if self.ivars().context_menu.get().is_none() {
			self.create_context_menu();
		}

		let Some(menu) = self.ivars().context_menu.get() else {
			return;
		};

		let location = event.locationInWindow();
		menu.popUpMenuPositioningItem_atLocation_inView(None, location, Some(self));
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
