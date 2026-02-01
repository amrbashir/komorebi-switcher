use std::cell::{OnceCell, RefCell};

use dispatch::Queue;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, DefinedClass, MainThreadOnly};
use objc2_app_kit::{
    NSApp, NSApplication, NSApplicationActivationPolicy, NSApplicationDelegate, NSStatusBar,
    NSStatusItem, NSTextAlignment, NSTextField, NSUserInterfaceLayoutOrientation,
    NSVariableStatusItemLength, NSView,
};
use objc2_foundation::{
    ns_string, MainThreadMarker, NSNotification, NSObject, NSObjectProtocol, NSPoint, NSRect,
    NSSize,
};

use self::workspace_button::WorkspaceButton;
use self::workspaces_stack_view::WorkspacesStackView;
use crate::config::Config;
use crate::macos::layout_button::LayoutButton;
use crate::macos::windows::settings::SettingsWindowController;

mod layout_button;
mod windows;
mod workspace_button;
mod workspaces_stack_view;

#[derive(Default)]
pub struct AppDelegateIvars {
    ns_status_item: OnceCell<Retained<NSStatusItem>>,
    ns_stack_view: OnceCell<Retained<WorkspacesStackView>>,
    buttons: RefCell<Vec<Retained<NSView>>>,
    config: OnceCell<Config>,
    settings_window: OnceCell<Retained<windows::settings::SettingsWindowController>>,
}

define_class!(
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = AppDelegateIvars]
    pub struct AppDelegate;

    unsafe impl NSObjectProtocol for AppDelegate {}

    unsafe impl NSApplicationDelegate for AppDelegate {
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, notification: &NSNotification) {
            let mtm = self.mtm();

            let app = notification
                .object()
                .unwrap()
                .downcast::<NSApplication>()
                .unwrap();

            app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

            #[allow(deprecated)]
            #[cfg(debug_assertions)]
            app.activateIgnoringOtherApps(true);

            let komorebi_state = crate::komorebi::read_state().unwrap_or_default();

            let ns_status_bar = NSStatusBar::systemStatusBar();
            let ns_status_item = ns_status_bar.statusItemWithLength(NSVariableStatusItemLength);

            let stack_view = {
                let stack = WorkspacesStackView::new(mtm);
                stack.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
                stack.setSpacing(2.0);
                stack
            };

            if let Some(btn) = ns_status_item.button(mtm) {
                btn.addSubview(&stack_view);
            }

            let _ = self.ivars().ns_status_item.set(ns_status_item);
            let _ = self.ivars().ns_stack_view.set(stack_view);

            let config = Config::load().unwrap_or_default();
            let _ = self.ivars().config.set(config);

            self.update_workspace_buttons(komorebi_state);

            std::thread::spawn(|| {
                crate::komorebi::listen_for_state(|new_state| {
                    Queue::main().exec_async(|| AppDelegate::dispatch_new_state(new_state));
                })
            });
        }
    }
);

impl AppDelegate {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(AppDelegateIvars::default());
        unsafe { msg_send![super(this), init] }
    }

    fn dispatch_new_state(state: crate::komorebi::State) {
        let mtm = MainThreadMarker::new().unwrap();
        let app = NSApp(mtm);
        let delegate = app.delegate().unwrap();
        if let Ok(delegate) = delegate.downcast::<Self>() {
            delegate.update_workspace_buttons(state);
        }
    }

    fn show_or_create_settings_window(&self) {
        if let Some(existing) = self.ivars().settings_window.get() {
            existing.show();
            return;
        }

        let mtm = self.mtm();
        let config = self.ivars().config.get().unwrap().clone();

        let window_controller = SettingsWindowController::new(mtm, config);

        let _ = self.ivars().settings_window.set(window_controller);
        self.ivars().settings_window.get().unwrap().show();
    }

    fn update_workspace_buttons(&self, state: crate::komorebi::State) {
        let mtm = self.mtm();
        let stack_view = self.ivars().ns_stack_view.get().unwrap();
        let mut views = self.ivars().buttons.borrow_mut();
        let config = self.ivars().config.get().unwrap();

        for button in views.iter() {
            button.removeFromSuperview();
        }

        views.clear();

        let Some(monitor) = state.monitors.first() else {
            return;
        };

        for workspace in &monitor.workspaces {
            let workspace_button = WorkspaceButton::new(mtm, workspace);
            stack_view.addArrangedSubview(&workspace_button);
            views.push(workspace_button.downcast().unwrap());
        }

        if config.show_layout_button {
            if let Some(focused_ws) = monitor.focused_workspace() {
                let separator = NSTextField::labelWithString(ns_string!("|"), mtm);
                separator.setAlignment(NSTextAlignment::Center);
                stack_view.addArrangedSubview(&separator);
                views.push(separator.downcast().unwrap());

                let layout_button = LayoutButton::new(mtm, focused_ws);
                stack_view.addArrangedSubview(&layout_button);
                views.push(layout_button.downcast().unwrap());
            }
        }

        let ns_status_item = self.ivars().ns_status_item.get().unwrap();
        if let Some(btn) = ns_status_item.button(mtm) {
            let fitting_size = stack_view.fittingSize();
            let size = NSSize::new(fitting_size.width, WorkspaceButton::HEIGHT);
            let frame = NSRect::new(NSPoint::new(0.0, 0.0), size);
            stack_view.setFrame(frame);
            btn.setFrame(frame);
        }
    }
}

pub fn run() -> anyhow::Result<()> {
    let mtm = MainThreadMarker::new().unwrap();

    let app = NSApplication::sharedApplication(mtm);
    let delegate = AppDelegate::new(mtm);
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

    app.run();

    Ok(())
}
