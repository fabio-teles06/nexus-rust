use bevy_ecs::prelude::*;
use glam::Vec3;
use rapier3d::prelude::*;

use crate::{
    body::{PhysicsBody, PhysicsBodyState},
    conversion::{from_rapier_rotation, from_rapier_vector, to_rapier_vector},
};

#[derive(Resource)]
pub struct PhysicsWorld {
    gravity: Vector,
    integration_parameters: IntegrationParameters,
    pipeline: PhysicsPipeline,
    islands: IslandManager,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
}

impl Default for PhysicsWorld {
    fn default() -> Self {
        let mut world = Self {
            gravity: Vector::new(0.0, -19.62, 0.0),
            integration_parameters: IntegrationParameters::default(),
            pipeline: PhysicsPipeline::new(),
            islands: IslandManager::new(),
            broad_phase: BroadPhaseBvh::new(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
        };

        // Piso: o topo fica em y = 0.0, igual à representação visual.
        world.add_static_box(Vec3::new(0.0, -0.1, 0.0), Vec3::new(15.0, 0.1, 15.0), 0.9);

        // Paredes laterais usadas pela cena de demonstração.
        for x in [-4.0, 4.0] {
            world.add_static_box(Vec3::new(x, 1.0, 0.0), Vec3::new(0.5, 1.0, 3.0), 0.8);
        }

        world
    }
}

impl PhysicsWorld {
    /// Adiciona um collider estático sem rigid-body pai.
    pub fn add_static_box(
        &mut self,
        center: Vec3,
        half_extents: Vec3,
        friction: f32,
    ) -> ColliderHandle {
        let collider = ColliderBuilder::cuboid(half_extents.x, half_extents.y, half_extents.z)
            .translation(to_rapier_vector(center))
            .friction(friction)
            .restitution(0.0)
            .build();

        self.colliders.insert(collider)
    }

    /// Cria um corpo dinâmico em forma de caixa para uma entidade ECS.
    pub fn create_dynamic_box(
        &mut self,
        position: Vec3,
        half_extents: Vec3,
        user_data: u128,
    ) -> PhysicsBody {
        let rigid_body = RigidBodyBuilder::dynamic()
            .translation(to_rapier_vector(position))
            .lock_rotations()
            .can_sleep(false)
            .ccd_enabled(true)
            .user_data(user_data)
            .build();

        let rigid_body_handle = self.bodies.insert(rigid_body);

        let collider = ColliderBuilder::cuboid(half_extents.x, half_extents.y, half_extents.z)
            .friction(0.0)
            .restitution(0.0)
            .density(1.0)
            .build();

        let collider_handle =
            self.colliders
                .insert_with_parent(collider, rigid_body_handle, &mut self.bodies);

        PhysicsBody {
            rigid_body: rigid_body_handle,
            collider: collider_handle,
        }
    }

    /// Mantém a velocidade vertical calculada pelo Rapier e substitui apenas
    /// os eixos controlados pelo jogador.
    pub fn set_horizontal_velocity(&mut self, body: PhysicsBody, desired_velocity: Vec3) {
        let Some(rigid_body) = self.bodies.get_mut(body.rigid_body) else {
            return;
        };

        let vertical_velocity = rigid_body.linvel().y;

        rigid_body.set_linvel(
            Vector::new(desired_velocity.x, vertical_velocity, desired_velocity.z),
            true,
        );
    }

    /// Executa um único passo fixo da simulação.
    pub fn step(&mut self, delta_seconds: f32) {
        self.integration_parameters.dt = delta_seconds.max(f32::EPSILON);

        self.pipeline.step(
            self.gravity,
            &self.integration_parameters,
            &mut self.islands,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.bodies,
            &mut self.colliders,
            &mut self.impulse_joints,
            &mut self.multibody_joints,
            &mut self.ccd_solver,
            &(),
            &(),
        );
    }

    pub fn body_state(&self, body: PhysicsBody) -> Option<PhysicsBodyState> {
        let rigid_body = self.bodies.get(body.rigid_body)?;

        Some(PhysicsBodyState {
            translation: from_rapier_vector(rigid_body.translation()),
            rotation: from_rapier_rotation(rigid_body.rotation()),
            linear_velocity: from_rapier_vector(rigid_body.linvel()),
        })
    }

    pub fn linear_velocity(&self, body: PhysicsBody) -> Option<Vec3> {
        self.bodies
            .get(body.rigid_body)
            .map(|rigid_body| from_rapier_vector(rigid_body.linvel()))
    }

    /// Remove o corpo e todos os colliders e joints anexados.
    pub fn remove_body(&mut self, body: PhysicsBody) -> bool {
        self.bodies
            .remove(
                body.rigid_body,
                &mut self.islands,
                &mut self.colliders,
                &mut self.impulse_joints,
                &mut self.multibody_joints,
                true,
            )
            .is_some()
    }

    pub fn rigid_body_count(&self) -> usize {
        self.bodies.len()
    }

    pub fn collider_count(&self) -> usize {
        self.colliders.len()
    }
}
