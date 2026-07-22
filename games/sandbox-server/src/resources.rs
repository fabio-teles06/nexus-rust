use std::collections::HashMap;

use engine_core::{ClientId, Tick};
use engine_ecs::prelude::*;
use sandbox_shared::{ClientMessage, NetworkId, ServerMessage};

#[derive(Resource, Default)]
pub(crate) struct PendingClientMessages {
    pub messages: Vec<(ClientId, ClientMessage)>,
}

#[derive(Resource, Default)]
pub(crate) struct OutgoingMessages {
    pub messages: Vec<(ClientId, ServerMessage)>,
}

#[derive(Resource)]
pub(crate) struct ServerState {
    pub running: bool,
}

impl Default for ServerState {
    fn default() -> Self {
        Self { running: true }
    }
}

#[derive(Resource, Debug, Clone, Copy)]
pub(crate) struct SimulationTime {
    pub tick: Tick,
    pub delta_seconds: f32,
}

impl SimulationTime {
    pub fn new(tick_rate: u32) -> Self {
        assert!(tick_rate > 0);

        Self {
            tick: Tick(0),
            delta_seconds: 1.0 / tick_rate as f32,
        }
    }
}

#[derive(Resource)]
pub(crate) struct NetworkIdGenerator {
    next_id: u64,
}

impl Default for NetworkIdGenerator {
    fn default() -> Self {
        Self { next_id: 1 }
    }
}

impl NetworkIdGenerator {
    pub fn generate(&mut self) -> NetworkId {
        let id = NetworkId(self.next_id);
        self.next_id += 1;
        id
    }
}

#[derive(Resource, Default)]
pub(crate) struct PlayerRegistry {
    pub players: HashMap<ClientId, NetworkId>,
}
