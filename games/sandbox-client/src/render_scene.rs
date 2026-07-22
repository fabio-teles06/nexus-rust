use engine_ecs::prelude::*;
use engine_render::RenderInstance;

use crate::{
    client::SandboxClient,
    components::{ClientEntityKind, LocalPlayer},
};

pub(crate) struct RenderScene {
    pub instances: Vec<RenderInstance>,
    pub camera_target: Option<Vec3>,
}

impl SandboxClient {
    pub(crate) fn build_render_scene(&mut self) -> RenderScene {
        let mut instances = static_world_instances();

        let mut camera_target = None;

        let mut query = self
            .world
            .query::<(&Transform, Option<&LocalPlayer>, Option<&ClientEntityKind>)>();

        for (transform, local_player, _entity_kind) in query.iter(&self.world) {
            let color = if local_player.is_some() {
                /*
                 * Jogador local.
                 */
                [0.12, 0.48, 1.0, 1.0]
            } else {
                /*
                 * Outras entidades replicadas.
                 */
                [1.0, 0.38, 0.12, 1.0]
            };

            instances.push(RenderInstance::new(transform.compute_matrix(), color));

            if local_player.is_some() {
                camera_target = Some(transform.translation + Vec3::Y * 0.35);
            }
        }

        RenderScene {
            instances,
            camera_target,
        }
    }
}

fn static_world_instances() -> Vec<RenderInstance> {
    let mut instances = Vec::with_capacity(8);

    /*
     * Piso.
     */
    instances.push(RenderInstance::from_translation_scale(
        Vec3::new(0.0, -0.6, 0.0),
        Vec3::new(30.0, 0.2, 30.0),
        [0.19, 0.27, 0.20, 1.0],
    ));

    /*
     * Pilar central de referência.
     */
    instances.push(RenderInstance::from_translation_scale(
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.25, 2.0, 0.25),
        [0.9, 0.75, 0.15, 1.0],
    ));

    add_pillar(&mut instances, Vec3::new(5.0, 1.0, 5.0));

    add_pillar(&mut instances, Vec3::new(-5.0, 1.0, 5.0));

    add_pillar(&mut instances, Vec3::new(5.0, 1.0, -5.0));

    add_pillar(&mut instances, Vec3::new(-5.0, 1.0, -5.0));

    instances
}

fn add_pillar(instances: &mut Vec<RenderInstance>, position: Vec3) {
    instances.push(RenderInstance::from_translation_scale(
        position,
        Vec3::new(0.7, 2.0, 0.7),
        [0.58, 0.35, 0.16, 1.0],
    ));
}
