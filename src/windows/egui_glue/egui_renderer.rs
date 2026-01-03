use std::sync::Arc;

use winit::window::Window;

pub struct EguiRenderer {
    pub state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    frame_started: bool,
}

impl EguiRenderer {
    pub fn new(
        device: &wgpu::Device,
        output_color_format: wgpu::TextureFormat,
        output_depth_format: Option<wgpu::TextureFormat>,
        msaa_samples: u32,
        window: &Arc<Window>,
    ) -> Self {
        let egui_context = egui::Context::default();

        {
            let window = window.clone();
            egui_context.set_request_repaint_callback(move |_| {
                window.request_redraw();
            });
        }

        let egui_state = egui_winit::State::new(
            egui_context,
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            window.theme(),
            Some(2 * 1024), // default dimension is 2048
        );

        let egui_renderer = egui_wgpu::Renderer::new(
            device,
            output_color_format,
            output_depth_format,
            msaa_samples,
            true,
        );

        Self {
            state: egui_state,
            renderer: egui_renderer,
            frame_started: false,
        }
    }

    pub fn egui_ctx(&self) -> &egui::Context {
        self.state.egui_ctx()
    }

    pub fn begin_frame(&mut self, window: &Window) {
        let raw_input = self.state.take_egui_input(window);
        self.state.egui_ctx().begin_pass(raw_input);
        self.frame_started = true;
    }

    pub fn end_frame_and_draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        window: &Window,
        window_surface_view: &wgpu::TextureView,
        screen_descriptor: egui_wgpu::ScreenDescriptor,
    ) {
        if !self.frame_started {
            panic!("begin_frame must be called before end_frame_and_draw can be called!");
        }

        let full_output = self.state.egui_ctx().end_pass();

        self.state
            .handle_platform_output(window, full_output.platform_output);

        let tris = self
            .egui_ctx()
            .tessellate(full_output.shapes, self.egui_ctx().pixels_per_point());

        for (id, image_delta) in &full_output.textures_delta.set {
            self.renderer
                .update_texture(device, queue, *id, image_delta);
        }

        self.renderer
            .update_buffers(device, queue, encoder, &tris, &screen_descriptor);

        let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: window_surface_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            label: Some("egui main render pass"),
            occlusion_query_set: None,
        });

        self.renderer
            .render(&mut rpass.forget_lifetime(), &tris, &screen_descriptor);

        for x in &full_output.textures_delta.free {
            self.renderer.free_texture(x)
        }

        self.frame_started = false;
    }
}
