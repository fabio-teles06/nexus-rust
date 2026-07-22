use engine_ecs::prelude::*;
use sandbox_shared::{
    PlayerScoreSnapshot, ServerMessage,
};

use crate::{
    components::{
        Collectible, NetworkEntity, Player, PlayerName, PlayerOwner,
        PlayerScore,
    },
    config::{
        ORB_COLLECT_DISTANCE, ORB_SPAWN_POINTS, TARGET_SCORE,
    },
    resources::{
        OrbRespawnState, OutgoingMessages, PlayerRegistry,
    },
};

pub(crate) fn collect_orb_system(
    mut players: Query<
        (
            &PlayerOwner,
            &PlayerName,
            &Transform,
            &mut PlayerScore,
        ),
        (With<Player>, Without<Collectible>),
    >,
    mut orbs: Query<
        (&NetworkEntity, &mut Transform),
        (With<Collectible>, Without<Player>),
    >,
    mut respawn: ResMut<OrbRespawnState>,
    registry: Res<PlayerRegistry>,
    mut outgoing: ResMut<OutgoingMessages>,
) {
    let Ok((_orb_network_id, mut orb_transform)) = orbs.single_mut() else {
        return;
    };

    let mut collected_by: Option<(String, bool)> = None;

    for (_owner, player_name, player_transform, mut score) in &mut players {
        let distance_squared = player_transform
            .translation
            .distance_squared(orb_transform.translation);

        if distance_squared
            > ORB_COLLECT_DISTANCE * ORB_COLLECT_DISTANCE
        {
            continue;
        }

        score.0 += 1;

        let won = score.0 >= TARGET_SCORE;
        collected_by = Some((player_name.0.clone(), won));
        break;
    }

    let Some((player_name, won)) = collected_by else {
        return;
    };

    respawn.next_index =
        (respawn.next_index + 1) % ORB_SPAWN_POINTS.len();

    orb_transform.translation = ORB_SPAWN_POINTS[respawn.next_index];

    if won {
        for (_, _, _, mut score) in &mut players {
            score.0 = 0;
        }

        broadcast(
            &mut outgoing,
            &registry,
            ServerMessage::RoundWon {
                player_name: player_name.clone(),
            },
        );

        println!("[jogo] {player_name} venceu a rodada");
    } else {
        println!("[jogo] {player_name} coletou o orbe");
    }

    let scoreboard = players
        .iter_mut()
        .map(|(_, name, _, score)| PlayerScoreSnapshot {
            player_name: name.0.clone(),
            score: score.0,
        })
        .collect::<Vec<_>>();

    broadcast(
        &mut outgoing,
        &registry,
        ServerMessage::Scoreboard {
            players: scoreboard,
            target_score: TARGET_SCORE,
        },
    );
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
