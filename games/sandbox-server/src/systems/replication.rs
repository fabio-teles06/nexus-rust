use engine_ecs::prelude::*;
use sandbox_shared::ServerMessage;

use crate::{
    components::{NetworkEntity, Player, PlayerOwner},
    resources::{OutgoingMessages, SimulationTime},
    snapshot::snapshot_from_transform,
};

pub(crate) fn replicate_changed_transforms(
    time: Res<SimulationTime>,
    players: Query<(&PlayerOwner, &NetworkEntity, &Transform), (With<Player>, Changed<Transform>)>,
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
