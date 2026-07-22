use engine_ecs::prelude::*;
use sandbox_shared::{EntityKind, NetworkId};

#[derive(Component, Debug)]
pub(crate) struct ReplicatedEntity;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ClientNetworkId(pub NetworkId);

#[derive(Component, Debug)]
pub(crate) struct LocalPlayer;

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct ClientEntityKind(pub EntityKind);
