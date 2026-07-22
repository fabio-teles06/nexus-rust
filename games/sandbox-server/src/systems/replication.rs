use engine_ecs::prelude::*;
use sandbox_shared::ServerMessage;

use crate::{
    components::NetworkEntity,
    resources::{OutgoingMessages, PlayerRegistry, SimulationTime},
    snapshot::snapshot_from_transform,
};

pub(crate) fn replicate_changed_transforms(
    time: Res<SimulationTime>,
    entities: Query<(&NetworkEntity, &Transform), Changed<Transform>>,
    players: Res<PlayerRegistry>,
    mut outgoing: ResMut<OutgoingMessages>,
) {
    for (network_entity, transform) in &entities {
        let message = ServerMessage::UpdateTransform {
            network_id: network_entity.0,
            server_tick: time.tick,
            transform: snapshot_from_transform(transform),
        };

        for client_id in players.players.keys().copied() {
            outgoing.messages.push((client_id, message.clone()));
        }
    }
}
