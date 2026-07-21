use egui::{Context, FullOutput, ViewportId};
use egui_wgpu::{RendererOptions, ScreenDescriptor};
use winit::{event::WindowEvent, window::Window};

use crate::game::DebugSnapshot;

pub struct DebugActions {
    pub spawn_cube: bool,
    pub regenerate_chunks: bool,
    pub horizontal_radius: i32,
    pub vertical_radius: i32,
    pub physics_paused: bool,
    pub gravity_y: f32,
}

pub struct GuiFrame {
    paint_jobs: Vec<egui::ClippedPrimitive>,
    textures_delta: egui::TexturesDelta,
    pixels_per_point: f32,
}

pub struct DebugGui {
    context: Context,
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    visible: bool,
    horizontal_radius: i32,
    vertical_radius: i32,
    physics_paused: bool,
    gravity_y: f32,
    pending_texture_frees: Vec<egui::TextureId>,
}

impl DebugGui {
    pub fn new(
        window: &Window,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        max_texture_side: usize,
        horizontal_radius: i32,
        vertical_radius: i32,
    ) -> Self {
        let context = Context::default();
        let state = egui_winit::State::new(
            context.clone(),
            ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            None,
            Some(max_texture_side),
        );
        let renderer = egui_wgpu::Renderer::new(
            device,
            surface_format,
            RendererOptions::default(),
        );

        Self {
            context,
            state,
            renderer,
            visible: false,
            horizontal_radius,
            vertical_radius,
            physics_paused: false,
            gravity_y: -18.0,
            pending_texture_frees: Vec::new(),
        }
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn toggle(&mut self) -> bool {
        self.visible = !self.visible;
        self.visible
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn on_window_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        if !self.visible {
            return false;
        }
        self.state.on_window_event(window, event).consumed
    }

    pub fn on_mouse_motion(&mut self, delta: (f64, f64)) -> bool {
        self.visible && self.state.on_mouse_motion(delta)
    }

    pub fn begin_frame(
        &mut self,
        window: &Window,
        snapshot: &DebugSnapshot,
    ) -> (GuiFrame, DebugActions) {
        let input = self.state.take_egui_input(window);
        let mut spawn_cube = false;
        let mut regenerate_chunks = false;

        let visible = self.visible;
        let mut horizontal_radius = self.horizontal_radius;
        let mut vertical_radius = self.vertical_radius;
        let mut physics_paused = self.physics_paused;
        let mut gravity_y = self.gravity_y;

        let output = self.context.run_ui(input, |root_ui| {
            if !visible {
                return;
            }

            let ctx = root_ui.ctx().clone();
            egui::Window::new("Nexus Debug")
                .default_width(340.0)
                .show(&ctx, |ui| {
                    ui.heading("Runtime");
                    ui.label(format!("FPS: {:.1}", snapshot.fps));
                    ui.label(format!("Frame: {:.2} ms", snapshot.frame_ms));
                    ui.separator();

                    ui.heading("Câmera e chunks");
                    ui.monospace(format!(
                        "Posição: {:.1}, {:.1}, {:.1}",
                        snapshot.camera_position.x,
                        snapshot.camera_position.y,
                        snapshot.camera_position.z,
                    ));
                    ui.label(format!("Chunk central: {:?}", snapshot.center_chunk));
                    ui.label(format!("Chunks carregados: {}", snapshot.loaded_chunks));
                    ui.label(format!("Blocos sólidos: {}", snapshot.solid_blocks));
                    ui.label(format!("Triângulos voxel: {}", snapshot.rendered_triangles));

                    ui.horizontal(|ui| {
                        ui.label("Raio horizontal");
                        ui.add(egui::Slider::new(&mut horizontal_radius, 0..=5));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Raio vertical");
                        ui.add(egui::Slider::new(&mut vertical_radius, 0..=4));
                    });
                    if ui.button("Regenerar streaming").clicked() {
                        regenerate_chunks = true;
                    }

                    ui.separator();
                    ui.heading("Física + ECS");
                    ui.label(format!("Rigid bodies: {}", snapshot.physics_bodies));
                    ui.label(format!("Colliders: {}", snapshot.physics_colliders));
                    ui.label(format!("Entidades ECS: {}", snapshot.ecs_entities));
                    ui.label(format!("Gravidade Y: {:.1}", snapshot.gravity_y));
                    ui.label(format!(
                        "Estado: {}",
                        if snapshot.physics_paused { "pausada" } else { "executando" }
                    ));
                    if let Some(name) = &snapshot.first_entity_name {
                        ui.label(format!("Primeira entidade: {name}"));
                    }
                    ui.checkbox(&mut physics_paused, "Pausar física");
                    ui.add(
                        egui::Slider::new(&mut gravity_y, -40.0..=0.0)
                            .text("Gravidade Y"),
                    );
                    if ui.button("Criar caixa física").clicked() {
                        spawn_cube = true;
                    }

                    ui.separator();
                    ui.small(
                        "F1 fecha o painel. WASD controla a câmera quando o painel está fechado.",
                    );
                });
        });

        self.horizontal_radius = horizontal_radius;
        self.vertical_radius = vertical_radius;
        self.physics_paused = physics_paused;
        self.gravity_y = gravity_y;

        let FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            ..
        } = output;
        self.state.handle_platform_output(window, platform_output);
        let paint_jobs = self.context.tessellate(shapes, pixels_per_point);

        (
            GuiFrame {
                paint_jobs,
                textures_delta,
                pixels_per_point,
            },
            DebugActions {
                spawn_cube,
                regenerate_chunks,
                horizontal_radius: self.horizontal_radius,
                vertical_radius: self.vertical_radius,
                physics_paused: self.physics_paused,
                gravity_y: self.gravity_y,
            },
        )
    }

    pub fn paint(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        size_in_pixels: [u32; 2],
        frame: &GuiFrame,
    ) -> Vec<wgpu::CommandBuffer> {
        for id in self.pending_texture_frees.drain(..) {
            self.renderer.free_texture(&id);
        }

        for (id, delta) in &frame.textures_delta.set {
            self.renderer.update_texture(device, queue, *id, delta);
        }

        let screen = ScreenDescriptor {
            size_in_pixels,
            pixels_per_point: frame.pixels_per_point,
        };
        let command_buffers = self.renderer.update_buffers(
            device,
            queue,
            encoder,
            &frame.paint_jobs,
            &screen,
        );

        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            let mut pass = pass.forget_lifetime();
            self.renderer.render(&mut pass, &frame.paint_jobs, &screen);
        }

        self.pending_texture_frees
            .extend(frame.textures_delta.free.iter().copied());

        command_buffers
    }
}
