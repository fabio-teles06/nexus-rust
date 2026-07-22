use engine_core::ClientId;
use engine_ecs::prelude::*;
use sandbox_shared::{ClientMessage, EntityKind, PlayerInput, ServerMessage};

use crate::{
    components::{LastInputSequence, NetworkEntity, Player, PlayerOwner},
    config::PLAYER_SPEED,
    resources::{
        NetworkIdGenerator, OutgoingMessages, PendingClientMessages, PlayerRegistry, ServerState,
    },
    snapshot::snapshot_from_transform,
};

pub(crate) fn process_client_messages(
    mut commands: Commands,
    mut pending: ResMut<PendingClientMessages>,
    mut outgoing: ResMut<OutgoingMessages>,
    mut server_state: ResMut<ServerState>,
    mut network_ids: ResMut<NetworkIdGenerator>,
    mut player_registry: ResMut<PlayerRegistry>,
    mut players: Query<(&PlayerOwner, &mut Velocity, &mut LastInputSequence), With<Player>>,
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

            ClientMessage::Input(input) => {
                handle_player_input(&mut players, client_id, input);
            }

            ClientMessage::Shutdown => {
                handle_shutdown(&mut server_state, &mut outgoing, client_id);
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
        println!("[servidor] cliente {} já possui jogador", client_id.0);

        return;
    }

    let network_id = network_ids.generate();
    let transform = Transform::from_xyz(0.0, 0.0, 0.0);

    commands.spawn((
        Player,
        PlayerOwner(client_id),
        NetworkEntity(network_id),
        LastInputSequence::default(),
        transform,
        Velocity::default(),
    ));

    player_registry.players.insert(client_id, network_id);

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

fn handle_player_input(
    players: &mut Query<(&PlayerOwner, &mut Velocity, &mut LastInputSequence), With<Player>>,
    client_id: ClientId,
    input: PlayerInput,
) {
    for (owner, mut velocity, mut last_sequence) in players.iter_mut() {
        if owner.0 != client_id {
            continue;
        }

        if !is_newer_sequence(input.sequence, last_sequence.0) {
            return;
        }

        last_sequence.0 = input.sequence;

        let direction = validate_direction(Vec3::from_array(input.direction));

        velocity.linear = direction * PLAYER_SPEED;

        return;
    }
}

fn handle_shutdown(
    server_state: &mut ServerState,
    outgoing: &mut OutgoingMessages,
    client_id: ClientId,
) {
    server_state.running = false;

    outgoing.messages.push((client_id, ServerMessage::Stopped));
}

fn validate_direction(direction: Vec3) -> Vec3 {
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

fn is_newer_sequence(sequence: u32, previous: u32) -> bool {
    const HALF_RANGE: u32 = 1 << 31;

    sequence != previous && sequence.wrapping_sub(previous) < HALF_RANGE
}
