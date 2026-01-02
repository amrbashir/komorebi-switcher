use std::sync::Arc;

use anyhow::Context;
use windows::core::*;
use winit::event::WindowEvent;
use winit::window::Window;

use super::dx12_surface::Dx12Surface;
use super::egui_renderer::EguiRenderer;
use super::EguiView;

pub struct WgpuSurface {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    surface: wgpu::Surface<'static>,
    pub egui_renderer: EguiRenderer,
    #[allow(unused)]
    dx12_surface: Dx12Surface,
}

impl WgpuSurface {
    pub async fn new(window: &Arc<Window>, instance: &wgpu::Instance) -> anyhow::Result<Self> {
        let dx12_surface = Dx12Surface::new(window)?;

        let visual = dx12_surface.wgpu_visual.as_raw();
        let visual = wgpu::SurfaceTargetUnsafe::CompositionVisual(visual);
        let surface = unsafe { instance.create_surface_unsafe(visual)? };

        let power_pref = wgpu::PowerPreference::LowPower;
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: power_pref,
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .context("Failed to find an appropriate adapter")?;

        let (width, height) = window.inner_size().into();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::MemoryUsage,
                },
                None,
            )
            .await?;

        let swapchain_capabilities = surface.get_capabilities(&adapter);
        let selected_format = wgpu::TextureFormat::Bgra8UnormSrgb;
        let swapchain_format = swapchain_capabilities
            .formats
            .iter()
            .find(|d| **d == selected_format)
            .context("failed to select proper surface texture format!")?;

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: *swapchain_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 0,
            alpha_mode: wgpu::CompositeAlphaMode::PreMultiplied,
            view_formats: vec![],
        };

        surface.configure(&device, &surface_config);

        unsafe { dx12_surface.desktop.Commit()? };

        let egui_renderer = EguiRenderer::new(&device, surface_config.format, None, 1, window);

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            egui_renderer,
            dx12_surface,
        })
    }

    pub fn resize_surface(&mut self, width: u32, height: u32) {
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn handle_input(
        &mut self,
        window: &Window,
        event: &WindowEvent,
    ) -> egui_winit::EventResponse {
        self.egui_renderer.state.on_window_event(window, event)
    }

    pub fn handle_resized(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.resize_surface(width, height);
        }
    }

    pub fn handle_redraw(
        &mut self,
        window: &Arc<Window>,
        egui_view: &mut (impl EguiView + ?Sized),
    ) -> anyhow::Result<()> {
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.surface_config.width, self.surface_config.height],
            pixels_per_point: self.egui_renderer.egui_ctx().pixels_per_point(),
        };

        let surface_texture = self.surface.get_current_texture()?;

        let surface_view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            self.egui_renderer.begin_frame(window);

            let ctx = self.egui_renderer.egui_ctx();

            egui_view.update(ctx);

            self.egui_renderer.end_frame_and_draw(
                &self.device,
                &self.queue,
                &mut encoder,
                window,
                &surface_view,
                screen_descriptor,
            );
        }

        self.queue.submit(Some(encoder.finish()));
        surface_texture.present();

        Ok(())
    }
}
