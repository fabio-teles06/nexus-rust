use engine_network::{LocalClientTransport, LocalNetworkHub, TransportError, local_network};
use engine_server::ServerRuntime;
use sandbox_shared::{ClientMessage, ServerMessage};
use std::thread::{self, JoinHandle};
use crate::{config::{CLIENT_CAPACITY, EVENT_CAPACITY, SERVER_TICK_RATE}, SandboxGame};

pub type SandboxNetworkHub = LocalNetworkHub<ClientMessage, ServerMessage>;
pub type SandboxClientTransport = LocalClientTransport<ClientMessage, ServerMessage>;

pub fn start_integrated_server() -> (SandboxNetworkHub, JoinHandle<Result<(), TransportError>>) {
    let (hub, transport) = local_network(EVENT_CAPACITY, CLIENT_CAPACITY);
    let thread = thread::Builder::new().name("integrated-server".into()).spawn(move || {
        ServerRuntime::new(SandboxGame::new(), transport, SERVER_TICK_RATE).run()
    }).expect("falha ao iniciar o servidor integrado");
    (hub, thread)
}
