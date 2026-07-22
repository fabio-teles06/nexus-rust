use engine_core::{ClientId, Tick};
use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
)]
pub struct NetworkId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TransformSnapshot {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityKind {
    Player,
    Orb,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerInput {
    pub sequence: u32,
    pub direction: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerScoreSnapshot {
    pub player_name: String,
    pub score: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Join {
        player_name: String,
    },

    Input(PlayerInput),

    Leave,

    /// Usado somente pelo modo de servidor integrado.
    ShutdownServer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

    Scoreboard {
        players: Vec<PlayerScoreSnapshot>,
        target_score: u32,
    },

    RoundWon {
        player_name: String,
    },

    ServerTick {
        tick: Tick,
    },

    Stopped,
}
