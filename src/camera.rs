use glam::{
    Mat4, Vec3,
    camera::rh::{proj::directx::perspective, view::look_at_mat4},
};

pub struct Camera {
    eye: Vec3,
    target: Vec3,
    up: Vec3,

    aspect: f32,
    fov_y: f32,
    near: f32,
    far: f32,
}

impl Camera {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            eye: Vec3::new(2.5, 2.0, 4.0),
            target: Vec3::ZERO,
            up: Vec3::Y,

            aspect: calculate_aspect(width, height),
            fov_y: 45.0_f32.to_radians(),
            near: 0.1,
            far: 100.0,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = calculate_aspect(width, height);
    }

    pub fn view_projection_matrix(&self) -> Mat4 {
        let view = look_at_mat4(self.eye, self.target, self.up);

        let projection = perspective(self.fov_y, self.aspect, self.near, self.far);

        projection * view
    }
}

fn calculate_aspect(width: u32, height: u32) -> f32 {
    width.max(1) as f32 / height.max(1) as f32
}

#[repr(C)]
#[derive(
    Debug,
    Clone,
    Copy,
    bytemuck::Pod,
    bytemuck::Zeroable,
)]
pub struct CameraUniform {
    pub view_projection: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_projection: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub fn update(&mut self, camera: &Camera) {
        self.view_projection = camera
            .view_projection_matrix()
            .to_cols_array_2d();
    }
}