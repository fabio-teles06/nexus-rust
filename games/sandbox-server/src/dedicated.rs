use engine_network::{TcpServerTransport, TransportError};
use engine_server::ServerRuntime;
use sandbox_shared::{ClientMessage, ServerMessage};

use crate::{
    config::{NETWORK_TRANSPORT_CAPACITY, SERVER_TICK_RATE},
    game::SandboxGame,
};

pub fn run_dedicated_server(address: &str) -> Result<(), TransportError> {
    let transport =
        TcpServerTransport::<ClientMessage, ServerMessage>::bind_with_disconnect(
            address,
            NETWORK_TRANSPORT_CAPACITY,
            ClientMessage::Leave,
        )?;

    println!("[servidor] Nexus Arena escutando em {address}");
    println!("[servidor] objetivo: colete o cubo dourado 5 vezes");

    ServerRuntime::new(
        SandboxGame::dedicated(),
        transport,
        SERVER_TICK_RATE,
    )
    .run()
}
