use std::collections::{HashMap, HashSet};

use glam::Vec3;
use rapier3d::prelude::*;

use crate::{mesh::MeshData, voxel::ChunkPos};

pub struct PhysicsWorld {
    pipeline: PhysicsPipeline,
    gravity: Vector,
    integration_parameters: IntegrationParameters,
    islands: IslandManager,
    broad_phase: BroadPhaseBvh,
    narrow_phase: NarrowPhase,
    bodies: RigidBodySet,
    colliders: ColliderSet,
    impulse_joints: ImpulseJointSet,
    multibody_joints: MultibodyJointSet,
    ccd_solver: CCDSolver,
    chunk_colliders: HashMap<ChunkPos, ColliderHandle>,
    processed_chunk_colliders: HashSet<ChunkPos>,
    keepalive_cache: HashSet<ChunkPos>,
    keepalive_body_chunks: HashMap<RigidBodyHandle, ChunkPos>,
    keepalive_parameters: (i32, i32, i32),
    keepalive_dirty: bool,
    paused: bool,
}

impl PhysicsWorld {
    pub fn new() -> Self {
        Self {
            pipeline: PhysicsPipeline::new(),
            gravity: Vector::new(0.0, -18.0, 0.0),
            integration_parameters: IntegrationParameters::default(),
            islands: IslandManager::new(),
            broad_phase: BroadPhaseBvh::new(),
            narrow_phase: NarrowPhase::new(),
            bodies: RigidBodySet::new(),
            colliders: ColliderSet::new(),
            impulse_joints: ImpulseJointSet::new(),
            multibody_joints: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
            chunk_colliders: HashMap::new(),
            processed_chunk_colliders: HashSet::new(),
            keepalive_cache: HashSet::new(),
            keepalive_body_chunks: HashMap::new(),
            keepalive_parameters: (-1, -1, -1),
            keepalive_dirty: true,
            paused: false,
        }
    }

    pub fn step(&mut self, delta_time: f32) {
        if self.paused {
            return;
        }

        self.integration_parameters.dt = delta_time;
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

    pub fn spawn_cube(&mut self, position: Vec3, half_extent: f32) -> RigidBodyHandle {
        let body = RigidBodyBuilder::dynamic()
            .translation(Vector::new(position.x, position.y, position.z))
            .linear_damping(0.08)
            .angular_damping(0.2)
            .lock_rotations()
            .build();
        let handle = self.bodies.insert(body);

        let collider = ColliderBuilder::cuboid(half_extent, half_extent, half_extent)
            .density(1.0)
            .friction(0.8)
            .restitution(0.05)
            .build();
        self.colliders
            .insert_with_parent(collider, handle, &mut self.bodies);
        self.keepalive_dirty = true;

        handle
    }

    pub fn upsert_chunk_collider(&mut self, position: ChunkPos, mesh: &MeshData) {
        self.remove_chunk_collider(position);
        self.processed_chunk_colliders.insert(position);
        if mesh.is_empty() {
            return;
        }

        let vertices = mesh
            .vertices
            .iter()
            .map(|vertex| {
                Vector::new(
                    vertex.position[0],
                    vertex.position[1],
                    vertex.position[2],
                )
            })
            .collect::<Vec<_>>();

        let triangles = mesh
            .indices
            .chunks_exact(3)
            .map(|triangle| [triangle[0], triangle[1], triangle[2]])
            .collect::<Vec<_>>();

        let Ok(builder) = ColliderBuilder::trimesh(vertices, triangles) else {
            self.processed_chunk_colliders.remove(&position);
            log::warn!("Não foi possível criar collider do chunk {position:?}");
            return;
        };

        let handle = self
            .colliders
            .insert(builder.friction(0.95).restitution(0.0).build());
        self.chunk_colliders.insert(position, handle);
    }

    pub fn remove_chunk_collider(&mut self, position: ChunkPos) {
        self.processed_chunk_colliders.remove(&position);
        let Some(handle) = self.chunk_colliders.remove(&position) else {
            return;
        };
        self.colliders
            .remove(handle, &mut self.islands, &mut self.bodies, true);
    }

    pub fn clear_chunk_colliders(&mut self) {
        let positions = self
            .processed_chunk_colliders
            .iter()
            .copied()
            .collect::<Vec<_>>();
        for position in positions {
            self.remove_chunk_collider(position);
        }
    }


    /// Retorna os chunks que precisam permanecer carregados para sustentar
    /// corpos dinâmicos, mesmo quando eles estiverem fora do raio visual.
    ///
    /// `below` deve ser maior que zero para manter terreno suficiente abaixo de
    /// objetos em queda. Como o carregamento é concluído antes do próximo passo
    /// da física, o volume protegido acompanha os corpos sem criar buracos.
    pub fn required_chunk_keepalive(
        &mut self,
        horizontal_radius: i32,
        below: i32,
        above: i32,
    ) -> HashSet<ChunkPos> {
        let horizontal_radius = horizontal_radius.max(0);
        let below = below.max(0);
        let above = above.max(0);
        let parameters = (horizontal_radius, below, above);

        let current_body_chunks = self
            .bodies
            .iter()
            .map(|(handle, body)| {
                let translation = body.translation();
                let chunk = ChunkPos::from_world_position(Vec3::new(
                    translation.x,
                    translation.y,
                    translation.z,
                ));
                (handle, chunk)
            })
            .collect::<HashMap<_, _>>();

        if !self.keepalive_dirty
            && self.keepalive_parameters == parameters
            && self.keepalive_body_chunks == current_body_chunks
        {
            return self.keepalive_cache.clone();
        }

        self.keepalive_dirty = false;
        self.keepalive_parameters = parameters;
        self.keepalive_body_chunks = current_body_chunks;
        self.keepalive_cache.clear();

        let radius_squared = horizontal_radius * horizontal_radius;
        for center in self.keepalive_body_chunks.values().copied() {
            for y in -below..=above {
                for z in -horizontal_radius..=horizontal_radius {
                    for x in -horizontal_radius..=horizontal_radius {
                        if x * x + z * z > radius_squared {
                            continue;
                        }

                        self.keepalive_cache.insert(ChunkPos::new(
                            center.x + x,
                            center.y + y,
                            center.z + z,
                        ));
                    }
                }
            }
        }

        self.keepalive_cache.clone()
    }


    pub fn has_processed_chunk(&self, position: ChunkPos) -> bool {
        self.processed_chunk_colliders.contains(&position)
    }

    pub fn bodies(&self) -> &RigidBodySet {
        &self.bodies
    }

    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }

    pub fn collider_count(&self) -> usize {
        self.colliders.len()
    }

    pub fn set_paused(&mut self, paused: bool) {
        self.paused = paused;
    }

    pub fn paused(&self) -> bool {
        self.paused
    }

    pub fn set_gravity_y(&mut self, gravity_y: f32) {
        self.gravity.y = gravity_y;
    }

    pub fn gravity_y(&self) -> f32 {
        self.gravity.y
    }
}
