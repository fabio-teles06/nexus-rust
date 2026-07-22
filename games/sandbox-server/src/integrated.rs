use std::thread::{self, JoinHandle};

use engine_core::ClientId;
use engine_network::{LocalClientTransport, TransportError, local_transport_pair};
use engine_server::ServerRuntime;
use sandbox_shared::{ClientMessage, ServerMessage};

use crate::{
    config::{LOCAL_TRANSPORT_CAPACITY, SERVER_TICK_RATE},
    game::SandboxGame,
};

pub type SandboxClientTransport = LocalClientTransport<ClientMessage, ServerMessage>;

pub fn start_integrated_server() -> (
    SandboxClientTransport,
    JoinHandle<Result<(), TransportError>>,
) {
    let client_id = ClientId(1);

    let (client_transport, server_transport) =
        local_transport_pair(client_id, LOCAL_TRANSPORT_CAPACITY);

    let server_thread = thread::Builder::new()
        .name("integrated-server".to_string())
        .spawn(move || {
            let game = SandboxGame::new();

            ServerRuntime::new(game, server_transport, SERVER_TICK_RATE).run()
        })
        .expect("não foi possível iniciar o servidor integrado");

    (client_transport, server_thread)
}
