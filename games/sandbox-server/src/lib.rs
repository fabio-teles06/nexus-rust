use engine_core::{ClientId, Tick};
use engine_ecs::prelude::*;
use engine_network::{LocalClientTransport, TransportError, local_transport_pair};
use engine_server::{ServerGame, ServerRuntime};
use sandbox_shared::{ClientMessage, ServerMessage};
use std::thread::{self, JoinHandle};

pub type SandboxClientTransport = LocalClientTransport<ClientMessage, ServerMessage>;

const SERVER_TICK_RATE: u32 = 30;
const LOCAL_TRANSPORT_CAPACITY: usize = 256;

#[derive(Component, Debug)]
pub struct Player;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerOwner(pub ClientId);

#[derive(Resource, Default)]
struct PendingClientMessages {
    messages: Vec<(ClientId, ClientMessage)>
}

#[derive(Resource, Default)]
struct OutgoingMessages {
    messages: Vec<(ClientId, ServerMessage)>,
}

#[derive(Resource)]
struct ServerState {
    running: bool
}

impl Default for ServerState {
    fn default() -> Self {
        Self { running: true }
    }
}

#[derive(Resource, Debug, Clone, Copy)]
pub struct SimulationTime {
    pub tick: Tick,
    pub delta_seconds: f32,
}

impl SimulationTime {
    pub fn new(tick_rate: u32) -> Self {
        assert!(
            tick_rate > 0,
            "a taxa de ticks deve ser maior que zero"
        );

        Self {
            tick: Tick(0),
            delta_seconds: 1.0 / tick_rate as f32,
        }
    }
}

pub struct SandboxGame {
    world: World,
    schedule: Schedule,
}

