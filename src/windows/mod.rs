use winit::event_loop::EventLoop;

use crate::windows::app::{App, AppMessage};

mod app;
mod egui_glue;
mod message_window;
mod taskbar;
mod tray_icon;
mod utils;
mod widgets;
#[allow(clippy::module_inception)]
mod windows;

pub fn run() -> anyhow::Result<()> {
    let evl = EventLoop::<AppMessage>::with_user_event().build()?;

    let proxy = evl.create_proxy();
    muda::MenuEvent::set_event_handler(Some(move |e| {
        if let Err(e) = proxy.send_event(AppMessage::MenuEvent(e)) {
            tracing::error!("Failed to send `AppMessage::MenuEvent`: {e}")
        }
    }));

    let mut app = App::new(evl.create_proxy())?;
    evl.run_app(&mut app)?;

    Ok(())
}
