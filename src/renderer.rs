use std::{
    borrow::Cow,
    collections::HashMap,
    mem,
    sync::Arc,
};

use glam::Mat4;
use wgpu::util::DeviceExt;
use winit::{
    dpi::PhysicalSize,
    event_loop::OwnedDisplayHandle,
    window::Window,
};

use crate::{
    frustum::Frustum,
    mesh::{MeshData, Vertex},
    voxel::ChunkPos,
};

const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_projection: [[f32; 4]; 4],
}

#[derive(Debug, Default, Clone, Copy)]
pub struct RenderStats {
    pub visible_chunks: usize,
    pub culled_chunks: usize,
}

struct GpuMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl GpuMesh {
    fn new(device: &wgpu::Device, label: &str, mesh: &MeshData) -> Option<Self> {
        if mesh.is_empty() {
            return None;
        }

        let vertex_label = format!("{label} vertex buffer");
        let index_label = format!("{label} index buffer");

        Some(Self {
            vertex_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&vertex_label),
                contents: bytemuck::cast_slice(&mesh.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }),
            index_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&index_label),
                contents: bytemuck::cast_slice(&mesh.indices),
                usage: wgpu::BufferUsages::INDEX,
            }),
            index_count: mesh.indices.len() as u32,
        })
    }
}

struct DepthTexture {
    _texture: wgpu::Texture,
    view: wgpu::TextureView,
}

impl DepthTexture {
    fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth texture"),
            size: wgpu::Extent3d {
                width: config.width.max(1),
                height: config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        Self {
            _texture: texture,
            view,
        }
    }
}

pub struct Renderer {
    instance: wgpu::Instance,
    window: Arc<Window>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    depth: DepthTexture,
    chunk_meshes: HashMap<ChunkPos, GpuMesh>,
    dynamic_mesh: Option<GpuMesh>,
    last_render_stats: RenderStats,
    size: PhysicalSize<u32>,
}

impl Renderer {
    pub async fn new(
        display_handle: OwnedDisplayHandle,
        window: Arc<Window>,
    ) -> Result<Self, String> {
        let instance = wgpu::Instance::new(
            wgpu::InstanceDescriptor::new_with_display_handle(Box::new(display_handle)),
        );
        let surface = instance
            .create_surface(Arc::clone(&window))
            .map_err(|error| format!("Erro ao criar surface: {error}"))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .map_err(|error| format!("Nenhuma GPU compatível: {error}"))?;

        let info = adapter.get_info();
        log::info!("GPU: {} ({:?})", info.name, info.backend);

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
            .map_err(|error| format!("Erro ao criar device: {error}"))?;

        let mut size = window.inner_size();
        size.width = size.width.max(1);
        size.height = size.height.max(1);

        let surface_config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or_else(|| "Não foi possível configurar a surface".to_string())?;
        surface.configure(&device, &surface_config);

        let camera_uniform = CameraUniform {
            view_projection: Mat4::IDENTITY.to_cols_array_2d(),
        };
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera uniform buffer"),
            contents: bytemuck::bytes_of(&camera_uniform),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera bind group layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(
                        mem::size_of::<CameraUniform>() as u64,
                    ),
                },
                count: None,
            }],
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera bind group"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Nexus voxel shader"),
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shader.wgsl"))),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Nexus pipeline layout"),
            bind_group_layouts: &[Some(&camera_layout)],
            immediate_size: 0,
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Nexus render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::layout()],
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
        let depth = DepthTexture::new(&device, &surface_config);

        Ok(Self {
            instance,
            window,
            device,
            queue,
            surface,
            surface_config,
            render_pipeline,
            camera_buffer,
            camera_bind_group,
            depth,
            chunk_meshes: HashMap::new(),
            dynamic_mesh: None,
            last_render_stats: RenderStats::default(),
            size,
        })
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn surface_format(&self) -> wgpu::TextureFormat {
        self.surface_config.format
    }

    pub fn max_texture_dimension(&self) -> usize {
        self.device.limits().max_texture_dimension_2d as usize
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.size = size;
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.surface.configure(&self.device, &self.surface_config);
        self.depth = DepthTexture::new(&self.device, &self.surface_config);
    }

    pub fn upsert_chunk(&mut self, position: ChunkPos, mesh: &MeshData) {
        match GpuMesh::new(&self.device, "Chunk", mesh) {
            Some(mesh) => {
                self.chunk_meshes.insert(position, mesh);
            }
            None => {
                self.chunk_meshes.remove(&position);
            }
        }
    }

    pub fn remove_chunk(&mut self, position: ChunkPos) {
        self.chunk_meshes.remove(&position);
    }

    pub fn upload_dynamic_mesh(&mut self, mesh: &MeshData) {
        self.dynamic_mesh = GpuMesh::new(&self.device, "Dynamic entities", mesh);
    }

    pub fn render_stats(&self) -> RenderStats {
        self.last_render_stats
    }

    pub fn render<F>(&mut self, view_projection: Mat4, overlay: F)
    where
        F: FnOnce(
            &wgpu::Device,
            &wgpu::Queue,
            &mut wgpu::CommandEncoder,
            &wgpu::TextureView,
            [u32; 2],
        ) -> Vec<wgpu::CommandBuffer>,
    {
        let uniform = CameraUniform {
            view_projection: view_projection.to_cols_array_2d(),
        };
        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));

        let frustum = Frustum::from_view_projection(view_projection);
        let mut render_stats = RenderStats::default();

        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame) => frame,
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.surface.configure(&self.device, &self.surface_config);
                frame
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded => return,
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.surface_config);
                return;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                if let Ok(surface) = self.instance.create_surface(Arc::clone(&self.window)) {
                    self.surface = surface;
                    self.surface.configure(&self.device, &self.surface_config);
                }
                return;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                log::error!("Erro de validação ao adquirir frame");
                return;
            }
        };

        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Nexus frame encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Nexus world pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.42,
                            g: 0.65,
                            b: 0.90,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth.view,
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

            pass.set_pipeline(&self.render_pipeline);
            pass.set_bind_group(0, &self.camera_bind_group, &[]);

            for (position, mesh) in &self.chunk_meshes {
                if frustum.intersects_chunk(*position) {
                    draw_mesh(&mut pass, mesh);
                    render_stats.visible_chunks += 1;
                } else {
                    render_stats.culled_chunks += 1;
                }
            }
            if let Some(mesh) = &self.dynamic_mesh {
                draw_mesh(&mut pass, mesh);
            }
        }

        self.last_render_stats = render_stats;

        let mut command_buffers = overlay(
            &self.device,
            &self.queue,
            &mut encoder,
            &view,
            [self.size.width, self.size.height],
        );
        command_buffers.push(encoder.finish());
        self.queue.submit(command_buffers);

        self.window.pre_present_notify();
        frame.present();
    }
}

fn draw_mesh<'a>(pass: &mut wgpu::RenderPass<'a>, mesh: &'a GpuMesh) {
    pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
    pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    pass.draw_indexed(0..mesh.index_count, 0, 0..1);
}
