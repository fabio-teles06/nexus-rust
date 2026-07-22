use engine_core::{ClientId, Tick};

/// Mensagens enviadas pelo cliente ao servidor.
#[derive(Debug, Clone)]
pub enum ClientMessage {
    Join { player_name: String },

    Move { delta: f32 },

    Shutdown,
}

/// Mensagens enviadas pelo servidor ao cliente.
#[derive(Debug, Clone)]
pub enum ServerMessage {
    Welcome {
        client_id: ClientId,
        player_name: String,
    },

    PlayerPosition {
        position: f32,
    },

    ServerTick {
        tick: Tick,
    },

    Stopped,
}
