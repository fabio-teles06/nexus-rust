use bevy_ecs::prelude::*;
use engine_ecs::{RenderTransform, SimulationTransform, Transform};
use engine_network::{ClientTransport, TransportError};
use glam::{Quat, Vec3};
use sandbox_shared::{
    EntitySnapshot, NetworkId, ServerMessage, SpawnSnapshot, TransformSnapshot,
};
use std::time::Instant;

use crate::{
    assets::ClientAssets,
    client::SandboxClient,
    components::{
        ClientEntityKind, ClientNetworkId, LocalPlayer, MaterialHandle, MeshHandle,
        NetworkTransform, ReplicatedEntity,
    },
    input::PredictionState,
};

const PLAYER_SPEED: f32 = 5.0;
const SNAPSHOT_RATE: f32 = 15.0;

impl SandboxClient {
    pub(crate) fn poll_server(&mut self) -> Result<(), TransportError> {
        loop {
            match self.runtime.transport_mut().try_receive() {
                Ok(Some(message)) => self.handle_server_message(message),
                Ok(None) => return Ok(()),
                Err(TransportError::Disconnected) => {
                    self.connected = false;
                    return Ok(());
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn handle_server_message(&mut self, message: ServerMessage) {
        match message {
            ServerMessage::Welcome { player_entity, .. } => {
                self.local_player = Some(player_entity);
                self.mark_local_player();
            }
            ServerMessage::SpawnBatch(spawns) => {
                for spawn in spawns {
                    self.spawn_replicated_entity(spawn);
                }
            }
            ServerMessage::SnapshotBatch {
                server_tick,
                entities,
            } => {
                for snapshot in entities {
                    self.apply_snapshot(server_tick, snapshot);
                }
            }
            ServerMessage::DespawnBatch(network_ids) => {
                for network_id in network_ids {
                    self.despawn_replicated_entity(network_id);
                }
            }
            ServerMessage::Stopped => {
                self.connected = false;
            }
        }
    }

    fn spawn_replicated_entity(&mut self, spawn: SpawnSnapshot) {
        if self.entities.contains_key(&spawn.network_id) {
            return;
        }

        let transform = transform_from_snapshot(spawn.transform);
        let (cube, material) = {
            let assets = self.world.resource::<ClientAssets>();
            let material = if self.local_player == Some(spawn.network_id) {
                assets.local_material
            } else {
                assets.remote_material
            };
            (assets.cube, material)
        };

        let entity = self
            .world
            .spawn((
                ReplicatedEntity,
                ClientNetworkId(spawn.network_id),
                ClientEntityKind(spawn.kind),
                MeshHandle(cube),
                MaterialHandle(material),
                NetworkTransform {
                    previous: transform,
                    current: transform,
                    previous_tick: Default::default(),
                    current_tick: Default::default(),
                    received_at: Instant::now(),
                },
                SimulationTransform(transform),
                RenderTransform(transform),
            ))
            .id();

        self.entities.insert(spawn.network_id, entity);
        self.mark_local_player();
    }

    fn mark_local_player(&mut self) {
        let Some(network_id) = self.local_player else {
            return;
        };
        let Some(&entity) = self.entities.get(&network_id) else {
            return;
        };

        let local_material = self.world.resource::<ClientAssets>().local_material;
        self.world
            .entity_mut(entity)
            .insert((LocalPlayer, MaterialHandle(local_material)));
    }

    fn apply_snapshot(&mut self, tick: engine_core::Tick, snapshot: EntitySnapshot) {
        let Some(&entity) = self.entities.get(&snapshot.network_id) else {
            return;
        };

        let authoritative = transform_from_snapshot(snapshot.transform);

        if self.local_player == Some(snapshot.network_id) {
            self.reconcile_local_player(
                entity,
                authoritative,
                snapshot.last_processed_input,
            );
        } else if let Some(mut network) = self.world.get_mut::<NetworkTransform>(entity) {
            network.previous = network.current;
            network.previous_tick = network.current_tick;
            network.current = authoritative;
            network.current_tick = tick;
            network.received_at = Instant::now();
        }
    }

    fn reconcile_local_player(
        &mut self,
        entity: Entity,
        authoritative: Transform,
        acknowledged_sequence: Option<u32>,
    ) {
        if let Some(acknowledged_sequence) = acknowledged_sequence {
            self.world
                .resource_mut::<PredictionState>()
                .pending
                .retain(|pending| {
                    engine_core::sequence_is_newer(
                        pending.input.sequence,
                        acknowledged_sequence,
                    )
                });
        }

        let pending_inputs: Vec<_> = self
            .world
            .resource::<PredictionState>()
            .pending
            .iter()
            .copied()
            .collect();

        let mut corrected = authoritative;
        for pending in pending_inputs {
            corrected.translation += Vec3::from_array(pending.input.direction)
                .normalize_or_zero()
                * PLAYER_SPEED
                * pending.delta_seconds;
        }

        self.world.entity_mut(entity).insert((
            SimulationTransform(corrected),
            RenderTransform(corrected),
        ));
    }

    pub(crate) fn interpolate_remote_entities(&mut self) {
        let mut query = self.world.query_filtered::<(
            &NetworkTransform,
            &mut SimulationTransform,
            &mut RenderTransform,
        ), Without<LocalPlayer>>();

        for (network, mut simulation, mut render) in query.iter_mut(&mut self.world) {
            let alpha = (network.received_at.elapsed().as_secs_f32() * SNAPSHOT_RATE)
                .clamp(0.0, 1.0);

            simulation.0 = network.current;
            render.0 = network.previous.lerp(&network.current, alpha);
        }
    }

    fn despawn_replicated_entity(&mut self, network_id: NetworkId) {
        if let Some(entity) = self.entities.remove(&network_id) {
            let _ = self.world.despawn(entity);
        }

        if self.local_player == Some(network_id) {
            self.local_player = None;
        }
    }
}

fn transform_from_snapshot(snapshot: TransformSnapshot) -> Transform {
    Transform {
        translation: Vec3::from_array(snapshot.translation),
        rotation: Quat::from_array(snapshot.rotation),
        scale: Vec3::from_array(snapshot.scale),
    }
}
