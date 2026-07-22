use engine_core::{ClientId, Tick};
use engine_ecs::prelude::*;
use engine_server::ServerGame;
use sandbox_shared::{ClientMessage, ServerMessage};

use crate::{
    config::SERVER_TICK_RATE,
    resources::{
        NetworkIdGenerator, OutgoingMessages, PendingClientMessages, PlayerRegistry, ServerState,
        SimulationTime,
    },
    systems::{
        movement_system, process_client_messages, replicate_changed_transforms, send_periodic_tick,
    },
};

pub struct SandboxGame {
    world: World,
    schedule: Schedule,
}

impl SandboxGame {
    pub fn new() -> Self {
        let mut world = World::new();

        world.insert_resource(PendingClientMessages::default());
        world.insert_resource(OutgoingMessages::default());
        world.insert_resource(ServerState::default());
        world.insert_resource(NetworkIdGenerator::default());
        world.insert_resource(PlayerRegistry::default());
        world.insert_resource(SimulationTime::new(SERVER_TICK_RATE));

        let mut schedule = Schedule::default();

        schedule.add_systems(
            (
                process_client_messages,
                movement_system,
                replicate_changed_transforms,
                send_periodic_tick,
            )
                .chain(),
        );

        Self { world, schedule }
    }
}

impl Default for SandboxGame {
    fn default() -> Self {
        Self::new()
    }
}

impl ServerGame for SandboxGame {
    type ClientMessage = ClientMessage;
    type ServerMessage = ServerMessage;

    fn handle_message(&mut self, client_id: ClientId, message: ClientMessage) {
        self.world
            .resource_mut::<PendingClientMessages>()
            .messages
            .push((client_id, message));
    }

    fn update(&mut self, tick: Tick) {
        self.world.resource_mut::<SimulationTime>().tick = tick;

        self.schedule.run(&mut self.world);
    }

    fn drain_outgoing(&mut self) -> Vec<(ClientId, ServerMessage)> {
        let mut outgoing = self.world.resource_mut::<OutgoingMessages>();

        std::mem::take(&mut outgoing.messages)
    }

    fn is_running(&self) -> bool {
        self.world.resource::<ServerState>().running
    }
}
