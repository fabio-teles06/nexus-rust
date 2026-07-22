use engine_core::{ClientId, Tick};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetworkId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformSnapshot {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntityKind {
    Player,
}

/// Mensagens enviadas pelo cliente ao servidor.
#[derive(Debug, Clone)]
pub enum ClientMessage {
    Join { player_name: String },

    Move { direction: [f32; 3] },

    Shutdown,
}

/// Mensagens enviadas pelo servidor ao cliente.
#[derive(Debug, Clone)]
pub enum ServerMessage {
    Welcome {
        client_id: ClientId,
        player_entity: NetworkId,
        player_name: String,
    },

    SpawnEntity {
        network_id: NetworkId,
        transform: TransformSnapshot,
        kind: EntityKind,
    },

    UpdateTransform {
        network_id: NetworkId,
        server_tick: Tick,
        transform: TransformSnapshot,
    },

    DespawnEntity {
        network_id: NetworkId,
    },

    ServerTick {
        tick: Tick,
    },

    Stopped,
}

