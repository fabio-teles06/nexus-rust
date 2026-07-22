use std::sync::Arc;

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};
use thiserror::Error;
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, event_loop::OwnedDisplayHandle, window::Window};

use crate::{
    camera::{Camera, CameraUniform},
    depth::{DEPTH_FORMAT, DepthTexture},
    mesh::{CUBE_INDICES, CUBE_VERTICES, Vertex},
};

const INITIAL_INSTANCE_CAPACITY: usize = 128;

#[derive(Debug, Error)]
pub enum RendererInitError {
    #[error("não foi possível criar a superfície gráfica")]
    CreateSurface(#[from] wgpu::CreateSurfaceError),

    #[error("nenhum adaptador gráfico compatível foi encontrado")]
    RequestAdapter(#[from] wgpu::RequestAdapterError),

    #[error("não foi possível criar o dispositivo gráfico")]
    RequestDevice(#[from] wgpu::RequestDeviceError),

    #[error("a superfície não é compatível com o adaptador")]
    UnsupportedSurface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderFrameStatus {
    Presented,
    Reconfigured,
    Skipped,
    SurfaceLost,
    ValidationError,
}

/// Uma instância de objeto que será desenhada.
///
/// Vários objetos podem compartilhar a mesma malha do cubo,
/// possuindo matrizes e cores diferentes.
#[derive(Debug, Clone, Copy)]
pub struct RenderInstance {
    pub model: Mat4,
    pub color: [f32; 4],
}

impl RenderInstance {
    pub fn new(model: Mat4, color: [f32; 4]) -> Self {
        Self { model, color }
    }

    pub fn from_translation_scale(translation: Vec3, scale: Vec3, color: [f32; 4]) -> Self {
        let model = Mat4::from_scale_rotation_translation(scale, glam::Quat::IDENTITY, translation);

        Self::new(model, color)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct InstanceRaw {
    model: [[f32; 4]; 4],
    color: [f32; 4],
}

impl InstanceRaw {
    const ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        2 => Float32x4,
        3 => Float32x4,
        4 => Float32x4,
        5 => Float32x4,
        6 => Float32x4
    ];

    fn from_instance(instance: &RenderInstance) -> Self {
        Self {
            model: instance.model.to_cols_array_2d(),

            color: instance.color,
        }
    }

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,

            step_mode: wgpu::VertexStepMode::Instance,

            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub struct Renderer {
    surface: wgpu::Surface<'static>,

    device: wgpu::Device,
    queue: wgpu::Queue,

    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,

    render_pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,

    instance_buffer: wgpu::Buffer,
    instance_capacity: usize,

    camera: Camera,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,

    depth_texture: DepthTexture,
}

impl Renderer {
    /// Inicialização síncrona para facilitar o uso dentro
    /// do ApplicationHandler do winit.
    pub fn new(
        window: Arc<Window>,
        display_handle: OwnedDisplayHandle,
    ) -> Result<Self, RendererInitError> {
        pollster::block_on(Self::new_async(window, display_handle))
    }

    async fn new_async(
        window: Arc<Window>,
        display_handle: OwnedDisplayHandle,
    ) -> Result<Self, RendererInitError> {
        let original_size = window.inner_size();

        let size = PhysicalSize::new(original_size.width.max(1), original_size.height.max(1));

        /*
         * O wgpu 30 recomenda fornecer o display handle
         * pertencente ao EventLoop/winit.
         */
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(
            Box::new(display_handle),
        ));

        /*
         * Passar Arc<Window> permite que a Surface possua
         * o handle da janela, resultando em Surface<'static>.
         */
        let surface = instance.create_surface(window)?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("engine-render-device"),

                required_features: wgpu::Features::empty(),

                required_limits: wgpu::Limits::default(),

                ..Default::default()
            })
            .await?;

        let mut config = surface
            .get_default_config(&adapter, size.width, size.height)
            .ok_or(RendererInitError::UnsupportedSurface)?;

        config.present_mode = wgpu::PresentMode::AutoVsync;

        surface.configure(&device, &config);

        let camera = Camera::new(size.width, size.height);

        let camera_uniform = CameraUniform::from_camera(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("camera-buffer"),

            contents: bytemuck::bytes_of(&camera_uniform),

            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera-bind-group-layout"),

                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,

                    visibility: wgpu::ShaderStages::VERTEX,

                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,

                        has_dynamic_offset: false,

                        min_binding_size: None,
                    },

                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera-bind-group"),

            layout: &camera_bind_group_layout,

            entries: &[wgpu::BindGroupEntry {
                binding: 0,

                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("basic-3d-shader"),

            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render-pipeline-layout"),

            bind_group_layouts: &[Some(&camera_bind_group_layout)],

            immediate_size: 0,
        });

        let vertex_layout = Vertex::layout();

        let instance_layout = InstanceRaw::layout();

        let vertex_buffers = [Some(vertex_layout), Some(instance_layout)];

        let color_targets = [Some(wgpu::ColorTargetState {
            format: config.format,

            blend: Some(wgpu::BlendState::REPLACE),

            write_mask: wgpu::ColorWrites::ALL,
        })];

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("basic-render-pipeline"),

            layout: Some(&pipeline_layout),

            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),

                compilation_options: Default::default(),

                buffers: &vertex_buffers,
            },

            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,

                /*
                 * Desativado inicialmente para
                 * tornar o exemplo menos sensível
                 * à ordem dos vértices.
                 */
                cull_mode: None,

                ..Default::default()
            },

            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,

                depth_write_enabled: Some(true),

                depth_compare: Some(wgpu::CompareFunction::Less),

                stencil: Default::default(),

                bias: Default::default(),
            }),

            multisample: Default::default(),

            fragment: Some(wgpu::FragmentState {
                module: &shader,

                entry_point: Some("fs_main"),

                compilation_options: Default::default(),

                targets: &color_targets,
            }),

            multiview_mask: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube-vertex-buffer"),

            contents: bytemuck::cast_slice(CUBE_VERTICES),

            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("cube-index-buffer"),

            contents: bytemuck::cast_slice(CUBE_INDICES),

            usage: wgpu::BufferUsages::INDEX,
        });

        let instance_buffer = create_instance_buffer(&device, INITIAL_INSTANCE_CAPACITY);

        let depth_texture = DepthTexture::new(&device, size.width, size.height);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            index_buffer,

            index_count: CUBE_INDICES.len() as u32,

            instance_buffer,

            instance_capacity: INITIAL_INSTANCE_CAPACITY,

            camera,
            camera_buffer,
            camera_bind_group,
            depth_texture,
        })
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        self.depth_texture = DepthTexture::new(&self.device, new_size.width, new_size.height);
        self.camera.resize(new_size.width, new_size.height);
        self.update_camera_buffer();
    }

    pub fn size(&self) -> PhysicalSize<u32> {
        self.size
    }

    pub fn set_camera_target(&mut self, target: Vec3) {
        self.camera.follow(target);
        self.update_camera_buffer();
    }

    pub fn render(&mut self, instances: &[RenderInstance]) -> RenderFrameStatus {
        self.ensure_instance_capacity(instances.len());

        let raw_instances: Vec<InstanceRaw> =
            instances.iter().map(InstanceRaw::from_instance).collect();

        if !raw_instances.is_empty() {
            self.queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&raw_instances),
            );
        }

        /*
         * No wgpu 30, get_current_texture retorna
         * CurrentSurfaceTexture em vez de Result.
         */
        let (surface_texture, should_reconfigure) = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(texture) => (texture, false),

            wgpu::CurrentSurfaceTexture::Suboptimal(texture) => (texture, true),

            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return RenderFrameStatus::Skipped;
            }

            wgpu::CurrentSurfaceTexture::Outdated => {
                self.reconfigure_surface();

                return RenderFrameStatus::Reconfigured;
            }

            wgpu::CurrentSurfaceTexture::Lost => {
                return RenderFrameStatus::SurfaceLost;
            }

            wgpu::CurrentSurfaceTexture::Validation => {
                return RenderFrameStatus::ValidationError;
            }
        };

        let view = surface_texture
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("render-command-encoder"),
            });
        {
            let color_attachments = [Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,

                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.025,
                        g: 0.035,
                        b: 0.055,
                        a: 1.0,
                    }),

                    store: wgpu::StoreOp::Store,
                },
            })];
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("main-render-pass"),
                color_attachments: &color_attachments,
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
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.index_count, 0, 0..instances.len() as u32);
        }

        self.queue.submit([encoder.finish()]);

        /*
         * wgpu 30 apresenta o frame pela Queue.
         */
        self.queue.present(surface_texture);

        if should_reconfigure {
            self.reconfigure_surface();

            RenderFrameStatus::Reconfigured
        } else {
            RenderFrameStatus::Presented
        }
    }

    fn update_camera_buffer(&self) {
        let camera_uniform = CameraUniform::from_camera(&self.camera);

        self.queue
            .write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&camera_uniform));
    }

    fn reconfigure_surface(&mut self) {
        if self.config.width == 0 || self.config.height == 0 {
            return;
        }

        self.surface.configure(&self.device, &self.config);

        self.depth_texture = DepthTexture::new(&self.device, self.config.width, self.config.height);
    }

    fn ensure_instance_capacity(&mut self, required: usize) {
        if required <= self.instance_capacity {
            return;
        }

        let new_capacity = required.next_power_of_two().max(INITIAL_INSTANCE_CAPACITY);

        self.instance_buffer = create_instance_buffer(&self.device, new_capacity);

        self.instance_capacity = new_capacity;
    }
}

fn create_instance_buffer(device: &wgpu::Device, capacity: usize) -> wgpu::Buffer {
    let size = capacity.max(1) * std::mem::size_of::<InstanceRaw>();

    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("instance-buffer"),

        size: size as wgpu::BufferAddress,

        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,

        mapped_at_creation: false,
    })
}
