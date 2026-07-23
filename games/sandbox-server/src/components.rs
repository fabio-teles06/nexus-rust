use bevy_ecs::prelude::*;
use engine_core::ClientId;
use engine_physics::PhysicsBody;
use glam::Vec3;
use sandbox_shared::{NetworkId, PlayerInput};
use std::collections::VecDeque;

#[derive(Component, Debug)]
pub(crate) struct Player;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerOwner(pub ClientId);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct NetworkEntity(pub NetworkId);

#[derive(Component, Debug)]
pub(crate) struct PlayerInputState {
    pub current_direction: Vec3,
    pub pending: VecDeque<PlayerInput>,
    pub last_processed: u32,
}

impl Default for PlayerInputState {
    fn default() -> Self {
        Self {
            current_direction: Vec3::ZERO,
            pending: VecDeque::new(),
            last_processed: 0,
        }
    }
}

#[derive(Bundle)]
pub(crate) struct PlayerBundle {
    pub marker: Player,
    pub owner: PlayerOwner,
    pub network: NetworkEntity,
    pub transform: engine_ecs::Transform,
    pub physics_body: PhysicsBody,
    pub input: PlayerInputState,
}
