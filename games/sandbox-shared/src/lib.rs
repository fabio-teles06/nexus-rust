use engine_core::{ClientId, Tick};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NetworkId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TransformSnapshot { pub translation: [f32; 3], pub rotation: [f32; 4], pub scale: [f32; 3] }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntitySnapshot {
    pub network_id: NetworkId,
    pub transform: TransformSnapshot,
    pub velocity: [f32; 3],
    pub last_processed_input: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntityKind { Player }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct SpawnSnapshot { pub network_id: NetworkId, pub kind: EntityKind, pub transform: TransformSnapshot }

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PlayerInput { pub sequence: u32, pub direction: [f32; 3] }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    Join { player_name: String },
    Input(PlayerInput),
    ShutdownServer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    Welcome { client_id: ClientId, player_entity: NetworkId, player_name: String },
    SpawnBatch(Vec<SpawnSnapshot>),
    SnapshotBatch { server_tick: Tick, entities: Vec<EntitySnapshot> },
    DespawnBatch(Vec<NetworkId>),
    Stopped,
}

pub fn encode_client(message: &ClientMessage) -> Result<Vec<u8>, postcard::Error> { postcard::to_allocvec(message) }
pub fn decode_client(bytes: &[u8]) -> Result<ClientMessage, postcard::Error> { postcard::from_bytes(bytes) }
pub fn encode_server(message: &ServerMessage) -> Result<Vec<u8>, postcard::Error> { postcard::to_allocvec(message) }
pub fn decode_server(bytes: &[u8]) -> Result<ServerMessage, postcard::Error> { postcard::from_bytes(bytes) }
