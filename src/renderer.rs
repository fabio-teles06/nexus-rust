use std::{borrow::Cow, mem, sync::Arc};

use wgpu::util::DeviceExt;

use winit::{
    dpi::PhysicalSize,
    event_loop::OwnedDisplayHandle,
    window::{CursorGrabMode, Window, WindowId},
};

use crate::{
    camera::{Camera, CameraUniform},
    depth::{DEPTH_FORMAT, DepthTexture},
    input::InputState,
    mesh::Vertex,
    voxel::{AIR, World, build_world_mesh, raycast_world},
};

pub struct Renderer {
    instance: wgpu::Instance,
    window: Arc<Window>,

    device: wgpu::Device,
    queue: wgpu::Queue,

    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,

    render_pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,

    world: World,

    camera: Camera,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    depth_texture: DepthTexture,

    size: PhysicalSize<u32>,
}

impl Renderer {
    pub async fn new(
        display_handle: OwnedDisplayHandle,
        window: Arc<Window>,
    ) -> Result<Self, String> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(display_handle),
        ));

        let surface = instance
            .create_surface(Arc::clone(&window))
            .map_err(|error| format!("Erro ao criar superfície: {error}"))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|error| format!("Nenhuma GPU compatível: {error}"))?;

        let adapter_info = adapter.get_info();

        println!("GPU: {}", adapter_info.name);
        println!("Backend: {:?}", adapter_info.backend);

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Nexus device"),

                required_features: wgpu::Features::empty(),

                required_limits: wgpu::Limits::default(),

                experimental_features: wgpu::ExperimentalFeatures::disabled(),

                memory_hints: wgpu::MemoryHints::Performance,

                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|error| format!("Erro ao criar dispositivo gráfico: {error}"))?;

        let mut size = window.inner_size();

        size.width = size.width.max(1);
        size.height = size.height.max(1);

        let surface_config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or_else(|| String::from("Não foi possível configurar a superfície"))?;

        surface.configure(&device, &surface_config);

        // Câmera

        let camera = Camera::new(surface_config.width, surface_config.height);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera uniform buffer"),

            contents: bytemuck::bytes_of(&camera_uniform),

            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Camera bind group layout"),

                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,

                    visibility: wgpu::ShaderStages::VERTEX,

                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,

                        has_dynamic_offset: false,

                        min_binding_size: wgpu::BufferSize::new(
                            mem::size_of::<CameraUniform>() as u64
                        ),
                    },

                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera bind group"),

            layout: &camera_bind_group_layout,

            entries: &[wgpu::BindGroupEntry {
                binding: 0,

                resource: camera_buffer.as_entire_binding(),
            }],
        });

        // Shader e pipeline

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Main shader"),

            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Main pipeline layout"),

            bind_group_layouts: &[Some(&camera_bind_group_layout)],

            immediate_size: 0,
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Main render pipeline"),

            layout: Some(&pipeline_layout),

            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),

                buffers: &[Some(Vertex::layout())],

                compilation_options: Default::default(),
            },

            fragment: Some(wgpu::FragmentState {
                module: &shader,

                entry_point: Some("fs_main"),

                targets: &[Some(surface_config.format.into())],

                compilation_options: Default::default(),
            }),

            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,

                strip_index_format: None,

                front_face: wgpu::FrontFace::Ccw,

                // Desativado temporariamente.
                // Depois corrigiremos o winding das faces.
                cull_mode: None,

                polygon_mode: wgpu::PolygonMode::Fill,

                unclipped_depth: false,
                conservative: false,
            },

            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,

                depth_write_enabled: Some(true),

                depth_compare: Some(wgpu::CompareFunction::Less),

                stencil: wgpu::StencilState::default(),

                bias: wgpu::DepthBiasState::default(),
            }),

            multisample: wgpu::MultisampleState::default(),

            multiview_mask: None,
            cache: None,
        });

        // mundo
        let world = World::demo();

        let world_mesh = build_world_mesh(&world);

        println!(
            "Mesh do mundo: {} vértices, {} índices, {} faces",
            world_mesh.vertices.len(),
            world_mesh.indices.len(),
            world_mesh.face_count()
        );

        if world_mesh.is_empty() {
            return Err("O mundo não gerou nenhuma geometria".to_string());
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("World vertex buffer"),

            contents: bytemuck::cast_slice(&world_mesh.vertices),

            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("World index buffer"),

            contents: bytemuck::cast_slice(&world_mesh.indices),

            usage: wgpu::BufferUsages::INDEX,
        });

        let depth_texture = DepthTexture::new(&device, &surface_config);

        let index_count = world_mesh.indices.len() as u32;

        Ok(Self {
            instance,
            window,

            device,
            queue,

            surface,
            surface_config,

            render_pipeline,

            vertex_buffer,
            index_buffer,
            index_count,

            world,

            camera,
            camera_buffer,
            camera_bind_group,

            depth_texture,

            size,
        })
    }

    pub fn window_id(&self) -> WindowId {
        self.window.id()
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.size = new_size;

        self.surface_config.width = new_size.width;

        self.surface_config.height = new_size.height;

        self.configure_surface();

        self.camera.resize(new_size.width, new_size.height);

        self.update_camera_buffer();

        self.depth_texture = DepthTexture::new(&self.device, &self.surface_config);
    }

    fn configure_surface(&self) {
        self.surface.configure(&self.device, &self.surface_config);
    }

    fn update_camera_buffer(&self) {
        let mut camera_uniform = CameraUniform::new();

        camera_uniform.update(&self.camera);

        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
    }

    fn rebuild_world_mesh(&mut self) {
        let world_mesh = build_world_mesh(&self.world);

        println!(
            "Mesh do mundo: {} vértices, {} índices, {} faces",
            world_mesh.vertices.len(),
            world_mesh.indices.len(),
            world_mesh.face_count()
        );

        if world_mesh.indices.is_empty() {
            self.index_count = 0;
            return;
        }

        self.vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("World vertex buffer"),
                contents: bytemuck::cast_slice(&world_mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });

        self.index_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("World index buffer"),
                contents: bytemuck::cast_slice(&world_mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            });

        self.index_count = world_mesh.indices.len() as u32;
    }

    pub fn render(&mut self) {
        if self.size.width == 0 || self.size.height == 0 {
            return;
        }

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,

            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return;
            }

            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                drop(frame);
                self.configure_surface();
                return;
            }

            wgpu::CurrentSurfaceTexture::Outdated => {
                self.configure_surface();
                return;
            }

            wgpu::CurrentSurfaceTexture::Lost => {
                match self.instance.create_surface(Arc::clone(&self.window)) {
                    Ok(surface) => {
                        self.surface = surface;
                        self.configure_surface();
                    }

                    Err(error) => {
                        eprintln!("Erro ao recriar superfície: {error}");
                    }
                }

                return;
            }

            wgpu::CurrentSurfaceTexture::Validation => {
                eprintln!("Erro de validação ao obter frame");

                return;
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Main command encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main render pass"),

                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,

                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.08,
                            g: 0.13,
                            b: 0.20,
                            a: 1.0,
                        }),

                        store: wgpu::StoreOp::Store,
                    },
                })],

                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,

                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),

                        store: wgpu::StoreOp::Store,
                    }),

                    stencil_ops: None,
                }),

                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);

            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);

            render_pass.draw_indexed(0..self.index_count, 0, 0..1);
        }

        self.queue.submit([encoder.finish()]);

        self.window.pre_present_notify();
        self.queue.present(frame);
    }

    pub fn set_cursor_captured(&self, captured: bool) -> bool {
        if !captured {
            let result = self.window.set_cursor_grab(CursorGrabMode::None);

            self.window.set_cursor_visible(true);

            if let Err(error) = result {
                eprintln!("Erro ao liberar o cursor: {error}");
            }

            return true;
        }

        let result = self
            .window
            .set_cursor_grab(CursorGrabMode::Locked)
            .or_else(|_| self.window.set_cursor_grab(CursorGrabMode::Confined));

        match result {
            Ok(()) => {
                self.window.set_cursor_visible(false);
                true
            }

            Err(error) => {
                eprintln!("Não foi possível capturar o cursor: {error}");

                self.window.set_cursor_visible(true);

                false
            }
        }
    }

    pub fn update(&mut self, input: &mut InputState, delta_time: f32) {
        self.camera.update(input, delta_time);
        self.update_camera_buffer();
    }

    pub fn break_targeted_block(&mut self) -> bool {
        const MAX_REACH: f32 = 8.0;

        let hit = raycast_world(
            &self.world,
            self.camera.position(),
            self.camera.forward(),
            MAX_REACH,
        );

        let Some(hit) = hit else {
            return false;
        };

        println!("Quebrando bloco em {:?}", hit);

        let removed = self
            .world
            .set(hit.position.x, hit.position.y, hit.position.z, AIR);

        if !removed {
            return false;
        }

        self.rebuild_world_mesh();

        true
    }
}
