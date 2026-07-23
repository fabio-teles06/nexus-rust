use bevy_ecs::prelude::*;
use glam::{Quat, Vec3};
use rapier3d::prelude::{ColliderHandle, RigidBodyHandle};

/// Liga uma entidade ECS ao corpo e ao collider armazenados no mundo do Rapier.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysicsBody {
    pub rigid_body: RigidBodyHandle,
    pub collider: ColliderHandle,
}

/// Estado extraído do Rapier após um passo da simulação.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicsBodyState {
    pub translation: Vec3,
    pub rotation: Quat,
    pub linear_velocity: Vec3,
}
