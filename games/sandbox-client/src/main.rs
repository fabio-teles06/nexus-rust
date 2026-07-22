use std::{
    collections::HashMap,
    error::Error,
    thread,
    time::{Duration, Instant},
};

use engine_client::ClientRuntime;
use engine_ecs::prelude::*;
use engine_network::{
    ClientTransport,
    TransportError,
};
use sandbox_server::{
    SandboxClientTransport,
    start_integrated_server,
};
use sandbox_shared::{
    ClientMessage,
    EntityKind,
    NetworkId,
    ServerMessage,
    TransformSnapshot,
};

/// Marca uma entidade criada a partir de dados recebidos
/// do servidor.
#[derive(Component, Debug)]
struct ReplicatedEntity;

/// ID utilizado para relacionar a entidade local com a
/// entidade existente no servidor.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ClientNetworkId(NetworkId);

/// Marca a entidade controlada pelo cliente atual.
#[derive(Component, Debug)]
struct LocalPlayer;

/// Tipo lógico da entidade recebida do servidor.
#[derive(Component, Debug, Clone, Copy)]
struct ClientEntityKind(EntityKind);

struct SandboxClient {
    runtime: ClientRuntime<SandboxClientTransport>,
    world: World,

    /// Evita percorrer todo o ECS sempre que uma mensagem
    /// de replicação for recebida.
    entities: HashMap<NetworkId, Entity>,

    local_player: Option<NetworkId>,
    connected: bool,
}

impl SandboxClient {
    fn new(
        transport: SandboxClientTransport,
    ) -> Self {
        Self {
            runtime: ClientRuntime::new(transport),
            world: World::new(),
            entities: HashMap::new(),
            local_player: None,
            connected: true,
        }
    }

    fn send(
        &mut self,
        message: ClientMessage,
    ) -> Result<(), TransportError> {
        self.runtime.send(message)
    }

    /// Processa mensagens durante determinado período.
    ///
    /// Futuramente isso será substituído pelo loop da janela.
    fn pump_for(
        &mut self,
        duration: Duration,
    ) -> Result<(), TransportError> {
        let deadline = Instant::now() + duration;

        while Instant::now() < deadline {
            self.poll_server()?;

            if !self.connected {
                break;
            }

            thread::sleep(Duration::from_millis(10));
        }

        Ok(())
    }

    /// Lê todas as mensagens disponíveis sem bloquear.
    ///
    /// Não usamos ClientRuntime::poll aqui porque queremos
    /// tratar o fechamento do canal depois da última mensagem
    /// recebida pelo servidor.
    fn poll_server(
        &mut self,
    ) -> Result<(), TransportError> {
        loop {
            let received = {
                self.runtime
                    .transport_mut()
                    .try_receive()
            };

            match received {
                Ok(Some(message)) => {
                    self.handle_server_message(message);

                    if !self.connected {
                        return Ok(());
                    }
                }

                Ok(None) => {
                    return Ok(());
                }

                Err(TransportError::Disconnected) => {
                    self.connected = false;

                    println!(
                        "[cliente] conexão com o servidor encerrada"
                    );

                    return Ok(());
                }

                Err(error) => {
                    return Err(error);
                }
            }
        }
    }

    fn handle_server_message(
        &mut self,
        message: ServerMessage,
    ) {
        match message {
            ServerMessage::Welcome {
                client_id,
                player_entity,
                player_name,
            } => {
                self.local_player = Some(player_entity);

                println!(
                    "[cliente] conectado como {} | cliente={} | entidade={}",
                    player_name,
                    client_id.0,
                    player_entity.0
                );

                self.mark_local_player_if_spawned();
            }

            ServerMessage::SpawnEntity {
                network_id,
                kind,
                transform,
            } => {
                self.spawn_or_update_entity(
                    network_id,
                    kind,
                    transform,
                );
            }

            ServerMessage::UpdateTransform {
                network_id,
                server_tick,
                transform,
            } => {
                self.update_entity_transform(
                    network_id,
                    transform,
                );

                if self.local_player == Some(network_id) {
                    let position =
                        Vec3::from_array(transform.translation);

                    println!(
                        "[cliente] tick={} | posição confirmada={:?}",
                        server_tick.0,
                        position
                    );
                }
            }

            ServerMessage::DespawnEntity {
                network_id,
            } => {
                self.despawn_entity(network_id);
            }

            ServerMessage::ServerTick { tick } => {
                println!(
                    "[cliente] servidor no tick {}",
                    tick.0
                );
            }

            ServerMessage::Stopped => {
                println!("[cliente] servidor foi desligado");

                self.connected = false;
            }
        }
    }

