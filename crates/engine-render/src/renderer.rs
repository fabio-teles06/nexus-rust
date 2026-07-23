use std::sync::Arc;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec3};
use thiserror::Error;
use wgpu::util::DeviceExt;
use winit::{dpi::PhysicalSize, event_loop::OwnedDisplayHandle, window::Window};

#[derive(Debug, Error)]
pub enum RendererInitError {
    #[error("falha ao criar superfície: {0}")] CreateSurface(#[from] wgpu::CreateSurfaceError),
    #[error("adaptador gráfico indisponível: {0}")] Adapter(#[from] wgpu::RequestAdapterError),
    #[error("falha ao criar dispositivo: {0}")] Device(#[from] wgpu::RequestDeviceError),
    #[error("superfície incompatível")] UnsupportedSurface,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)] pub enum RenderFrameStatus { Presented, Reconfigured, Skipped, Lost, ValidationError }
#[derive(Debug, Clone, Copy)] pub struct RenderInstance { pub model: Mat4, pub color: [f32; 4] }
impl RenderInstance {
    pub fn new(model: Mat4, color: [f32; 4]) -> Self { Self { model, color } }
    pub fn cube(position: Vec3, scale: Vec3, color: [f32; 4]) -> Self { Self::new(Mat4::from_scale_rotation_translation(scale, Quat::IDENTITY, position), color) }
}

#[repr(C)] #[derive(Clone, Copy, Pod, Zeroable)] struct Vertex { position: [f32;3], normal: [f32;3] }
impl Vertex {
    const ATTRS: [wgpu::VertexAttribute;2] = wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3];
    fn layout() -> wgpu::VertexBufferLayout<'static> { wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Self>() as _, step_mode: wgpu::VertexStepMode::Vertex, attributes: &Self::ATTRS } }
}
#[repr(C)] #[derive(Clone, Copy, Pod, Zeroable)] struct InstanceRaw { model: [[f32;4];4], color: [f32;4] }
impl InstanceRaw {
    const ATTRS: [wgpu::VertexAttribute;5] = wgpu::vertex_attr_array![2=>Float32x4,3=>Float32x4,4=>Float32x4,5=>Float32x4,6=>Float32x4];
    fn layout() -> wgpu::VertexBufferLayout<'static> { wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Self>() as _, step_mode: wgpu::VertexStepMode::Instance, attributes: &Self::ATTRS } }
}
#[repr(C)] #[derive(Clone, Copy, Pod, Zeroable)] struct CameraUniform { view_projection: [[f32;4];4] }

const VERTICES: &[Vertex] = &[
    Vertex{position:[-0.5,-0.5,0.5],normal:[0.,0.,1.]},Vertex{position:[0.5,-0.5,0.5],normal:[0.,0.,1.]},Vertex{position:[0.5,0.5,0.5],normal:[0.,0.,1.]},Vertex{position:[-0.5,0.5,0.5],normal:[0.,0.,1.]},
    Vertex{position:[0.5,-0.5,-0.5],normal:[0.,0.,-1.]},Vertex{position:[-0.5,-0.5,-0.5],normal:[0.,0.,-1.]},Vertex{position:[-0.5,0.5,-0.5],normal:[0.,0.,-1.]},Vertex{position:[0.5,0.5,-0.5],normal:[0.,0.,-1.]},
    Vertex{position:[0.5,-0.5,0.5],normal:[1.,0.,0.]},Vertex{position:[0.5,-0.5,-0.5],normal:[1.,0.,0.]},Vertex{position:[0.5,0.5,-0.5],normal:[1.,0.,0.]},Vertex{position:[0.5,0.5,0.5],normal:[1.,0.,0.]},
    Vertex{position:[-0.5,-0.5,-0.5],normal:[-1.,0.,0.]},Vertex{position:[-0.5,-0.5,0.5],normal:[-1.,0.,0.]},Vertex{position:[-0.5,0.5,0.5],normal:[-1.,0.,0.]},Vertex{position:[-0.5,0.5,-0.5],normal:[-1.,0.,0.]},
    Vertex{position:[-0.5,0.5,0.5],normal:[0.,1.,0.]},Vertex{position:[0.5,0.5,0.5],normal:[0.,1.,0.]},Vertex{position:[0.5,0.5,-0.5],normal:[0.,1.,0.]},Vertex{position:[-0.5,0.5,-0.5],normal:[0.,1.,0.]},
    Vertex{position:[-0.5,-0.5,-0.5],normal:[0.,-1.,0.]},Vertex{position:[0.5,-0.5,-0.5],normal:[0.,-1.,0.]},Vertex{position:[0.5,-0.5,0.5],normal:[0.,-1.,0.]},Vertex{position:[-0.5,-0.5,0.5],normal:[0.,-1.,0.]},
];
const INDICES: &[u16] = &[0,1,2,0,2,3,4,5,6,4,6,7,8,9,10,8,10,11,12,13,14,12,14,15,16,17,18,16,18,19,20,21,22,20,22,23];

