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

mod layout_button;
mod workspace_button;
mod workspaces_stack_view;

#[derive(Debug)]
pub struct AppDelegateIvars {
    ns_status_item: OnceCell<Retained<NSStatusItem>>,
    ns_stack_view: OnceCell<Retained<WorkspacesStackView>>,
    buttons: RefCell<Vec<Retained<NSView>>>,
    config: OnceCell<Config>,
}

impl Default for AppDelegateIvars {
    fn default() -> Self {
        Self {
            ns_status_item: OnceCell::new(),
            ns_stack_view: OnceCell::new(),
            buttons: RefCell::new(Vec::new()),
            config: OnceCell::new(),
        }
    }
}

define_class!(
    // SAFETY:
    // - The superclass NSObject does not have any subclassing requirements.
    // - `Delegate` does not implement `Drop`.
    #[unsafe(super = NSObject)]
    #[thread_kind = MainThreadOnly]
    #[ivars = AppDelegateIvars]
    pub struct AppDelegate;

    // SAFETY: `NSObjectProtocol` has no safety requirements.
    unsafe impl NSObjectProtocol for AppDelegate {}

    // SAFETY: `NSApplicationDelegate` has no safety requirements.
    unsafe impl NSApplicationDelegate for AppDelegate {
        // SAFETY: The signature is correct.
        #[unsafe(method(applicationDidFinishLaunching:))]
        fn did_finish_launching(&self, notification: &NSNotification) {
            let mtm = self.mtm();

            let app = notification
                .object()
                .unwrap()
                .downcast::<NSApplication>()
                .unwrap();

            // Set the activation policy to Accessory to hide the dock icon and menu bar.
            app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);

            // Activate the application.
            // Required when launching unbundled (as is done with Cargo).
            #[allow(deprecated)]
            #[cfg(debug_assertions)]
            app.activateIgnoringOtherApps(true);

            let komorebi_state = crate::komorebi::read_state().unwrap_or_default();

            // create status bar item
            let ns_status_bar = NSStatusBar::systemStatusBar();
            let ns_status_item = ns_status_bar.statusItemWithLength(NSVariableStatusItemLength);

            // Create stack view for horizontal button layout
            let stack_view = {
                let stack = WorkspacesStackView::new(mtm);
                stack.setOrientation(NSUserInterfaceLayoutOrientation::Horizontal);
                stack.setSpacing(2.0);
                stack
            };

            // Add stack view to status item button
            if let Some(btn) = ns_status_item.button(mtm) {
                btn.addSubview(&stack_view);
            }

            let _ = self.ivars().ns_status_item.set(ns_status_item);
            let _ = self.ivars().ns_stack_view.set(stack_view);

            // Load config
            let config = Config::load().unwrap_or_default();
            let _ = self.ivars().config.set(config);

            // Create initial workspace buttons
            self.update_workspace_buttons(komorebi_state);

            // Listen for komorebi state changes on a separate thread
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
        // SAFETY: The signature of `NSObject`'s `init` method is correct.
        unsafe { msg_send![super(this), init] }
    }

    fn dispatch_new_state(state: crate::komorebi::State) {
        // SAFETY: This is called on the main thread using `Queue::main()`.
        let mtm = MainThreadMarker::new().unwrap();
        let app = NSApp(mtm);
        // SAFETY: We have set a delegate for the application.
        let delegate = app.delegate().unwrap();
        if let Ok(delegate) = delegate.downcast::<Self>() {
            delegate.update_workspace_buttons(state);
        }
    }

    fn update_workspace_buttons(&self, state: crate::komorebi::State) {
        let mtm = self.mtm();
        // SAFETY: We have initialized these ivars in `did_finish_launching`.
        let stack_view = self.ivars().ns_stack_view.get().unwrap();
        let mut views = self.ivars().buttons.borrow_mut();
        let config = self.ivars().config.get().unwrap();

        // Remove all existing buttons from stack view
        for button in views.iter() {
            button.removeFromSuperview();
        }

        views.clear();

        // Get first monitor (we only support one for now)
        let Some(monitor) = state.monitors.first() else {
            return;
        };

        // Create new buttons for all workspaces
        for workspace in &monitor.workspaces {
            let workspace_button = WorkspaceButton::new(mtm, workspace);
            stack_view.addArrangedSubview(&workspace_button);

            // Store button
            views.push(workspace_button.downcast().unwrap());
        }

        // show layout button for focused workspace
        if config.show_layout_button {
            if let Some(focused_ws) = monitor.focused_workspace() {
                let separator = NSTextField::labelWithString(ns_string!("|"), mtm);
                separator.setAlignment(NSTextAlignment::Center);
                stack_view.addArrangedSubview(&separator);

                // Store separator
                views.push(separator.downcast().unwrap());

                let layout_button = LayoutButton::new(mtm, focused_ws);
                stack_view.addArrangedSubview(&layout_button);

                // Store button
                views.push(layout_button.downcast().unwrap());
            }
        }

        // SAFETY: We have initialized this ivar in `did_finish_launching`.
        let ns_status_item = self.ivars().ns_status_item.get().unwrap();

        // Update status item button frame to match new stack view size
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
    // SAFETY: `run` is the main entry point and is called on the main thread.
    let mtm = MainThreadMarker::new().unwrap();

    let app = NSApplication::sharedApplication(mtm);
    let delegate = AppDelegate::new(mtm);
    app.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));

    app.run();

    Ok(())
}
