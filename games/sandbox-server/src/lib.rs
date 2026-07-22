use engine_core::{ClientId, Tick};

use engine_network::{LocalClientTransport, TransportError, local_transport_pair};

use engine_server::{ServerGame, ServerRuntime};

use sandbox_shared::{ClientMessage, ServerMessage};

use std::thread::{self, JoinHandle};

pub type SandboxClientTransport = LocalClientTransport<ClientMessage, ServerMessage>;

pub struct SandboxGame {
    running: bool,
    player: Option<ClientId>,
    player_name: Option<String>,
    player_position: f32,

    outgoing: Vec<(ClientId, ServerMessage)>,
}

impl SandboxGame {
    pub fn new() -> Self {
        Self {
            running: true,
            player: None,
            player_name: None,
            player_position: 0.0,
            outgoing: Vec::new(),
        }
    }

    fn send(&mut self, client_id: ClientId, message: ServerMessage) {
        self.outgoing.push((client_id, message));
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

    fn handle_message(&mut self, client_id: ClientId, message: Self::ClientMessage) {
        match message {
            ClientMessage::Join { player_name } => {
                println!("[servidor] jogador conectado: {player_name}");

                self.player = Some(client_id);
                self.player_name = Some(player_name.clone());

                self.send(
                    client_id,
                    ServerMessage::Welcome {
                        client_id,
                        player_name,
                    },
                );

                self.send(
                    client_id,
                    ServerMessage::PlayerPosition {
                        position: self.player_position,
                    },
                );
            }

            ClientMessage::Move { delta } => {
                if self.player != Some(client_id) {
                    return;
                }

                /*
                 * O servidor limita o movimento.
                 *
                 * Mesmo que o cliente envie 5000, ele só poderá
                 * movimentar uma unidade por comando.
                 */
                let validated_delta = delta.clamp(-1.0, 1.0);

                self.player_position += validated_delta;

                println!("[servidor] posição atual: {}", self.player_position);

                self.send(
                    client_id,
                    ServerMessage::PlayerPosition {
                        position: self.player_position,
                    },
                );
            }

            ClientMessage::Shutdown => {
                println!("[servidor] desligamento solicitado");

                self.running = false;

                self.send(client_id, ServerMessage::Stopped);
            }
        }
    }

    fn update(&mut self, tick: Tick) {
        /*
         * Apenas para demonstrar que o servidor está
         * executando com ticks fixos.
         */
        if tick.0 % 30 == 0 {
            if let Some(client_id) = self.player {
                self.send(client_id, ServerMessage::ServerTick { tick });
            }
        }
    }

    fn drain_outgoing(&mut self) -> Vec<(ClientId, Self::ServerMessage)> {
        std::mem::take(&mut self.outgoing)
    }

    fn is_running(&self) -> bool {
        self.running
    }
}

/// Inicia o servidor dentro do processo do cliente.
///
/// A simulação é executada em outra thread.
pub fn start_integrated_server() -> (
    SandboxClientTransport,
    JoinHandle<Result<(), TransportError>>,
) {
    let client_id = ClientId(1);

    let (client_transport, server_transport) = local_transport_pair(client_id, 256);

    let server_thread = thread::Builder::new()
        .name("integrated-server".to_string())
        .spawn(move || {
            println!("[servidor] servidor integrado iniciado");

            let game = SandboxGame::new();

            let runtime = ServerRuntime::new(game, server_transport, 30);

            let result = runtime.run();

            println!("[servidor] servidor finalizado");

            result
        })
        .expect("não foi possível criar a thread do servidor");

    (client_transport, server_thread)
}
