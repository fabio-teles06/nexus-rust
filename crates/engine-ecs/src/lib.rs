pub use bevy_ecs;
pub use glam;

pub mod components;
pub mod resources;
pub mod schedules;

pub mod prelude {
    pub use bevy_ecs::prelude::*;

    pub use glam::{
        Affine3A, EulerRot, IVec2, IVec3, Mat3, Mat4, Quat, UVec2, UVec3, Vec2, Vec3, Vec3A, Vec4,
    };

    pub use crate::components::*;
    pub use crate::resources::*;
    pub use crate::schedules::*;
}
