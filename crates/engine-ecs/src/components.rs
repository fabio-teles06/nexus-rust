use bevy_ecs::prelude::*;
use glam::{Mat4, Quat, Vec3};

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub const IDENTITY: Self = Self { translation: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE };
    pub fn from_translation(translation: Vec3) -> Self { Self { translation, ..Self::IDENTITY } }
    pub fn from_xyz(x: f32, y: f32, z: f32) -> Self { Self::from_translation(Vec3::new(x, y, z)) }
    pub fn compute_matrix(&self) -> Mat4 { Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation) }
    pub fn lerp(&self, other: &Self, alpha: f32) -> Self {
        let alpha = alpha.clamp(0.0, 1.0);
        Self {
            translation: self.translation.lerp(other.translation, alpha),
            rotation: self.rotation.slerp(other.rotation, alpha),
            scale: self.scale.lerp(other.scale, alpha),
        }
    }
}
impl Default for Transform { fn default() -> Self { Self::IDENTITY } }

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Velocity { pub linear: Vec3, pub angular: Vec3 }

/// Estado usado pelas regras locais e pela prediction.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct SimulationTransform(pub Transform);

/// Estado consumido exclusivamente pela extração de renderização.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct RenderTransform(pub Transform);
