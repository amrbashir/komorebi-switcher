use std::cell::RefCell;

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, sel, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSBox, NSBoxType, NSButton, NSButtonType, NSControlStateValue, NSStackView,
    NSStackViewAlignment, NSStackViewGravity, NSTextField, NSUserInterfaceLayoutOrientation,
};
use objc2_foundation::{MainThreadMarker, NSObject, NSObjectProtocol, NSString};

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = SettingsViewControllerIvars]
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

        this.setup_ui(mtm);

        this
    }

    fn setup_ui(&self, mtm: MainThreadMarker) {
        let main_stack = NSStackView::new(mtm);
        main_stack.setOrientation(NSUserInterfaceLayoutOrientation::Vertical);
        main_stack.setSpacing(12.0);
        main_stack.setAlignment(NSStackViewAlignment::Leading);

        let header = NSTextField::labelWithString(&NSString::from_str("Global Settings"), mtm);
        let font = unsafe {
            objc2_app_kit::NSFont::boldSystemFontOfSize(NSFont::systemFontSizeForControlSize(
                objc2_app_kit::NSControlSize::Regular,
            ))
        };
        header.setFont(Some(&font));
        main_stack.addArrangedSubview(&header);

        let separator = NSBox::boxWithBoxType(NSBoxType::Separator, mtm);
        main_stack.addArrangedSubview(&separator);

        let checkbox_stack = NSStackView::new(mtm);
        checkbox_stack.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
        checkbox_stack.setSpacing(8.0);
        checkbox_stack.setAlignment(NSStackViewAlignment::CenterY);

        let label = NSTextField::labelWithString(&NSString::from_str("Show layout button"), mtm);
        checkbox_stack.addArrangedSubview(&label);

        let checkbox = NSButton::new(mtm);
        checkbox.setButtonType(NSButtonType::Switch);
        checkbox.setTitle(&NSString::from_str(""));
        let config = self.ivars().config.borrow();
        checkbox.setState(if config.show_layout_button {
            NSControlStateValue::On
        } else {
            NSControlStateValue::Off
        });
        *self.ivars().show_layout_button_checkbox.borrow_mut() = Some(checkbox);
        checkbox_stack.addArrangedSubview(&checkbox);

        main_stack.addArrangedSubview(&checkbox_stack);

        let button_stack = NSStackView::new(mtm);
        button_stack.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
        button_stack.setSpacing(12.0);
        button_stack.setAlignment(NSStackViewAlignment::CenterY);
        button_stack.setGravity(NSStackViewGravity::Trailing);

        let save_button = NSButton::new(mtm);
        save_button.setButtonType(NSButtonType::MomentaryPushIn);
        save_button.setTitle(&NSString::from_str("Save"));
        unsafe { save_button.setTarget(Some(&this)) };
        unsafe { save_button.setAction(Some(sel!(saveClicked:))) };
        button_stack.addArrangedSubview(&save_button);

        let cancel_button = NSButton::new(mtm);
        cancel_button.setButtonType(NSButtonType::MomentaryPushIn);
        cancel_button.setTitle(&NSString::from_str("Cancel"));
        unsafe { cancel_button.setTarget(Some(&this)) };
        unsafe { cancel_button.setAction(Some(sel!(cancelClicked:))) };
        button_stack.addArrangedSubview(&cancel_button);

        main_stack.addArrangedSubview(&button_stack);

        self.setContentView(Some(ProtocolObject::from_ref(&*main_stack)));
    }

    fn save_config(&self) -> anyhow::Result<()> {
        let mut config = self.ivars().config.borrow_mut();
        if let Some(checkbox) = self.ivars().show_layout_button_checkbox.borrow().as_ref() {
            config.show_layout_button = checkbox.state() == NSControlStateValue::On;
        }
        config.save()?;
        Ok(())
    }

    fn close_window(&self) {
        if let Some(view) = self.contentView() {
            if let Some(window) = view.window() {
                window.orderOut(None);
            }
        }
    }
}
