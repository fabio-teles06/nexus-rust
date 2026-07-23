use bevy_ecs::prelude::*;
use engine_assets::{Assets, Handle, MaterialAsset, MeshAsset};
use engine_ecs::{RenderTransform, Transform};
use glam::{Vec3, Vec4};

use crate::components::{MaterialHandle, MeshHandle};

#[derive(Resource)]
pub(crate) struct ClientAssets {
    pub meshes: Assets<MeshAsset>,
    pub materials: Assets<MaterialAsset>,
    pub cube: Handle<MeshAsset>,
    pub local_material: Handle<MaterialAsset>,
    pub remote_material: Handle<MaterialAsset>,
    pub floor_material: Handle<MaterialAsset>,
    pub wall_material: Handle<MaterialAsset>,
}

impl Default for ClientAssets {
    fn default() -> Self {
        let mut meshes = Assets::default();
        let cube = meshes.add(MeshAsset::Cube);

        let mut materials = Assets::default();
        let local_material = materials.add(MaterialAsset::new(Vec4::new(0.1, 0.45, 1.0, 1.0)));
        let remote_material = materials.add(MaterialAsset::new(Vec4::new(1.0, 0.3, 0.1, 1.0)));
        let floor_material = materials.add(MaterialAsset::new(Vec4::new(0.2, 0.28, 0.2, 1.0)));
        let wall_material = materials.add(MaterialAsset::new(Vec4::new(0.42, 0.3, 0.18, 1.0)));

        Self {
            meshes,
            materials,
            cube,
            local_material,
            remote_material,
            floor_material,
            wall_material,
        }
    }
}

pub(crate) fn initialize_assets_and_scene(world: &mut World) {
    world.insert_resource(ClientAssets::default());

    let (cube, floor, wall) = {
        let assets = world.resource::<ClientAssets>();
        (assets.cube, assets.floor_material, assets.wall_material)
    };

    world.spawn((
        MeshHandle(cube),
        MaterialHandle(floor),
        RenderTransform(Transform {
            translation: Vec3::new(0.0, -0.1, 0.0),
            scale: Vec3::new(30.0, 0.2, 30.0),
            ..Transform::IDENTITY
        }),
    ));

    for x in [-4.0, 4.0] {
        world.spawn((
            MeshHandle(cube),
            MaterialHandle(wall),
            RenderTransform(Transform {
                translation: Vec3::new(x, 1.0, 0.0),
                scale: Vec3::new(1.0, 2.0, 6.0),
                ..Transform::IDENTITY
            }),
        ));
    }
}
