pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub struct DepthTexture {
    pub view: wgpu::TextureView,

    _texture: wgpu::Texture,
}

impl DepthTexture {
    pub fn new(device: &wgpu::Device, surface_config: &wgpu::SurfaceConfiguration) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width: surface_config.width.max(1),
                height: surface_config.height.max(1),
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
            view,
            _texture: texture,
        }
    }
}
