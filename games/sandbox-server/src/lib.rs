use std::{
    collections::HashMap,
    thread::{self, JoinHandle},
};

use engine_core::{ClientId, Tick};
use engine_ecs::prelude::*;
use engine_network::{
    LocalClientTransport,
    TransportError,
    local_transport_pair,
};
use engine_server::{ServerGame, ServerRuntime};
use sandbox_shared::{
    ClientMessage,
    EntityKind,
    NetworkId,
    ServerMessage,
    TransformSnapshot,
};

pub type SandboxClientTransport =
    LocalClientTransport<ClientMessage, ServerMessage>;

const SERVER_TICK_RATE: u32 = 30;
const LOCAL_TRANSPORT_CAPACITY: usize = 256;
const PLAYER_SPEED: f32 = 4.0;

/// Marca uma entidade como jogador.
#[derive(Component, Debug)]
pub struct Player;

/// Indica qual cliente controla a entidade.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerOwner(pub ClientId);

/// ID estável usado na replicação entre servidor e cliente.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NetworkEntity(pub NetworkId);

/// Mensagens recebidas pelo transporte e ainda não processadas
/// pelos sistemas ECS.
#[derive(Resource, Default)]
struct PendingClientMessages {
    messages: Vec<(ClientId, ClientMessage)>,
}

/// Mensagens produzidas pelos sistemas ECS.
#[derive(Resource, Default)]
struct OutgoingMessages {
    messages: Vec<(ClientId, ServerMessage)>,
}

/// Controla a execução do servidor.
#[derive(Resource)]
struct ServerState {
    running: bool,
}

impl Default for ServerState {
    fn default() -> Self {
        Self { running: true }
    }
}

/// Informações temporais da simulação.
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

/// Gera IDs estáveis para entidades replicadas.
#[derive(Resource)]
struct NetworkIdGenerator {
    next_id: u64,
}

impl Default for NetworkIdGenerator {
    fn default() -> Self {
        Self { next_id: 1 }
    }
}

impl NetworkIdGenerator {
    fn generate(&mut self) -> NetworkId {
        let network_id = NetworkId(self.next_id);

        self.next_id += 1;

        network_id
    }
}

/// Registro dos jogadores conectados.
///
/// Esse registro também impede que duas mensagens Join no mesmo
/// tick criem duas entidades para o mesmo cliente.
#[derive(Resource, Default)]
struct PlayerRegistry {
    players: HashMap<ClientId, NetworkId>,
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
        world.insert_resource(NetworkIdGenerator::default());
        world.insert_resource(PlayerRegistry::default());
        world.insert_resource(SimulationTime::new(
            SERVER_TICK_RATE,
        ));

        let mut schedule = Schedule::default();