pub struct Renderer {
    surface: wgpu::Surface<'static>, device: wgpu::Device, queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration, size: PhysicalSize<u32>, pipeline: wgpu::RenderPipeline,
    vertex: wgpu::Buffer, index: wgpu::Buffer, instance: wgpu::Buffer, instance_capacity: usize,
    camera_buffer: wgpu::Buffer, camera_group: wgpu::BindGroup, depth: wgpu::TextureView,
}
impl Renderer {
    pub fn new(window: Arc<Window>, display: OwnedDisplayHandle) -> Result<Self, RendererInitError> { pollster::block_on(Self::new_async(window, display)) }
    async fn new_async(window: Arc<Window>, display: OwnedDisplayHandle) -> Result<Self, RendererInitError> {
        let size = window.inner_size(); let width=size.width.max(1); let height=size.height.max(1);
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_with_display_handle(Box::new(display)));
        let surface = instance.create_surface(window)?;
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions { power_preference: wgpu::PowerPreference::HighPerformance, compatible_surface: Some(&surface), force_fallback_adapter: false }).await?;
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor { label: Some("ferrum-device"), required_features: wgpu::Features::empty(), required_limits: wgpu::Limits::default(), ..Default::default() }).await?;
        let mut config = surface.get_default_config(&adapter,width,height).ok_or(RendererInitError::UnsupportedSurface)?;
        config.present_mode = wgpu::PresentMode::AutoVsync; surface.configure(&device,&config);
        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label:Some("camera-layout"), entries:&[wgpu::BindGroupLayoutEntry { binding:0, visibility:wgpu::ShaderStages::VERTEX, ty:wgpu::BindingType::Buffer { ty:wgpu::BufferBindingType::Uniform, has_dynamic_offset:false, min_binding_size:None }, count:None }] });
        let camera = camera_uniform(Vec3::ZERO,width,height);
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label:Some("camera-buffer"), contents:bytemuck::bytes_of(&camera), usage:wgpu::BufferUsages::UNIFORM|wgpu::BufferUsages::COPY_DST });
        let camera_group = device.create_bind_group(&wgpu::BindGroupDescriptor { label:Some("camera-group"), layout:&camera_layout, entries:&[wgpu::BindGroupEntry { binding:0, resource:camera_buffer.as_entire_binding() }] });
        let shader=device.create_shader_module(wgpu::ShaderModuleDescriptor{label:Some("shader"),source:wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into())});
        let layout=device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor{label:Some("pipeline-layout"),bind_group_layouts:&[Some(&camera_layout)],immediate_size:0});
        let buffers=[Vertex::layout(),InstanceRaw::layout()];
        let targets=[Some(wgpu::ColorTargetState{format:config.format,blend:Some(wgpu::BlendState::REPLACE),write_mask:wgpu::ColorWrites::ALL})];
        let pipeline=device.create_render_pipeline(&wgpu::RenderPipelineDescriptor{
            label:Some("pipeline"),layout:Some(&layout),
            vertex:wgpu::VertexState{module:&shader,entry_point:Some("vs_main"),compilation_options:Default::default(),buffers:&buffers},
            primitive:wgpu::PrimitiveState{cull_mode:None,..Default::default()},
            depth_stencil:Some(wgpu::DepthStencilState{format:wgpu::TextureFormat::Depth24Plus,depth_write_enabled:Some(true),depth_compare:Some(wgpu::CompareFunction::Less),stencil:Default::default(),bias:Default::default()}),
            multisample:Default::default(),fragment:Some(wgpu::FragmentState{module:&shader,entry_point:Some("fs_main"),compilation_options:Default::default(),targets:&targets}),multiview_mask:None,cache:None,
        });
        let vertex=device.create_buffer_init(&wgpu::util::BufferInitDescriptor{label:Some("vertices"),contents:bytemuck::cast_slice(VERTICES),usage:wgpu::BufferUsages::VERTEX});
        let index=device.create_buffer_init(&wgpu::util::BufferInitDescriptor{label:Some("indices"),contents:bytemuck::cast_slice(INDICES),usage:wgpu::BufferUsages::INDEX});
        let instance_capacity=128; let instance=create_instance_buffer(&device,instance_capacity);
        let depth=create_depth(&device,width,height);
        Ok(Self{surface,device,queue,config,size:PhysicalSize::new(width,height),pipeline,vertex,index,instance,instance_capacity,camera_buffer,camera_group,depth})
    }
    pub fn resize(&mut self,size:PhysicalSize<u32>){if size.width==0||size.height==0{return;}self.size=size;self.config.width=size.width;self.config.height=size.height;self.surface.configure(&self.device,&self.config);self.depth=create_depth(&self.device,size.width,size.height);}
    pub fn set_camera_target(&self,target:Vec3){let uniform=camera_uniform(target,self.size.width,self.size.height);self.queue.write_buffer(&self.camera_buffer,0,bytemuck::bytes_of(&uniform));}
    pub fn render(&mut self,instances:&[RenderInstance])->RenderFrameStatus{
        if instances.len()>self.instance_capacity{self.instance_capacity=instances.len().next_power_of_two();self.instance=create_instance_buffer(&self.device,self.instance_capacity);}
        let raw:Vec<InstanceRaw>=instances.iter().map(|i|InstanceRaw{model:i.model.to_cols_array_2d(),color:i.color}).collect();if !raw.is_empty(){self.queue.write_buffer(&self.instance,0,bytemuck::cast_slice(&raw));}
        let (frame,reconfigure)=match self.surface.get_current_texture(){
            wgpu::CurrentSurfaceTexture::Success(t)=>(t,false),wgpu::CurrentSurfaceTexture::Suboptimal(t)=>(t,true),
            wgpu::CurrentSurfaceTexture::Timeout|wgpu::CurrentSurfaceTexture::Occluded=>return RenderFrameStatus::Skipped,
            wgpu::CurrentSurfaceTexture::Outdated=>{self.surface.configure(&self.device,&self.config);return RenderFrameStatus::Reconfigured;},
            wgpu::CurrentSurfaceTexture::Lost=>return RenderFrameStatus::Lost,wgpu::CurrentSurfaceTexture::Validation=>return RenderFrameStatus::ValidationError,
        };
        let view=frame.texture.create_view(&Default::default());let mut encoder=self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor{label:Some("encoder")});
        {let colors=[Some(wgpu::RenderPassColorAttachment{view:&view,depth_slice:None,resolve_target:None,ops:wgpu::Operations{load:wgpu::LoadOp::Clear(wgpu::Color{r:0.03,g:0.04,b:0.06,a:1.}),store:wgpu::StoreOp::Store}})];
        let mut pass=encoder.begin_render_pass(&wgpu::RenderPassDescriptor{label:Some("main-pass"),color_attachments:&colors,depth_stencil_attachment:Some(wgpu::RenderPassDepthStencilAttachment{view:&self.depth,depth_ops:Some(wgpu::Operations{load:wgpu::LoadOp::Clear(1.),store:wgpu::StoreOp::Store}),stencil_ops:None}),timestamp_writes:None,occlusion_query_set:None,multiview_mask:None});
        pass.set_pipeline(&self.pipeline);pass.set_bind_group(0,&self.camera_group,&[]);pass.set_vertex_buffer(0,self.vertex.slice(..));pass.set_vertex_buffer(1,self.instance.slice(..));pass.set_index_buffer(self.index.slice(..),wgpu::IndexFormat::Uint16);pass.draw_indexed(0..INDICES.len() as u32,0,0..instances.len() as u32);}
        self.queue.submit([encoder.finish()]);frame.present();if reconfigure{self.surface.configure(&self.device,&self.config);RenderFrameStatus::Reconfigured}else{RenderFrameStatus::Presented}
    }
}
fn camera_uniform(target:Vec3,width:u32,height:u32)->CameraUniform{let eye=target+Vec3::new(8.,7.,10.);let view=Mat4::look_at_rh(eye,target,Vec3::Y);let proj=Mat4::perspective_rh(55f32.to_radians(),width.max(1) as f32/height.max(1) as f32,0.1,500.);CameraUniform{view_projection:(proj*view).to_cols_array_2d()}}
fn create_instance_buffer(device:&wgpu::Device,cap:usize)->wgpu::Buffer{device.create_buffer(&wgpu::BufferDescriptor{label:Some("instances"),size:(cap.max(1)*std::mem::size_of::<InstanceRaw>())as u64,usage:wgpu::BufferUsages::VERTEX|wgpu::BufferUsages::COPY_DST,mapped_at_creation:false})}
fn create_depth(device:&wgpu::Device,width:u32,height:u32)->wgpu::TextureView{device.create_texture(&wgpu::TextureDescriptor{label:Some("depth"),size:wgpu::Extent3d{width:width.max(1),height:height.max(1),depth_or_array_layers:1},mip_level_count:1,sample_count:1,dimension:wgpu::TextureDimension::D2,format:wgpu::TextureFormat::Depth24Plus,usage:wgpu::TextureUsages::RENDER_ATTACHMENT,view_formats:&[]}).create_view(&Default::default())}
