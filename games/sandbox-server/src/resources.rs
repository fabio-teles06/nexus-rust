use bevy_ecs::prelude::*;
use engine_core::{ClientId, Tick};
use engine_physics::PhysicsWorld;
use engine_server::Outgoing;
use sandbox_shared::{ClientMessage, NetworkId, ServerMessage};
use std::collections::{HashMap, HashSet};

#[derive(Resource, Default)] pub(crate) struct PendingMessages(pub Vec<(ClientId, ClientMessage)>);
#[derive(Resource, Default)] pub(crate) struct Outbox(pub Vec<Outgoing<ServerMessage>>);
#[derive(Resource, Default)] pub(crate) struct ConnectedClients(pub HashSet<ClientId>);
#[derive(Resource, Default)] pub(crate) struct PlayerRegistry(pub HashMap<ClientId, NetworkId>);
#[derive(Resource)] pub(crate) struct ServerState { pub running: bool }
impl Default for ServerState { fn default() -> Self { Self { running: true } } }
#[derive(Resource)] pub(crate) struct NetworkIdGenerator(pub u64);
impl Default for NetworkIdGenerator { fn default() -> Self { Self(1) } }
impl NetworkIdGenerator { pub fn next(&mut self) -> NetworkId { let id = NetworkId(self.0); self.0 += 1; id } }
#[derive(Resource, Clone, Copy)] pub(crate) struct SimulationTime { pub tick: Tick, pub delta_seconds: f32 }
impl SimulationTime { pub fn new(rate: u32) -> Self { Self { tick: Tick(0), delta_seconds: 1.0 / rate as f32 } } }

pub(crate) fn insert_resources(world: &mut World, tick_rate: u32) {
    world.insert_resource(PendingMessages::default());
    world.insert_resource(Outbox::default());
    world.insert_resource(ConnectedClients::default());
    world.insert_resource(PlayerRegistry::default());
    world.insert_resource(ServerState::default());
    world.insert_resource(NetworkIdGenerator::default());
    world.insert_resource(SimulationTime::new(tick_rate));
    world.insert_resource(PhysicsWorld::default());
}
