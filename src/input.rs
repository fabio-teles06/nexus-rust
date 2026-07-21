use glam::Vec2;
use winit::{event::ElementState, keyboard::KeyCode};

#[derive(Debug, Default)]
pub struct InputState {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub sprint: bool,
    mouse_delta: Vec2,
}

impl InputState {
    pub fn process_key(&mut self, key: KeyCode, state: ElementState) -> bool {
        let pressed = state == ElementState::Pressed;

        match key {
            KeyCode::KeyW => self.forward = pressed,
            KeyCode::KeyS => self.backward = pressed,
            KeyCode::KeyA => self.left = pressed,
            KeyCode::KeyD => self.right = pressed,
            KeyCode::Space => self.up = pressed,
            KeyCode::ControlLeft => self.down = pressed,
            KeyCode::ShiftLeft => self.sprint = pressed,
            _ => return false,
        }

        true
    }

    pub fn add_mouse_delta(&mut self, delta: (f64, f64)) {
        self.mouse_delta += Vec2::new(delta.0 as f32, delta.1 as f32);
    }

    pub fn take_mouse_delta(&mut self) -> Vec2 {
        std::mem::take(&mut self.mouse_delta)
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}
