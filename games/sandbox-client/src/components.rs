use bevy_ecs::prelude::*;
use engine_assets::{Handle, MaterialAsset, MeshAsset};
use engine_core::Tick;
use engine_ecs::Transform;
use sandbox_shared::{EntityKind, NetworkId};
use std::time::Instant;

#[derive(Component, Debug)] pub(crate) struct ReplicatedEntity;
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)] pub(crate) struct ClientNetworkId(pub NetworkId);
#[derive(Component, Debug)] pub(crate) struct LocalPlayer;
#[derive(Component, Debug, Clone, Copy)] pub(crate) struct ClientEntityKind(pub EntityKind);
#[derive(Component, Debug, Clone, Copy)] pub(crate) struct MeshHandle(pub Handle<MeshAsset>);
#[derive(Component, Debug, Clone, Copy)] pub(crate) struct MaterialHandle(pub Handle<MaterialAsset>);

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct NetworkTransform {
    pub previous: Transform,
    pub current: Transform,
    pub previous_tick: Tick,
    pub current_tick: Tick,
    pub received_at: Instant,
}
