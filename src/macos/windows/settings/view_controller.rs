use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::Sel;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSBox, NSBoxType, NSButton, NSButtonType, NSControlStateValueOff, NSControlStateValueOn,
    NSFont, NSLayoutAttribute, NSStackView, NSTextField, NSUserInterfaceLayoutOrientation, NSView,
    NSViewController, NSWindow,
};
use objc2_foundation::{MainThreadMarker, NSEdgeInsets, NSObjectProtocol, NSString};

define_class!(
    #[unsafe(super = NSViewController)]
    #[thread_kind = MainThreadOnly]
    #[ivars = SettingsViewControllerIvars]
    #[derive(Debug)]
    pub struct SettingsViewController;

    unsafe impl NSObjectProtocol for SettingsViewController {}

    impl SettingsViewController {
        #[unsafe(method(saveClicked:))]
        fn save_clicked(&self, _sender: &NSButton) {
            if let Err(e) = self.save_config() {
                tracing::error!("Failed to save config: {e}");
            }
            self.close_window();
        }

        #[unsafe(method(cancelClicked:))]
        fn cancel_clicked(&self, _sender: &NSButton) {
            self.close_window();
        }
    }
);

#[derive(Debug)]
pub struct SettingsViewControllerIvars {
    config: RefCell<crate::config::Config>,
    show_layout_button_checkbox: RefCell<Option<Retained<NSButton>>>,
}

impl SettingsViewControllerIvars {
    fn new(config: crate::config::Config) -> Self {
        Self {
            config: RefCell::new(config),
            show_layout_button_checkbox: RefCell::new(None),
        }
    }
}

impl SettingsViewController {
    pub fn new(mtm: MainThreadMarker, config: crate::config::Config) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(SettingsViewControllerIvars::new(config));
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };

        this.setup_ui();

        this
    }

    fn setup_ui(&self) {
        let main_stack = self.create_vstack();
        main_stack.setEdgeInsets(NSEdgeInsets {
            top: 16.0,
            left: 16.0,
            bottom: 16.0,
            right: 16.0,
        });

        main_stack.addArrangedSubview(&self.create_header("Global Settings"));
        main_stack.addArrangedSubview(&self.create_global_settings_ui());
        main_stack.addArrangedSubview(&self.create_separator());
        main_stack.addArrangedSubview(&self.create_action_buttons_ui());

        self.setView(&main_stack);
    }

    fn create_vstack(&self) -> Retained<NSStackView> {
        let mtm = self.mtm();

        let main_stack = NSStackView::new(mtm);
        main_stack.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        main_stack.setAlignment(NSLayoutAttribute::Leading);
        main_stack.setSpacing(12.0);

        main_stack
    }

    fn create_hstack(&self) -> Retained<NSStackView> {
        let mtm = self.mtm();

        let main_stack = NSStackView::new(mtm);
        main_stack.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
        main_stack.setSpacing(8.0);

        main_stack
    }

    fn create_separator(&self) -> Retained<NSBox> {
        let mtm = self.mtm();
        let separator = NSBox::new(mtm);
        separator.setBoxType(NSBoxType::Separator);
        separator
    }

    fn create_header(&self, title: &str) -> Retained<NSTextField> {
        let mtm = self.mtm();
        let header = NSTextField::labelWithString(&NSString::from_str(title), mtm);
        header.setFont(Some(&*NSFont::boldSystemFontOfSize(16.0)));
        header
    }

    fn create_global_settings_ui(&self) -> Retained<NSStackView> {
        let mtm = self.mtm();

        let vstack = self.create_vstack();

        let checkbox = NSButton::new(mtm);
        checkbox.setButtonType(NSButtonType::Switch);
        checkbox.setTitle(&NSString::from_str("Show layout button"));

        let config = self.ivars().config.borrow();
        checkbox.setState(if config.show_layout_button {
            NSControlStateValueOn
        } else {
            NSControlStateValueOff
        });

        vstack.addArrangedSubview(&checkbox);

        *self.ivars().show_layout_button_checkbox.borrow_mut() = Some(checkbox);

        vstack
    }

    fn create_action_button(&self, title: &str, action: Sel) -> Retained<NSButton> {
        let mtm = self.mtm();

        let button = NSButton::new(mtm);
        button.setTitle(&NSString::from_str(title));
        unsafe { button.setTarget(Some(self)) };
        unsafe { button.setAction(Some(action)) };

        button
    }

    fn create_action_buttons_ui(&self) -> Retained<NSStackView> {
        let hstack = self.create_hstack();

        let save_button = self.create_action_button("Save", sel!(saveClicked:));
        hstack.addArrangedSubview(&save_button);

        let cancel_button = self.create_action_button("Cancel", sel!(cancelClicked:));
        hstack.addArrangedSubview(&cancel_button);

        hstack
    }

    fn save_config(&self) -> anyhow::Result<()> {
        let mut config = self.ivars().config.borrow_mut();
        if let Some(checkbox) = self.ivars().show_layout_button_checkbox.borrow().as_ref() {
            config.show_layout_button = checkbox.state() == NSControlStateValueOn;
        }
        config.save()?;
        Ok(())
    }

    fn close_window(&self) {
        let view: Retained<NSView> = self.view();
        if let Some(window) = view.window() {
            let window: &NSWindow = &window;
            window.orderOut(None);
        }
    }
}
