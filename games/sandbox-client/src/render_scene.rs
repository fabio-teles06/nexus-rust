use engine_assets::{MaterialAsset, MeshAsset};
use engine_ecs::RenderTransform;
use engine_render::RenderInstance;
use glam::Vec3;

use crate::{
    assets::ClientAssets,
    client::SandboxClient,
    components::{ClientNetworkId, MaterialHandle, MeshHandle},
};

pub(crate) struct RenderScene {
    pub instances: Vec<RenderInstance>,
    pub camera_target: Option<Vec3>,
}

impl SandboxClient {
    pub(crate) fn build_render_scene(&mut self) -> RenderScene {
        let mut query = self.world.query::<(
            &RenderTransform,
            &MeshHandle,
            &MaterialHandle,
            Option<&ClientNetworkId>,
        )>();
        let assets = self.world.resource::<ClientAssets>();
        let mut instances = Vec::new();
        let mut camera_target = None;

        for (render, mesh, material, network_id) in query.iter(&self.world) {
            let Some(MeshAsset::Cube) = assets.meshes.get(mesh.0) else {
                continue;
            };

            let color = assets
                .materials
                .get(material.0)
                .map(material_color)
                .unwrap_or([1.0, 1.0, 1.0, 1.0]);

            instances.push(RenderInstance::new(render.0.compute_matrix(), color));

            if network_id.is_some_and(|id| self.local_player == Some(id.0)) {
                camera_target = Some(render.0.translation + Vec3::Y * 0.3);
            }
        }

        RenderScene {
            instances,
            camera_target,
        }
    }
}

fn material_color(material: &MaterialAsset) -> [f32; 4] {
    material.base_color.to_array()
}
