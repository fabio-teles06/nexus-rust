use glam::{
    camera::rh::{proj::directx::perspective, view::look_at_mat4},
    Mat4, Vec3,
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
            position: Vec3::new(0.0, 38.0, 24.0),
            yaw: -90.0_f32.to_radians(),
            pitch: -28.0_f32.to_radians(),
            aspect: aspect(width, height),
            fov_y: 70.0_f32.to_radians(),
            near: 0.05,
            far: 1500.0,
            movement_speed: 12.0,
            sprint_multiplier: 3.0,
            mouse_sensitivity: 0.0023,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = aspect(width, height);
    }

    pub fn update(&mut self, input: &mut InputState, delta_time: f32) {
        let mouse = input.take_mouse_delta();
        self.yaw += mouse.x * self.mouse_sensitivity;
        self.pitch -= mouse.y * self.mouse_sensitivity;
        self.pitch = self.pitch.clamp(
            -89.0_f32.to_radians(),
            89.0_f32.to_radians(),
        );

        let forward = self.forward();
        let horizontal_forward = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
        let right = horizontal_forward.cross(Vec3::Y).normalize_or_zero();
        let mut movement = Vec3::ZERO;

        if input.forward {
            movement += horizontal_forward;
        }
        if input.backward {
            movement -= horizontal_forward;
        }
        if input.right {
            movement += right;
        }
        if input.left {
            movement -= right;
        }
        if input.up {
            movement += Vec3::Y;
        }
        if input.down {
            movement -= Vec3::Y;
        }

        if movement.length_squared() > 0.0 {
            let speed = self.movement_speed
                * if input.sprint {
                    self.sprint_multiplier
                } else {
                    1.0
                };
            self.position += movement.normalize() * speed * delta_time;
        }
    }

    pub fn position(&self) -> Vec3 {
        self.position
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize()
    }

    pub fn view_projection(&self) -> Mat4 {
        let view = look_at_mat4(self.position, self.position + self.forward(), Vec3::Y);
        let projection = perspective(self.fov_y, self.aspect, self.near, self.far);
        projection * view
    }
}

fn aspect(width: u32, height: u32) -> f32 {
    width.max(1) as f32 / height.max(1) as f32
}
