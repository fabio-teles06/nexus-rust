use bevy_ecs::prelude::*;
use engine_core::{ClientId, sequence_is_newer};
use engine_ecs::Transform;
use engine_physics::PhysicsWorld;
use engine_server::Outgoing;
use glam::Vec3;
use sandbox_shared::{ClientMessage, EntityKind, ServerMessage, SpawnSnapshot};

use crate::{
    components::{NetworkEntity, PlayerBundle, PlayerInputState, PlayerOwner},
    resources::{NetworkIdGenerator, Outbox, PendingMessages, PlayerRegistry, ServerState},
    snapshot::snapshot,
};

pub(crate) fn process_messages(world: &mut World) {
    let messages = std::mem::take(&mut world.resource_mut::<PendingMessages>().0);

    for (client_id, message) in messages {
        match message {
            ClientMessage::Join { player_name } => join(world, client_id, player_name),
            ClientMessage::Input(input) => {
                let mut query = world.query::<(&PlayerOwner, &mut PlayerInputState)>();

                for (owner, mut state) in query.iter_mut(world) {
                    if owner.0 == client_id
                        && sequence_is_newer(input.sequence, state.last_processed)
                    {
                        state.pending.push_back(input);

                        while state.pending.len() > 64 {
                            state.pending.pop_front();
                        }

                        break;
                    }
                }
            }
            ClientMessage::ShutdownServer => {
                world.resource_mut::<ServerState>().running = false;
                world
                    .resource_mut::<Outbox>()
                    .0
                    .push(Outgoing::Broadcast(ServerMessage::Stopped));
            }
        }
    }
}

fn join(world: &mut World, client_id: ClientId, player_name: String) {
    if world
        .resource::<PlayerRegistry>()
        .0
        .contains_key(&client_id)
    {
        return;
    }

    let network_id = world.resource_mut::<NetworkIdGenerator>().next();
    let spawn_position = Vec3::new(client_id.0 as f32 * 1.5 - 1.5, 1.0, 0.0);
    let transform = Transform::from_translation(spawn_position);

    let physics_body = world
        .resource_mut::<PhysicsWorld>()
        .create_dynamic_box(
            spawn_position,
            Vec3::splat(0.45),
            network_id.0 as u128,
        );

    world.spawn(PlayerBundle {
        marker: crate::components::Player,
        owner: PlayerOwner(client_id),
        network: NetworkEntity(network_id),
        transform,
        physics_body,
        input: PlayerInputState::default(),
    });

    world
        .resource_mut::<PlayerRegistry>()
        .0
        .insert(client_id, network_id);

    let mut existing = Vec::new();
    let mut query = world.query::<(&NetworkEntity, &Transform)>();

    for (network, transform) in query.iter(world) {
        existing.push(SpawnSnapshot {
            network_id: network.0,
            kind: EntityKind::Player,
            transform: snapshot(transform),
        });
    }

    let mut outbox = world.resource_mut::<Outbox>();
    outbox.0.push(Outgoing::To(
        client_id,
        ServerMessage::Welcome {
            client_id,
            player_entity: network_id,
            player_name,
        },
    ));
    outbox
        .0
        .push(Outgoing::To(client_id, ServerMessage::SpawnBatch(existing)));
    outbox
        .0
        .push(Outgoing::Broadcast(ServerMessage::SpawnBatch(vec![
            SpawnSnapshot {
                network_id,
                kind: EntityKind::Player,
                transform: snapshot(&transform),
            },
        ])));
}
