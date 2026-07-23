use bevy_ecs::prelude::*;
use engine_core::{ClientId, Tick};
use engine_server::{Outgoing, ServerGame};
use sandbox_shared::{ClientMessage, ServerMessage};
use crate::{config::SERVER_TICK_RATE, resources::{insert_resources, Outbox, PendingMessages, ServerState, SimulationTime}, systems::{physics_movement, process_connected, process_disconnected, process_messages, replicate_snapshot_batch}};

pub struct SandboxGame { world: World, fixed_schedule: Schedule, post_schedule: Schedule }
impl SandboxGame {
    pub fn new() -> Self {
        let mut world = World::new();
        insert_resources(&mut world, SERVER_TICK_RATE);
        let mut fixed_schedule = Schedule::default();
        fixed_schedule.add_systems(physics_movement);
        let mut post_schedule = Schedule::default();
        post_schedule.add_systems(replicate_snapshot_batch);
        Self { world, fixed_schedule, post_schedule }
    }
}
impl Default for SandboxGame { fn default() -> Self { Self::new() } }
impl ServerGame for SandboxGame {
    type ClientMessage = ClientMessage;
    type ServerMessage = ServerMessage;
    fn connected(&mut self, id: ClientId) { process_connected(id, &mut self.world); }
    fn disconnected(&mut self, id: ClientId) { process_disconnected(id, &mut self.world); }
    fn handle_message(&mut self, id: ClientId, message: ClientMessage) { self.world.resource_mut::<PendingMessages>().0.push((id, message)); }
    fn update(&mut self, tick: Tick) {
        self.world.resource_mut::<SimulationTime>().tick = tick;
        process_messages(&mut self.world);
        self.fixed_schedule.run(&mut self.world);
        self.post_schedule.run(&mut self.world);
    }
    fn drain_outgoing(&mut self) -> Vec<Outgoing<ServerMessage>> { std::mem::take(&mut self.world.resource_mut::<Outbox>().0) }
    fn is_running(&self) -> bool { self.world.resource::<ServerState>().running }
}
