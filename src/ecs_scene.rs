use bevy_ecs::prelude::*;
use glam::{Mat4, Quat, Vec3};
use rapier3d::prelude::{RigidBodyHandle, RigidBodySet};

use crate::mesh::{MeshData, append_cube};

#[derive(Component, Debug, Clone)]
pub struct DebugName(pub String);

#[derive(Component, Debug, Clone, Copy)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct PhysicsBody {
    pub handle: RigidBodyHandle,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct RenderCube {
    pub color: [f32; 3],
}

pub struct EcsScene {
    world: World,
    next_cube: u64,
}

impl EcsScene {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            next_cube: 1,
        }
    }

    pub fn spawn_physics_cube(
        &mut self,
        handle: RigidBodyHandle,
        position: Vec3,
        size: f32,
    ) {
        let color_variant = (self.next_cube % 3) as f32;
        let color = [0.85 - color_variant * 0.12, 0.35 + color_variant * 0.16, 0.18];
        let name = format!("Physics Cube {}", self.next_cube);
        self.next_cube += 1;

        self.world.spawn((
            DebugName(name),
            Transform {
                translation: position,
                rotation: Quat::IDENTITY,
                scale: Vec3::splat(size),
            },
            PhysicsBody { handle },
            RenderCube { color },
        ));
    }

    pub fn sync_from_physics(&mut self, bodies: &RigidBodySet) {
        let mut query = self.world.query::<(&PhysicsBody, &mut Transform)>();
        for (physics_body, mut transform) in query.iter_mut(&mut self.world) {
            let Some(body) = bodies.get(physics_body.handle) else {
                continue;
            };
            let translation = body.translation();
            transform.translation = Vec3::new(translation.x, translation.y, translation.z);
        }
    }

    pub fn build_render_mesh(&mut self) -> MeshData {
        let mut mesh = MeshData::default();
        let mut query = self.world.query::<(&Transform, &RenderCube)>();

        for (transform, cube) in query.iter(&self.world) {
            let model = Mat4::from_scale_rotation_translation(
                transform.scale,
                transform.rotation,
                transform.translation,
            );
            append_cube(&mut mesh, model, cube.color);
        }

        mesh
    }

    pub fn entity_count(&mut self) -> usize {
        let mut query = self.world.query::<&Transform>();
        query.iter(&self.world).count()
    }

    pub fn first_entity_name(&mut self) -> Option<String> {
        let mut query = self.world.query::<&DebugName>();
        query.iter(&self.world).next().map(|name| name.0.clone())
    }
}
