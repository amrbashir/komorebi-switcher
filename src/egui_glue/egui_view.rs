use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;

use crate::app::AppMessage;

pub trait EguiView {
    fn handle_app_message(
        &mut self,
        event_loop: &ActiveEventLoop,
        message: &AppMessage,
    ) -> anyhow::Result<()> {
        let _ = event_loop;
        let _ = message;
        Ok(())
    }

    fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: WindowEvent,
    ) -> anyhow::Result<()> {
        let _ = event_loop;
        let _ = event;
        Ok(())
    }

    fn update(&mut self, ctx: &egui::Context);
}
