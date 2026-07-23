use bevy_ecs::prelude::*;
use engine_ecs::Transform;
use engine_physics::{PhysicsBody, PhysicsWorld};
use glam::Vec3;

use crate::{
    components::{Player, PlayerInputState},
    config::PLAYER_SPEED,
    resources::SimulationTime,
};

/// Consome inputs, atualiza velocidades do Rapier, executa um único passo
/// físico e sincroniza os `Transform` do ECS com o estado autoritativo.
pub(crate) fn physics_movement(world: &mut World) {
    let delta_seconds = world.resource::<SimulationTime>().delta_seconds;

    let mut velocity_commands = Vec::new();

    {
        let mut players =
            world.query_filtered::<(&PhysicsBody, &mut PlayerInputState), With<Player>>();

        for (physics_body, mut input) in players.iter_mut(world) {
            if let Some(latest) = input.pending.pop_back() {
                input.pending.clear();

                let direction = Vec3::from_array(latest.direction);
                input.current_direction = if direction.is_finite() {
                    direction.normalize_or_zero()
                } else {
                    Vec3::ZERO
                };
                input.last_processed = latest.sequence;
            }

            velocity_commands.push((*physics_body, input.current_direction * PLAYER_SPEED));
        }
    }

    world.resource_scope(|world, mut physics: Mut<PhysicsWorld>| {
        for (body, velocity) in velocity_commands {
            physics.set_horizontal_velocity(body, velocity);
        }

        physics.step(delta_seconds);

        let mut players = world.query_filtered::<(&PhysicsBody, &mut Transform), With<Player>>();

        for (physics_body, mut transform) in players.iter_mut(world) {
            let Some(state) = physics.body_state(*physics_body) else {
                continue;
            };

            transform.translation = state.translation;
            transform.rotation = state.rotation;
        }
    });
}
