use engine_ecs::prelude::*;

use crate::{components::Player, resources::SimulationTime};

pub(crate) fn movement_system(
    time: Res<SimulationTime>,
    mut players: Query<(&mut Transform, &Velocity), With<Player>>,
) {
    for (mut transform, velocity) in &mut players {
        if velocity.linear == Vec3::ZERO {
            continue;
        }

        transform.translation += velocity.linear * time.delta_seconds;
    }
}
