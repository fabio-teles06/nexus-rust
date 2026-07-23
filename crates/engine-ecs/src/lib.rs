pub use bevy_ecs;
pub use glam;

mod components;
pub use components::*;

pub mod prelude {
    pub use bevy_ecs::prelude::*;
    pub use glam::{EulerRot, IVec2, IVec3, Mat3, Mat4, Quat, UVec2, UVec3, Vec2, Vec3, Vec4};
    pub use crate::{RenderTransform, SimulationTransform, Transform, Velocity};
}
