use engine_core::{ClientId, Tick};
use engine_ecs::prelude::*;
use engine_server::ServerGame;
use sandbox_shared::{ClientMessage, ServerMessage};

use crate::{
    components::{Collectible, NetworkEntity},
    config::{ORB_SPAWN_POINTS, SERVER_TICK_RATE},
    resources::{
        NetworkIdGenerator, OrbRespawnState, OutgoingMessages,
        PendingClientMessages, PlayerRegistry, ServerSettings, ServerState,
        SimulationTime,
    },
    systems::{
        collect_orb_system, movement_system, process_client_messages,
        replicate_changed_transforms, send_periodic_tick,
    },
};

pub struct SandboxGame {
    world: World,
    schedule: Schedule,
}

impl SandboxGame {
    pub fn integrated() -> Self {
        Self::new(true)
    }

    pub fn dedicated() -> Self {
        Self::new(false)
    }

    fn new(allow_shutdown: bool) -> Self {
        let mut world = World::new();

        let mut network_ids = NetworkIdGenerator::default();
        let orb_network_id = network_ids.generate();

        world.spawn((
            Collectible,
            NetworkEntity(orb_network_id),
            Transform {
                translation: ORB_SPAWN_POINTS[0],
                rotation: Quat::IDENTITY,
                scale: Vec3::splat(0.7),
            },
        ));

        world.insert_resource(PendingClientMessages::default());
        world.insert_resource(OutgoingMessages::default());
        world.insert_resource(ServerState::default());
        world.insert_resource(ServerSettings { allow_shutdown });
        world.insert_resource(network_ids);
        world.insert_resource(PlayerRegistry::default());
        world.insert_resource(OrbRespawnState::default());
        world.insert_resource(SimulationTime::new(SERVER_TICK_RATE));

        let mut schedule = Schedule::default();

        schedule.add_systems(
            (
                process_client_messages,
                movement_system,
                collect_orb_system,
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
        Self::dedicated()
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