impl SandboxGame {
    pub fn new() -> Self {
        let mut world = World::new();

        world.insert_resource(PendingClientMessages::default());
        world.insert_resource(OutgoingMessages::default());
        world.insert_resource(ServerState::default());
        world.insert_resource(SimulationTime::new(
            SERVER_TICK_RATE,
        ));

        let mut schedule = Schedule::default();

        /*
         * A ordem é importante:
         *
         * 1. processa comandos;
         * 2. atualiza movimentos contínuos;
         * 3. envia informações periódicas.
         */
        schedule.add_systems(
            (
                process_client_messages,
                movement_system,
                send_periodic_tick,
            )
                .chain(),
        );

        Self { world, schedule }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn world_mut(&mut self) -> &mut World {
        &mut self.world
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

    fn handle_message(
        &mut self,
        client_id: ClientId,
        message: Self::ClientMessage,
    ) {
        self.world
            .resource_mut::<PendingClientMessages>()
            .messages
            .push((client_id, message));
    }

    fn update(&mut self, tick: Tick) {
        {
            let mut simulation_time =
                self.world.resource_mut::<SimulationTime>();

            simulation_time.tick = tick;
        }

        self.schedule.run(&mut self.world);
    }

    fn drain_outgoing(
        &mut self,
    ) -> Vec<(ClientId, Self::ServerMessage)> {
        let mut outgoing =
            self.world.resource_mut::<OutgoingMessages>();

        std::mem::take(&mut outgoing.messages)
    }

    fn is_running(&self) -> bool {
        self.world.resource::<ServerState>().running
    }
}

/// Processa as mensagens enviadas pelos clientes.
fn process_client_messages(
    mut commands: Commands,
    mut pending: ResMut<PendingClientMessages>,
    mut outgoing: ResMut<OutgoingMessages>,
    mut server_state: ResMut<ServerState>,
    mut players: Query<
        (&PlayerOwner, &mut Transform),
        With<Player>,
    >,
) {
    let messages = std::mem::take(&mut pending.messages);

    for (client_id, message) in messages {
        match message {
            ClientMessage::Join { player_name } => {
                handle_join(
                    &mut commands,
                    &mut players,
                    &mut outgoing,
                    client_id,
                    player_name,
                );
            }

            ClientMessage::Move { delta } => {
                handle_movement_command(
                    &mut players,
                    &mut outgoing,
                    client_id,
                    delta,
                );
            }

            ClientMessage::Shutdown => {
                println!(
                    "[servidor] desligamento solicitado pelo cliente {}",
                    client_id.0
                );

                server_state.running = false;

                outgoing.messages.push((
                    client_id,
                    ServerMessage::Stopped,
                ));
            }
        }
    }
}

fn handle_join(
    commands: &mut Commands,
    players: &mut Query<
        (&PlayerOwner, &mut Transform),
        With<Player>,
    >,
    outgoing: &mut OutgoingMessages,
    client_id: ClientId,
    player_name: String,
) {
    let already_connected = players
        .iter_mut()
        .any(|(owner, _)| owner.0 == client_id);

    if already_connected {
        println!(
            "[servidor] cliente {} já possui um jogador",
            client_id.0
        );

        return;
    }

    let transform = Transform::from_xyz(0.0, 0.0, 0.0);

    commands.spawn((
        Player,
        PlayerOwner(client_id),
        transform,
        Velocity::default(),
    ));

    println!(
        "[servidor] jogador conectado: {} ({})",
        player_name,
        client_id.0
    );

    outgoing.messages.push((
        client_id,
        ServerMessage::Welcome {
            client_id,
            player_name,
        },
    ));

    /*
     * O protocolo atual utiliza somente um f32.
     * Portanto, enviamos a coordenada X.
     */
    outgoing.messages.push((
        client_id,
        ServerMessage::PlayerPosition {
            position: transform.translation.x,
        },
    ));
}

/// Valida e aplica um comando pontual de movimento.
fn handle_movement_command(
    players: &mut Query<
        (&PlayerOwner, &mut Transform),
        With<Player>,
    >,
    outgoing: &mut OutgoingMessages,
    client_id: ClientId,
    requested_delta: f32,
) {
    let validated_delta = requested_delta.clamp(-1.0, 1.0);

    for (owner, mut transform) in players.iter_mut() {
        if owner.0 != client_id {
            continue;
        }

        transform.translation.x += validated_delta;

        println!(
            "[servidor] jogador {} está na posição {:?}",
            client_id.0,
            transform.translation
        );

        outgoing.messages.push((
            client_id,
            ServerMessage::PlayerPosition {
                position: transform.translation.x,
            },
        ));

        return;
    }

    println!(
        "[servidor] movimento ignorado: cliente {} não possui jogador",
        client_id.0
    );
}

/// Sistema preparado para movimentação baseada em velocidade.
///
/// Por enquanto, as entidades começam com velocidade zero.
/// Posteriormente, os inputs alterarão a velocidade em vez de
/// modificarem diretamente o Transform.
fn movement_system(
    time: Res<SimulationTime>,
    mut query: Query<(&mut Transform, &Velocity)>,
) {
    for (mut transform, velocity) in &mut query {
        transform.translation +=
            velocity.linear * time.delta_seconds;

        if velocity.angular != Vec3::ZERO {
            let angular_displacement =
                velocity.angular * time.delta_seconds;

            let rotation = Quat::from_euler(
                EulerRot::XYZ,
                angular_displacement.x,
                angular_displacement.y,
                angular_displacement.z,
            );

            transform.rotation =
                (rotation * transform.rotation).normalize();
        }
    }
}

/// Envia o tick do servidor uma vez por segundo.
fn send_periodic_tick(
    time: Res<SimulationTime>,
    players: Query<&PlayerOwner, With<Player>>,
    mut outgoing: ResMut<OutgoingMessages>,
) {
    if time.tick.0 == 0
        || !time
            .tick
            .0
            .is_multiple_of(SERVER_TICK_RATE as u64)
    {
        return;
    }

    for owner in &players {
        outgoing.messages.push((
            owner.0,
            ServerMessage::ServerTick {
                tick: time.tick,
            },
        ));
    }
}

/// Inicializa o servidor integrado em outra thread.
pub fn start_integrated_server() -> (
    SandboxClientTransport,
    JoinHandle<Result<(), TransportError>>,
) {
    let client_id = ClientId(1);

    let (client_transport, server_transport) =
        local_transport_pair(
            client_id,
            LOCAL_TRANSPORT_CAPACITY,
        );

    let server_thread = thread::Builder::new()
        .name("integrated-server".to_string())
        .spawn(move || {
            println!(
                "[servidor] servidor integrado iniciado em {} TPS",
                SERVER_TICK_RATE
            );

            let game = SandboxGame::new();

            let runtime = ServerRuntime::new(
                game,
                server_transport,
                SERVER_TICK_RATE,
            );

            let result = runtime.run();

            match &result {
                Ok(()) => {
                    println!(
                        "[servidor] servidor finalizado corretamente"
                    );
                }

                Err(error) => {
                    eprintln!(
                        "[servidor] erro ao executar servidor: {error}"
                    );
                }
            }

            result
        })
        .expect(
            "não foi possível criar a thread do servidor integrado",
        );

    (client_transport, server_thread)
}