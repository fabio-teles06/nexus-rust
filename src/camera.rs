use glam::{
    Mat4, Vec3,
    camera::rh::{proj::directx::perspective, view::look_at_mat4},
};

use crate::input::InputState;

pub struct Camera {
    position: Vec3,

    yaw: f32,
    pitch: f32,

    aspect: f32,
    fov_y: f32,
    near: f32,
    far: f32,

    movement_speed: f32,
    sprint_multiplier: f32,
    mouse_sensitivity: f32,
}

impl Camera {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            position: Vec3::ZERO,
            yaw: -90.0_f32.to_radians(),
            pitch: 0.0,

            aspect: calculate_aspect(width, height),
            fov_y: 45.0_f32.to_radians(),
            near: 0.1,
            far: 100.0,

            movement_speed: 5.0,
            sprint_multiplier: 2.0,
            mouse_sensitivity: 0.001,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = calculate_aspect(width, height);
    }

    pub fn update(&mut self, input: &mut InputState, delta_time: f32) {
        self.update_position(input, delta_time);
        self.update_rotation(input);
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn forward(&self) -> Vec3 {
        self.forward_direction()
    }

    fn update_rotation(&mut self, input: &mut InputState) {
        let mouse_delta = input.take_mouse_delta();

        self.yaw += mouse_delta.x * self.mouse_sensitivity;
        self.pitch -= mouse_delta.y * self.mouse_sensitivity;

        let pitch_limit = 89.0_f32.to_radians();
        self.pitch = self.pitch.clamp(-pitch_limit, pitch_limit);
    }

    fn update_position(&mut self, input: &InputState, delta_time: f32) {
        let forward = self.forward_direction();

        let horizontal_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let right = horizontal_forward.cross(Vec3::Y).normalize_or_zero();

        let mut movement = Vec3::ZERO;

        if input.forward {
            movement += horizontal_forward;
        }

        if input.backward {
            movement -= horizontal_forward;
        }

        if input.left {
            movement -= right;
        }

        if input.right {
            movement += right;
        }

        if input.up {
            movement += Vec3::Y;
        }

        if input.down {
            movement -= Vec3::Y;
        }

        if movement.length_squared() == 0.0 {
            return;
        }

        let mut speed = self.movement_speed;
        if input.sprint {
            speed *= self.sprint_multiplier;
        }

        self.position += movement.normalize() * speed * delta_time;
    }

    fn forward_direction(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize()
    }

    pub fn view_projection_matrix(&self) -> Mat4 {
        let direction = self.forward_direction();

        let target = self.position + direction;

        let view = look_at_mat4(self.position, target, Vec3::Y);

        let projection = perspective(self.fov_y, self.aspect, self.near, self.far);

        projection * view
    }
}

fn calculate_aspect(width: u32, height: u32) -> f32 {
    width.max(1) as f32 / height.max(1) as f32
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
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
        self.view_projection = camera.view_projection_matrix().to_cols_array_2d();
    }
}
