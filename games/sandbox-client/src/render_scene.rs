use engine_ecs::prelude::*;
use engine_render::RenderInstance;
use sandbox_shared::EntityKind;

use crate::{
    client::SandboxClient,
    components::{
        ClientEntityKind, ClientNetworkId, LocalPlayer,
    },
};

pub(crate) struct RenderScene {
    pub instances: Vec<RenderInstance>,
    pub camera_target: Option<Vec3>,
}

impl SandboxClient {
    pub(crate) fn build_render_scene(&mut self) -> RenderScene {
        let mut instances = static_world_instances();
        let mut camera_target = None;

        let mut query = self.world.query::<(
            &ClientNetworkId,
            &Transform,
            Option<&LocalPlayer>,
            &ClientEntityKind,
        )>();

        for (
            network_id,
            transform,
            local_player,
            entity_kind,
        ) in query.iter(&self.world)
        {
            let color = match entity_kind.0 {
                EntityKind::Orb => [1.0, 0.78, 0.08, 1.0],

                EntityKind::Player if local_player.is_some() => {
                    [0.12, 0.48, 1.0, 1.0]
                }

                EntityKind::Player => {
                    player_color(network_id.0.0)
                }
            };

            instances.push(RenderInstance::new(
                transform.compute_matrix(),
                color,
            ));

            if local_player.is_some() {
                camera_target =
                    Some(transform.translation + Vec3::Y * 0.35);
            }
        }

        RenderScene {
            instances,
            camera_target,
        }
    }
}

fn static_world_instances() -> Vec<RenderInstance> {
    let mut instances = Vec::with_capacity(16);

    instances.push(RenderInstance::from_translation_scale(
        Vec3::new(0.0, -0.6, 0.0),
        Vec3::new(26.0, 0.2, 26.0),
        [0.10, 0.17, 0.14, 1.0],
    ));

    for x in [-12.5, 12.5] {
        instances.push(RenderInstance::from_translation_scale(
            Vec3::new(x, 0.5, 0.0),
            Vec3::new(0.3, 2.0, 26.0),
            [0.32, 0.36, 0.42, 1.0],
        ));
    }

    for z in [-12.5, 12.5] {
        instances.push(RenderInstance::from_translation_scale(
            Vec3::new(0.0, 0.5, z),
            Vec3::new(26.0, 2.0, 0.3),
            [0.32, 0.36, 0.42, 1.0],
        ));
    }

    add_pillar(&mut instances, Vec3::new(6.0, 1.0, 6.0));
    add_pillar(&mut instances, Vec3::new(-6.0, 1.0, 6.0));
    add_pillar(&mut instances, Vec3::new(6.0, 1.0, -6.0));
    add_pillar(&mut instances, Vec3::new(-6.0, 1.0, -6.0));

    instances
}

fn add_pillar(
    instances: &mut Vec<RenderInstance>,
    position: Vec3,
) {
    instances.push(RenderInstance::from_translation_scale(
        position,
        Vec3::new(0.7, 2.0, 0.7),
        [0.58, 0.35, 0.16, 1.0],
    ));
}

fn player_color(network_id: u64) -> [f32; 4] {
    const COLORS: [[f32; 4]; 6] = [
        [1.0, 0.25, 0.20, 1.0],
        [0.25, 0.85, 0.35, 1.0],
        [0.80, 0.30, 1.0, 1.0],
        [1.0, 0.50, 0.10, 1.0],
        [0.10, 0.80, 0.85, 1.0],
        [0.95, 0.20, 0.60, 1.0],
    ];

    COLORS[network_id as usize % COLORS.len()]
}
