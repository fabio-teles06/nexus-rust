use std::{
    borrow::Cow,
    mem,
    sync::Arc,
};

use wgpu::util::DeviceExt;

use winit::{
    dpi::PhysicalSize,
    event_loop::OwnedDisplayHandle,
    window::{Window, WindowId},
};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x3
        ];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>()
                as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.5, 0.5, 0.0],
        color: [1.0, 0.2, 0.2],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.2, 1.0, 0.2],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [0.2, 0.4, 1.0],
    },
    Vertex {
        position: [0.5, 0.5, 0.0],
        color: [1.0, 0.8, 0.2],
    },
];

const INDICES: &[u16] = &[
    0, 1, 2,
    0, 2, 3,
];

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

    size: PhysicalSize<u32>,
}

impl Renderer {
    pub async fn new(
        display_handle: OwnedDisplayHandle,
        window: Arc<Window>,
    ) -> Result<Self, String> {
        let instance = wgpu::Instance::new(
            wgpu::InstanceDescriptor::new_with_display_handle(
                Box::new(display_handle),
            ),
        );

        let surface = instance
            .create_surface(Arc::clone(&window))
            .map_err(|error| {
                format!("Erro ao criar a superfície: {error}")
            })?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference:
                    wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await
            .map_err(|error| {
                format!("Nenhuma GPU compatível encontrada: {error}")
            })?;

        let adapter_info = adapter.get_info();

        println!("GPU: {}", adapter_info.name);
        println!("Backend: {:?}", adapter_info.backend);

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("Nexus device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features:
                    wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .await
            .map_err(|error| {
                format!("Erro ao criar dispositivo gráfico: {error}")
            })?;

        let mut size = window.inner_size();

        size.width = size.width.max(1);
        size.height = size.height.max(1);

        let surface_config = surface
            .get_default_config(
                &adapter,
                size.width,
                size.height,
            )
            .ok_or_else(|| {
                String::from(
                    "Não foi possível configurar a superfície",
                )
            })?;

        surface.configure(&device, &surface_config);

        let shader = device.create_shader_module(
            wgpu::ShaderModuleDescriptor {
                label: Some("Main shader"),
                source: wgpu::ShaderSource::Wgsl(
                    Cow::Borrowed(include_str!("shader.wgsl")),
                ),
            },
        );

        let pipeline_layout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Main pipeline layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            },
        );

        let render_pipeline = device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
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
                    targets: &[Some(
                        surface_config.format.into(),
                    )],
                    compilation_options: Default::default(),
                }),

                primitive: wgpu::PrimitiveState {
                    topology:
                        wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },

                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            },
        );

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Square vertex buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            },
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Square index buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX,
            },
        );

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
            index_count: INDICES.len() as u32,
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
    }

    fn configure_surface(&self) {
        self.surface
            .configure(&self.device, &self.surface_config);
    }

    pub fn render(&mut self) {
        if self.size.width == 0 || self.size.height == 0 {
            return;
        }

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,

            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded => {
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
                match self.instance.create_surface(
                    Arc::clone(&self.window),
                ) {
                    Ok(surface) => {
                        self.surface = surface;
                        self.configure_surface();
                    }

                    Err(error) => {
                        eprintln!(
                            "Erro ao recriar superfície: {error}"
                        );
                    }
                }

                return;
            }

            wgpu::CurrentSurfaceTexture::Validation => {
                eprintln!("Erro de validação ao obter frame");
                return;
            }
        };

        let view = frame.texture.create_view(
            &wgpu::TextureViewDescriptor::default(),
        );

        let mut encoder = self.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor {
                label: Some("Main command encoder"),
            },
        );

        {
            let mut render_pass = encoder.begin_render_pass(
                &wgpu::RenderPassDescriptor {
                    label: Some("Main render pass"),

                    color_attachments: &[Some(
                        wgpu::RenderPassColorAttachment {
                            view: &view,
                            depth_slice: None,
                            resolve_target: None,

                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(
                                    wgpu::Color {
                                        r: 0.03,
                                        g: 0.05,
                                        b: 0.09,
                                        a: 1.0,
                                    },
                                ),
                                store: wgpu::StoreOp::Store,
                            },
                        },
                    )],

                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                },
            );

            render_pass.set_pipeline(&self.render_pipeline);

            render_pass.set_vertex_buffer(
                0,
                self.vertex_buffer.slice(..),
            );

            render_pass.set_index_buffer(
                self.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );

            render_pass.draw_indexed(
                0..self.index_count,
                0,
                0..1,
            );
        }

        self.queue.submit([encoder.finish()]);

        self.window.pre_present_notify();
        self.queue.present(frame);
    }
}