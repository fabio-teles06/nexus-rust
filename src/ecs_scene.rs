use bevy_ecs::prelude::*;
use glam::{Mat4, Quat, Vec3};
use rapier3d::prelude::{RigidBodyHandle, RigidBodySet};

use crate::mesh::InstanceData;

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
    entity_count: usize,
    first_entity_name: Option<String>,
    instance_scratch: Vec<InstanceData>,
}

impl EcsScene {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            next_cube: 1,
            entity_count: 0,
            first_entity_name: None,
            instance_scratch: Vec::new(),
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

        if self.first_entity_name.is_none() {
            self.first_entity_name = Some(name.clone());
        }

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
        self.entity_count += 1;
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

    /// Reutiliza a mesma alocação a cada frame e envia apenas uma matriz e uma
    /// cor por entidade. A geometria do cubo permanece estática na GPU.
    pub fn render_instances(&mut self) -> &[InstanceData] {
        self.instance_scratch.clear();
        if self.instance_scratch.capacity() < self.entity_count {
            self.instance_scratch.reserve(self.entity_count);
        }

        let mut query = self.world.query::<(&Transform, &RenderCube)>();
        for (transform, cube) in query.iter(&self.world) {
            let model = Mat4::from_scale_rotation_translation(
                transform.scale,
                transform.rotation,
                transform.translation,
            );
            self.instance_scratch
                .push(InstanceData::new(model, cube.color));
        }

        &self.instance_scratch
    }

    pub const fn entity_count(&self) -> usize {
        self.entity_count
    }

    pub fn first_entity_name(&self) -> Option<String> {
        self.first_entity_name.clone()
    }
}