        /*
         * A ordem é importante:
         *
         * 1. processar mensagens;
         * 2. movimentar entidades;
         * 3. replicar transformações alteradas;
         * 4. enviar informações periódicas.
         */
        schedule.add_systems(
            (
                process_client_messages,
                movement_system,
                replicate_changed_transforms,
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

fn process_client_messages(
    mut commands: Commands,
    mut pending: ResMut<PendingClientMessages>,
    mut outgoing: ResMut<OutgoingMessages>,
    mut server_state: ResMut<ServerState>,
    mut network_ids: ResMut<NetworkIdGenerator>,
    mut player_registry: ResMut<PlayerRegistry>,
    mut players: Query<
        (&PlayerOwner, &mut Velocity),
        With<Player>,
    >,
) {
    let messages = std::mem::take(&mut pending.messages);

    for (client_id, message) in messages {
        match message {
            ClientMessage::Join { player_name } => {
                handle_join(
                    &mut commands,
                    &mut outgoing,
                    &mut network_ids,
                    &mut player_registry,
                    client_id,
                    player_name,
                );
            }

            ClientMessage::Move { direction } => {
                handle_move(
                    &mut players,
                    client_id,
                    direction,
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
    outgoing: &mut OutgoingMessages,
    network_ids: &mut NetworkIdGenerator,
    player_registry: &mut PlayerRegistry,
    client_id: ClientId,
    player_name: String,
) {
    if player_registry.players.contains_key(&client_id) {
        println!(
            "[servidor] cliente {} já possui um jogador",
            client_id.0
        );

        return;
    }

    let network_id = network_ids.generate();
    let transform = Transform::from_xyz(0.0, 0.0, 0.0);

    commands.spawn((
        Player,
        PlayerOwner(client_id),
        NetworkEntity(network_id),
        transform,
        Velocity::default(),
    ));

    player_registry
        .players
        .insert(client_id, network_id);

    println!(
        "[servidor] jogador conectado: {} | cliente={} | entidade={}",
        player_name,
        client_id.0,
        network_id.0
    );

    outgoing.messages.push((
        client_id,
        ServerMessage::Welcome {
            client_id,
            player_entity: network_id,
            player_name,
        },
    ));

    outgoing.messages.push((
        client_id,
        ServerMessage::SpawnEntity {
            network_id,
            kind: EntityKind::Player,
            transform: snapshot_from_transform(&transform),
        },
    ));
}

fn handle_move(
    players: &mut Query<
        (&PlayerOwner, &mut Velocity),
        With<Player>,
    >,
    client_id: ClientId,
    direction: [f32; 3],
) {
    let requested_direction = Vec3::from_array(direction);

    let validated_direction =
        validate_movement_direction(requested_direction);

    for (owner, mut velocity) in players.iter_mut() {
        if owner.0 != client_id {
            continue;
        }

        velocity.linear =
            validated_direction * PLAYER_SPEED;

        println!(
            "[servidor] cliente {} definiu velocidade para {:?}",
            client_id.0,
            velocity.linear
        );

        return;
    }

    println!(
        "[servidor] movimento ignorado: cliente {} não possui jogador",
        client_id.0
    );
}

/// Impede vetores inválidos, NaN e movimento acima da
/// intensidade máxima permitida.
fn validate_movement_direction(
    direction: Vec3,
) -> Vec3 {
    if !direction.is_finite() {
        return Vec3::ZERO;
    }

    let length_squared = direction.length_squared();

    if length_squared <= f32::EPSILON {
        return Vec3::ZERO;
    }

    if length_squared > 1.0 {
        return direction.normalize();
    }

    direction
}

fn movement_system(
    time: Res<SimulationTime>,
    mut players: Query<
        (&mut Transform, &Velocity),
        With<Player>,
    >,
) {
    for (mut transform, velocity) in &mut players {
        if velocity.linear == Vec3::ZERO {
            continue;
        }

        transform.translation +=
            velocity.linear * time.delta_seconds;
    }
}

fn replicate_changed_transforms(
    time: Res<SimulationTime>,
    players: Query<
        (
            &PlayerOwner,
            &NetworkEntity,
            &Transform,
        ),
        (With<Player>, Changed<Transform>),
    >,
    mut outgoing: ResMut<OutgoingMessages>,
) {
    for (owner, network_entity, transform) in &players {
        outgoing.messages.push((
            owner.0,
            ServerMessage::UpdateTransform {
                network_id: network_entity.0,
                server_tick: time.tick,
                transform: snapshot_from_transform(transform),
            },
        ));
    }
}

fn send_periodic_tick(
    time: Res<SimulationTime>,
    players: Query<&PlayerOwner, With<Player>>,
    mut outgoing: ResMut<OutgoingMessages>,
) {
    if time.tick.0 == 0
        || time.tick.0 % SERVER_TICK_RATE as u64 != 0
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

fn snapshot_from_transform(
    transform: &Transform,
) -> TransformSnapshot {
    TransformSnapshot {
        translation: transform.translation.to_array(),
        rotation: transform.rotation.to_array(),
        scale: transform.scale.to_array(),
    }
}

/// Inicializa o servidor dentro do mesmo processo do cliente,
/// mas em uma thread separada.
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
                        "[servidor] erro durante a execução: {error}"
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