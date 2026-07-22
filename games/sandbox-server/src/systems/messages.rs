use engine_core::ClientId;
use engine_ecs::prelude::*;
use sandbox_shared::{
    ClientMessage, EntityKind, PlayerInput, PlayerScoreSnapshot,
    ServerMessage,
};

use crate::{
    components::{
        Collectible, LastInputSequence, NetworkEntity, Player, PlayerName,
        PlayerOwner, PlayerScore,
    },
    config::{PLAYER_SPEED, TARGET_SCORE},
    resources::{
        NetworkIdGenerator, OutgoingMessages, PendingClientMessages,
        PlayerRegistry, ServerSettings, ServerState,
    },
    snapshot::snapshot_from_transform,
};

pub(crate) fn process_client_messages(
    mut commands: Commands,
    mut pending: ResMut<PendingClientMessages>,
    mut outgoing: ResMut<OutgoingMessages>,
    mut server_state: ResMut<ServerState>,
    settings: Res<ServerSettings>,
    mut network_ids: ResMut<NetworkIdGenerator>,
    mut player_registry: ResMut<PlayerRegistry>,
    mut players: Query<
        (
            &PlayerOwner,
            &mut Velocity,
            &mut LastInputSequence,
        ),
        With<Player>,
    >,
    existing_entities: Query<
        (
            &NetworkEntity,
            &Transform,
            Option<&Player>,
            Option<&Collectible>,
        ),
    >,
    player_entities: Query<
        (Entity, &PlayerOwner, &NetworkEntity),
        With<Player>,
    >,
    scores: Query<
        (&PlayerOwner, &PlayerName, &PlayerScore),
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
                    &existing_entities,
                    &scores,
                    client_id,
                    sanitize_player_name(player_name),
                );
            }

            ClientMessage::Input(input) => {
                handle_player_input(&mut players, client_id, input);
            }

            ClientMessage::Leave => {
                handle_leave(
                    &mut commands,
                    &mut outgoing,
                    &mut player_registry,
                    &player_entities,
                    &scores,
                    client_id,
                );
            }

            ClientMessage::ShutdownServer => {
                if settings.allow_shutdown {
                    server_state.running = false;
                    outgoing
                        .messages
                        .push((client_id, ServerMessage::Stopped));
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_join(
    commands: &mut Commands,
    outgoing: &mut OutgoingMessages,
    network_ids: &mut NetworkIdGenerator,
    player_registry: &mut PlayerRegistry,
    existing_entities: &Query<
        (
            &NetworkEntity,
            &Transform,
            Option<&Player>,
            Option<&Collectible>,
        ),
    >,
    scores: &Query<
        (&PlayerOwner, &PlayerName, &PlayerScore),
        With<Player>,
    >,
    client_id: ClientId,
    player_name: String,
) {
    if player_registry.players.contains_key(&client_id) {
        return;
    }

    let network_id = network_ids.generate();
    let spawn_index = player_registry.players.len();
    let angle = spawn_index as f32 * 1.7;
    let transform = Transform::from_xyz(
        angle.cos() * 6.0,
        0.0,
        angle.sin() * 6.0,
    );

    // Primeiro, o novo cliente recebe tudo que já existe.
    for (entity_network_id, entity_transform, player, collectible) in
        existing_entities.iter()
    {
        let kind = if player.is_some() {
            EntityKind::Player
        } else if collectible.is_some() {
            EntityKind::Orb
        } else {
            continue;
        };

        outgoing.messages.push((
            client_id,
            ServerMessage::SpawnEntity {
                network_id: entity_network_id.0,
                transform: snapshot_from_transform(entity_transform),
                kind,
            },
        ));
    }

    commands.spawn((
        Player,
        PlayerOwner(client_id),
        PlayerName(player_name.clone()),
        PlayerScore::default(),
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
            player_name: player_name.clone(),
        },
    ));

    let spawn_message = ServerMessage::SpawnEntity {
        network_id,
        kind: EntityKind::Player,
        transform: snapshot_from_transform(&transform),
    };

    broadcast(
        outgoing,
        player_registry,
        spawn_message,
    );

    let mut scoreboard = scores
        .iter()
        .map(|(_, name, score)| PlayerScoreSnapshot {
            player_name: name.0.clone(),
            score: score.0,
        })
        .collect::<Vec<_>>();

    scoreboard.push(PlayerScoreSnapshot {
        player_name: player_name.clone(),
        score: 0,
    });

    broadcast(
        outgoing,
        player_registry,
        ServerMessage::Scoreboard {
            players: scoreboard,
            target_score: TARGET_SCORE,
        },
    );

    println!(
        "[jogo] {} entrou como cliente {} / entidade {}",
        player_name, client_id.0, network_id.0
    );
}

fn handle_player_input(
    players: &mut Query<
        (
            &PlayerOwner,
            &mut Velocity,
            &mut LastInputSequence,
        ),
        With<Player>,
    >,
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

fn handle_leave(
    commands: &mut Commands,
    outgoing: &mut OutgoingMessages,
    player_registry: &mut PlayerRegistry,
    player_entities: &Query<
        (Entity, &PlayerOwner, &NetworkEntity),
        With<Player>,
    >,
    scores: &Query<
        (&PlayerOwner, &PlayerName, &PlayerScore),
        With<Player>,
    >,
    client_id: ClientId,
) {
    let Some(network_id) = player_registry.players.remove(&client_id) else {
        return;
    };

    for (entity, owner, _) in player_entities.iter() {
        if owner.0 == client_id {
            commands.entity(entity).despawn();
            break;
        }
    }

    broadcast(
        outgoing,
        player_registry,
        ServerMessage::DespawnEntity { network_id },
    );

    let scoreboard = scores
        .iter()
        .filter(|(owner, _, _)| owner.0 != client_id)
        .map(|(_, name, score)| PlayerScoreSnapshot {
            player_name: name.0.clone(),
            score: score.0,
        })
        .collect::<Vec<_>>();

    broadcast(
        outgoing,
        player_registry,
        ServerMessage::Scoreboard {
            players: scoreboard,
            target_score: TARGET_SCORE,
        },
    );

    println!("[jogo] cliente {} saiu da arena", client_id.0);
}

fn broadcast(
    outgoing: &mut OutgoingMessages,
    players: &PlayerRegistry,
    message: ServerMessage,
) {
    for client_id in players.players.keys().copied() {
        outgoing.messages.push((client_id, message.clone()));
    }
}

fn sanitize_player_name(player_name: String) -> String {
    let trimmed = player_name.trim();

    if trimmed.is_empty() {
        return "Jogador".to_string();
    }

    trimmed.chars().take(24).collect()
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