    fn spawn_or_update_entity(
        &mut self,
        network_id: NetworkId,
        kind: EntityKind,
        snapshot: TransformSnapshot,
    ) {
        let transform =
            transform_from_snapshot(snapshot);

        if let Some(&entity) =
            self.entities.get(&network_id)
        {
            self.world
                .entity_mut(entity)
                .insert((
                    transform,
                    ClientEntityKind(kind),
                ));

            println!(
                "[cliente] entidade {} já existia e foi atualizada",
                network_id.0
            );

            return;
        }

        let entity = self
            .world
            .spawn((
                ReplicatedEntity,
                ClientNetworkId(network_id),
                ClientEntityKind(kind),
                transform,
            ))
            .id();

        self.entities.insert(network_id, entity);

        if self.local_player == Some(network_id) {
            self.world
                .entity_mut(entity)
                .insert(LocalPlayer);
        }

        println!(
            "[cliente] entidade replicada criada: network_id={} entity={:?}",
            network_id.0,
            entity
        );
    }

    fn update_entity_transform(
        &mut self,
        network_id: NetworkId,
        snapshot: TransformSnapshot,
    ) {
        let Some(&entity) =
            self.entities.get(&network_id)
        else {
            println!(
                "[cliente] atualização ignorada: entidade {} ainda não existe",
                network_id.0
            );

            return;
        };

        let transform =
            transform_from_snapshot(snapshot);

        self.world
            .entity_mut(entity)
            .insert(transform);
    }

    fn despawn_entity(
        &mut self,
        network_id: NetworkId,
    ) {
        let Some(entity) =
            self.entities.remove(&network_id)
        else {
            return;
        };

        let _ = self.world.despawn(entity);

        println!(
            "[cliente] entidade {} removida",
            network_id.0
        );

        if self.local_player == Some(network_id) {
            self.local_player = None;
        }
    }

    fn mark_local_player_if_spawned(&mut self) {
        let Some(network_id) = self.local_player else {
            return;
        };

        let Some(&entity) =
            self.entities.get(&network_id)
        else {
            return;
        };

        self.world
            .entity_mut(entity)
            .insert(LocalPlayer);
    }

    fn print_local_player(&mut self) {
        let mut query = self.world.query_filtered::<
            (
                &ClientNetworkId,
                &ClientEntityKind,
                &Transform,
            ),
            With<LocalPlayer>,
        >();

        for (network_id, kind, transform) in
            query.iter(&self.world)
        {
            println!(
                "[cliente] jogador local: id={} tipo={:?} posição={:?}",
                network_id.0.0,
                kind.0,
                transform.translation
            );
        }
    }
}

fn transform_from_snapshot(
    snapshot: TransformSnapshot,
) -> Transform {
    Transform {
        translation: Vec3::from_array(
            snapshot.translation,
        ),
        rotation: Quat::from_array(
            snapshot.rotation,
        ),
        scale: Vec3::from_array(
            snapshot.scale,
        ),
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    println!("[cliente] iniciando sandbox");

    let (transport, server_thread) =
        start_integrated_server();

    let mut client = SandboxClient::new(transport);

    client.send(ClientMessage::Join {
        player_name: "Fabio".to_string(),
    })?;

    client.pump_for(Duration::from_millis(200))?;
    client.print_local_player();

    /*
     * Anda no eixo X durante aproximadamente meio segundo.
     */
    println!("[cliente] começando movimento no eixo X");

    client.send(ClientMessage::Move {
        direction: [1.0, 0.0, 0.0],
    })?;

    client.pump_for(Duration::from_millis(500))?;

    /*
     * Para o jogador.
     */
    client.send(ClientMessage::Move {
        direction: [0.0, 0.0, 0.0],
    })?;

    client.pump_for(Duration::from_millis(100))?;
    client.print_local_player();

    /*
     * Anda no eixo Z.
     */
    println!("[cliente] começando movimento no eixo Z");

    client.send(ClientMessage::Move {
        direction: [0.0, 0.0, 1.0],
    })?;

    client.pump_for(Duration::from_millis(400))?;

    client.send(ClientMessage::Move {
        direction: [0.0, 0.0, 0.0],
    })?;

    client.pump_for(Duration::from_millis(100))?;
    client.print_local_player();

    /*
     * Tenta enviar um vetor extremamente grande.
     *
     * O servidor normaliza esse vetor, impedindo que o
     * cliente altere sua velocidade máxima.
     */
    println!(
        "[cliente] tentando enviar movimento inválido de 5000 unidades"
    );

    client.send(ClientMessage::Move {
        direction: [5000.0, 0.0, 0.0],
    })?;

    client.pump_for(Duration::from_millis(300))?;

    client.send(ClientMessage::Move {
        direction: [0.0, 0.0, 0.0],
    })?;

    client.pump_for(Duration::from_millis(100))?;
    client.print_local_player();

    println!("[cliente] solicitando desligamento");

    client.send(ClientMessage::Shutdown)?;
    client.pump_for(Duration::from_millis(200))?;

    match server_thread.join() {
        Ok(result) => {
            result?;
        }

        Err(_) => {
            return Err(
                "a thread do servidor integrado entrou em pânico"
                    .into(),
            );
        }
    }

    println!("[cliente] sandbox finalizado");

    Ok(())
}