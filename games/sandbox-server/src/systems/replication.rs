use bevy_ecs::prelude::*;
use engine_ecs::Transform;
use engine_physics::{PhysicsBody, PhysicsWorld};
use engine_server::Outgoing;
use sandbox_shared::{EntitySnapshot, ServerMessage};

use crate::{
    components::{NetworkEntity, Player, PlayerInputState},
    config::{SERVER_TICK_RATE, SNAPSHOT_RATE},
    resources::{Outbox, SimulationTime},
    snapshot::snapshot,
};

pub(crate) fn replicate_snapshot_batch(
    time: Res<SimulationTime>,
    physics: Res<PhysicsWorld>,
    players: Query<
        (&NetworkEntity, &Transform, &PhysicsBody, &PlayerInputState),
        With<Player>,
    >,
    mut outbox: ResMut<Outbox>,
) {
    let divisor = (SERVER_TICK_RATE / SNAPSHOT_RATE).max(1) as u64;

    if time.tick.0 % divisor != 0 {
        return;
    }

    let entities = players
        .iter()
        .map(|(network, transform, physics_body, input)| EntitySnapshot {
            network_id: network.0,
            transform: snapshot(transform),
            velocity: physics
                .linear_velocity(*physics_body)
                .unwrap_or_default()
                .to_array(),
            last_processed_input: Some(input.last_processed),
        })
        .collect();

    outbox
        .0
        .push(Outgoing::Broadcast(ServerMessage::SnapshotBatch {
            server_tick: time.tick,
            entities,
        }));
}
