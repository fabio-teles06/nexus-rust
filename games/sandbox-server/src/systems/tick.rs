use engine_ecs::prelude::*;
use sandbox_shared::ServerMessage;

use crate::{
    config::SERVER_TICK_RATE,
    resources::{OutgoingMessages, PlayerRegistry, SimulationTime},
};

pub(crate) fn send_periodic_tick(
    time: Res<SimulationTime>,
    players: Res<PlayerRegistry>,
    mut outgoing: ResMut<OutgoingMessages>,
) {
    if time.tick.0 == 0 || time.tick.0 % SERVER_TICK_RATE as u64 != 0 {
        return;
    }

    for client_id in players.players.keys().copied() {
        outgoing.messages.push((
            client_id,
            ServerMessage::ServerTick { tick: time.tick },
        ));
    }
}
