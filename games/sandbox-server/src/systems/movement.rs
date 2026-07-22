use engine_ecs::prelude::*;

use crate::{
    components::Player,
    config::ARENA_HALF_SIZE,
    resources::SimulationTime,
};

pub(crate) fn movement_system(
    time: Res<SimulationTime>,
    mut players: Query<(&mut Transform, &Velocity), With<Player>>,
) {
    for (mut transform, velocity) in &mut players {
        if velocity.linear == Vec3::ZERO {
            continue;
        }

        transform.translation += velocity.linear * time.delta_seconds;
        transform.translation.x = transform
            .translation
            .x
            .clamp(-ARENA_HALF_SIZE, ARENA_HALF_SIZE);
        transform.translation.z = transform
            .translation
            .z
            .clamp(-ARENA_HALF_SIZE, ARENA_HALF_SIZE);
        transform.translation.y = 0.0;
    }
}
