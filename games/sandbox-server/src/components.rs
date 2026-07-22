use engine_core::ClientId;
use engine_ecs::prelude::*;
use sandbox_shared::NetworkId;

#[derive(Component, Debug)]
pub(crate) struct Player;

#[derive(Component, Debug)]
pub(crate) struct Collectible;

#[derive(Component, Debug, Clone)]
pub(crate) struct PlayerName(pub String);

#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct PlayerScore(pub u32);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerOwner(pub ClientId);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct NetworkEntity(pub NetworkId);

#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct LastInputSequence(pub u32);
