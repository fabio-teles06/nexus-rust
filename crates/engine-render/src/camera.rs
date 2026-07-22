use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec3};

#[derive(Debug, Clone, Copy)]
pub(crate) struct Camera {
    pub eye: Vec3,
    pub target: Vec3,
    pub up: Vec3,

    pub aspect: f32,
    pub fovy_radians: f32,
    pub z_near: f32,
    pub z_far: f32,

    follow_offset: Vec3,
}

impl Camera {
    pub fn new(width: u32, height: u32) -> Self {
        let target = Vec3::ZERO;

        let follow_offset = Vec3::new(7.0, 6.0, 10.0);

        Self {
            eye: target + follow_offset,
            target,
            up: Vec3::Y,

            aspect: calculate_aspect(width, height),
            fovy_radians: 55.0_f32.to_radians(),
            z_near: 0.1,
            z_far: 500.0,

            follow_offset,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = calculate_aspect(width, height);
    }

    pub fn follow(&mut self, target: Vec3) {
        self.target = target;
        self.eye = target + self.follow_offset;
    }

    pub fn view_projection(&self) -> Mat4 {
        /*
         * perspective_rh produz uma matriz com profundidade
         * de 0 a 1, compatível com WebGPU/Direct3D/Metal.
         */
        #[allow(deprecated)]
        let view = Mat4::look_at_rh(self.eye, self.target, self.up);

        #[allow(deprecated)]
        let projection =
            Mat4::perspective_rh(self.fovy_radians, self.aspect, self.z_near, self.z_far);

        projection * view
    }
}

fn calculate_aspect(width: u32, height: u32) -> f32 {
    let width = width.max(1) as f32;
    let height = height.max(1) as f32;

    width / height
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct CameraUniform {
    pub view_projection: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn from_camera(camera: &Camera) -> Self {
        Self {
            view_projection: camera.view_projection().to_cols_array_2d(),
        }
    }
}
