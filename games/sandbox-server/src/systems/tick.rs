use engine_ecs::prelude::*;
use sandbox_shared::ServerMessage;

use crate::{
    components::{Player, PlayerOwner},
    config::SERVER_TICK_RATE,
    resources::{OutgoingMessages, SimulationTime},
};

pub(crate) fn send_periodic_tick(
    time: Res<SimulationTime>,
    players: Query<&PlayerOwner, With<Player>>,
    mut outgoing: ResMut<OutgoingMessages>,
) {
    if time.tick.0 == 0 || time.tick.0 % SERVER_TICK_RATE as u64 != 0 {
        return;
    }

    for owner in &players {
        outgoing
            .messages
            .push((owner.0, ServerMessage::ServerTick { tick: time.tick }));
    }
}
