use std::{
    borrow::Cow,
    collections::HashMap,
    mem,
    sync::Arc,
};

use glam::{Mat4, Vec3};
use winit::{
    dpi::PhysicalSize,
    event_loop::OwnedDisplayHandle,
    window::Window,
};

use crate::{
    frustum::Frustum,
    mesh::{InstanceData, MeshData, Vertex, unit_cube_mesh},
    voxel::{CHUNK_SIZE, ChunkPos},
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
    vertex_capacity: u64,
    index_capacity: u64,
    index_count: u32,
}

impl GpuMesh {
    fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        mesh: &MeshData,
    ) -> Option<Self> {
        if mesh.is_empty() {
            return None;
        }

        let vertex_bytes = bytemuck::cast_slice(&mesh.vertices);
        let index_bytes = bytemuck::cast_slice(&mesh.indices);
        let vertex_capacity = buffer_capacity(vertex_bytes.len() as u64);
        let index_capacity = buffer_capacity(index_bytes.len() as u64);

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: vertex_capacity,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: index_capacity,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buffer, 0, vertex_bytes);
        queue.write_buffer(&index_buffer, 0, index_bytes);

        Some(Self {
            vertex_buffer,
            index_buffer,
            vertex_capacity,
            index_capacity,
            index_count: mesh.indices.len() as u32,
        })
    }

    fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        mesh: &MeshData,
    ) {
        let vertex_bytes = bytemuck::cast_slice(&mesh.vertices);
        let index_bytes = bytemuck::cast_slice(&mesh.indices);

        if vertex_bytes.len() as u64 > self.vertex_capacity {
            self.vertex_capacity = buffer_capacity(vertex_bytes.len() as u64);
            self.vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: self.vertex_capacity,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        if index_bytes.len() as u64 > self.index_capacity {
            self.index_capacity = buffer_capacity(index_bytes.len() as u64);
            self.index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: self.index_capacity,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }

        queue.write_buffer(&self.vertex_buffer, 0, vertex_bytes);
        queue.write_buffer(&self.index_buffer, 0, index_bytes);
        self.index_count = mesh.indices.len() as u32;
    }
}

struct ChunkGpuMesh {
    mesh: GpuMesh,
    min: Vec3,
    max: Vec3,
}

impl ChunkGpuMesh {
    fn new(position: ChunkPos, mesh: GpuMesh) -> Self {
        let min = position.world_origin().as_vec3();
        let max = min + Vec3::splat(CHUNK_SIZE as f32);
        Self { mesh, min, max }
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
    instance_pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    depth: DepthTexture,
    chunk_meshes: HashMap<ChunkPos, ChunkGpuMesh>,
    instance_cube: GpuMesh,
    instance_buffer: wgpu::Buffer,
    instance_capacity: u64,
    instance_count: u32,
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
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Camera uniform buffer"),
            size: mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));

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

        let primitive = wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        };
        let depth_stencil = Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Nexus chunk pipeline"),
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
            primitive: primitive.clone(),
            depth_stencil: depth_stencil.clone(),
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let instance_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Nexus instance pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_instanced"),
                buffers: &[Vertex::layout(), InstanceData::layout()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(surface_config.format.into())],
                compilation_options: Default::default(),
            }),
            primitive,
            depth_stencil,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        let depth = DepthTexture::new(&device, &surface_config);
        let instance_cube = GpuMesh::new(
            &device,
            &queue,
            "Dynamic cube mesh",
            &unit_cube_mesh(),
        )
        .ok_or_else(|| "Não foi possível criar a mesh do cubo".to_string())?;
        let instance_capacity = buffer_capacity(mem::size_of::<InstanceData>() as u64);
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dynamic instance buffer"),
            size: instance_capacity,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Ok(Self {
            instance,
            window,
            device,
            queue,
            surface,
            surface_config,
            render_pipeline,
            instance_pipeline,
            camera_buffer,
            camera_bind_group,
            depth,
            chunk_meshes: HashMap::new(),
            instance_cube,
            instance_buffer,
            instance_capacity,
            instance_count: 0,
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
        if mesh.is_empty() {
            self.chunk_meshes.remove(&position);
            return;
        }

        if let Some(existing) = self.chunk_meshes.get_mut(&position) {
            existing
                .mesh
                .update(&self.device, &self.queue, "Chunk mesh", mesh);
            return;
        }

        if let Some(gpu_mesh) = GpuMesh::new(
            &self.device,
            &self.queue,
            "Chunk mesh",
            mesh,
        ) {
            self.chunk_meshes
                .insert(position, ChunkGpuMesh::new(position, gpu_mesh));
        }
    }

    pub fn remove_chunk(&mut self, position: ChunkPos) {
        self.chunk_meshes.remove(&position);
    }

    pub fn update_instances(&mut self, instances: &[InstanceData]) {
        self.instance_count = instances.len() as u32;
        if instances.is_empty() {
            return;
        }

        let bytes = bytemuck::cast_slice(instances);
        if bytes.len() as u64 > self.instance_capacity {
            self.instance_capacity = buffer_capacity(bytes.len() as u64);
            self.instance_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Dynamic instance buffer"),
                size: self.instance_capacity,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        self.queue.write_buffer(&self.instance_buffer, 0, bytes);
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
                        store: wgpu::StoreOp::Discard,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            pass.set_pipeline(&self.render_pipeline);
            pass.set_bind_group(0, &self.camera_bind_group, &[]);

            for chunk in self.chunk_meshes.values() {
                if frustum.intersects_aabb(chunk.min, chunk.max) {
                    draw_mesh(&mut pass, &chunk.mesh);
                    render_stats.visible_chunks += 1;
                } else {
                    render_stats.culled_chunks += 1;
                }
            }

            if self.instance_count > 0 {
                pass.set_pipeline(&self.instance_pipeline);
                pass.set_vertex_buffer(0, self.instance_cube.vertex_buffer.slice(..));
                pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
                pass.set_index_buffer(
                    self.instance_cube.index_buffer.slice(..),
                    wgpu::IndexFormat::Uint32,
                );
                pass.draw_indexed(
                    0..self.instance_cube.index_count,
                    0,
                    0..self.instance_count,
                );
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

#[inline]
fn buffer_capacity(required: u64) -> u64 {
    required.max(4).next_power_of_two()
}
