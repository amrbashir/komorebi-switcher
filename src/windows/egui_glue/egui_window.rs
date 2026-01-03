use std::ops::Deref;
use std::sync::Arc;

use muda::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use super::egui_view::EguiView;
use super::wgpu_surface::WgpuSurface;

pub struct EguiWindow {
    pub window: Arc<Window>,
    pub surface: WgpuSurface,
    pub view: Box<dyn EguiView>,
}

impl Deref for EguiWindow {
    type Target = Window;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl EguiWindow {
    pub fn new(
        window: Arc<Window>,
        instance: &wgpu::Instance,
        view: impl EguiView + 'static,
    ) -> anyhow::Result<Self> {
        let surface = pollster::block_on(WgpuSurface::new(&window, instance))?;
        Ok(Self {
            window,
            surface,
            view: Box::new(view),
        })
    }

    pub fn handle_input(&mut self, event: &WindowEvent) -> egui_winit::EventResponse {
        self.surface.handle_input(&self.window, event)
    }

    pub fn handle_resized(&mut self, size: PhysicalSize<u32>) {
        self.surface.handle_resized(size.width, size.height);
    }

    pub fn handle_window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        event: WindowEvent,
    ) -> anyhow::Result<()> {
        let resposne = self.handle_input(&event);

        if resposne.repaint {
            self.handle_redraw()?;
        }

        if let WindowEvent::Resized(size) = event {
            self.handle_resized(size);
        }

        let ctx = self.surface.egui_renderer.egui_ctx();
        self.view.handle_window_event(ctx, event_loop, event)
    }

    pub fn handle_redraw(&mut self) -> anyhow::Result<()> {
        self.surface.handle_redraw(&self.window, self.view.as_mut())
    }
}
