use std::collections::{HashMap, HashSet};

use glam::{Mat4, Vec3};
use rayon::prelude::*;

use crate::{
    camera::Camera,
    ecs_scene::EcsScene,
    input::InputState,
    mesh::MeshData,
    physics::PhysicsWorld,
    voxel::{ChunkPos, VoxelWorld, build_chunk_mesh},
};

const PHYSICS_KEEPALIVE_HORIZONTAL_RADIUS: i32 = 1;
const PHYSICS_KEEPALIVE_CHUNKS_BELOW: i32 = 2;
const PHYSICS_KEEPALIVE_CHUNKS_ABOVE: i32 = 1;

pub enum ChunkRenderUpdate {
    Upsert(ChunkPos, MeshData),
    Remove(ChunkPos),
}

#[derive(Clone)]
pub struct DebugSnapshot {
    pub fps: f32,
    pub frame_ms: f32,
    pub camera_position: Vec3,
    pub center_chunk: Option<ChunkPos>,
    pub loaded_chunks: usize,
    pub render_radius_chunks: usize,
    pub physics_keepalive_chunks: usize,
    pub frustum_visible_chunks: usize,
    pub frustum_culled_chunks: usize,
    pub solid_blocks: usize,
    pub rendered_triangles: usize,
    pub physics_bodies: usize,
    pub physics_colliders: usize,
    pub ecs_entities: usize,
    pub first_entity_name: Option<String>,
    pub physics_paused: bool,
    pub gravity_y: f32,
}

pub struct Game {
    camera: Camera,
    voxel_world: VoxelWorld,
    physics: PhysicsWorld,
    ecs: EcsScene,
    horizontal_radius: i32,
    vertical_radius: i32,
    chunk_updates: Vec<ChunkRenderUpdate>,
    triangle_counts: HashMap<ChunkPos, usize>,
    physics_accumulator: f32,
}

impl Game {
    pub fn new(width: u32, height: u32) -> Self {
        let mut game = Self {
            camera: Camera::new(width, height),
            voxel_world: VoxelWorld::new(0xC0FFEE),
            physics: PhysicsWorld::new(),
            ecs: EcsScene::new(),
            horizontal_radius: 6,
            vertical_radius: 3,
            chunk_updates: Vec::new(),
            triangle_counts: HashMap::new(),
            physics_accumulator: 0.0,
        };

        game.refresh_streaming(true);
        for offset in [-2.0_f32, 0.0, 2.0] {
            game.spawn_physics_cube_at(Vec3::new(offset, 45.0 + offset.abs(), 0.0));
        }

        // Os corpos acabaram de ser criados, então atualizamos novamente para
        // incluir imediatamente os chunks protegidos pela física.
        game.refresh_streaming(true);
        game
    }

    pub fn update(&mut self, input: &mut InputState, delta_time: f32) {
        self.camera.update(input, delta_time);

        // Carrega o terreno necessário antes de avançar a simulação. Assim um
        // corpo nunca executa um passo sobre uma região cujo collider foi removido.
        self.refresh_streaming(false);

        const FIXED_STEP: f32 = 1.0 / 60.0;
        self.physics_accumulator += delta_time.min(0.1);
        while self.physics_accumulator >= FIXED_STEP {
            self.physics.step(FIXED_STEP);
            self.physics_accumulator -= FIXED_STEP;
        }
        self.ecs.sync_from_physics(self.physics.bodies());
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.camera.resize(width, height);
    }

    pub fn camera_matrix(&self) -> Mat4 {
        self.camera.view_projection()
    }

    pub fn camera_position(&self) -> Vec3 {
        self.camera.position()
    }

    pub fn drain_chunk_updates(&mut self) -> Vec<ChunkRenderUpdate> {
        std::mem::take(&mut self.chunk_updates)
    }

    pub fn dynamic_mesh(&mut self) -> MeshData {
        self.ecs.build_render_mesh()
    }

    pub fn spawn_cube_in_front(&mut self) {
        let position = self.camera.position() + self.camera.forward() * 5.0 + Vec3::Y * 3.0;
        self.spawn_physics_cube_at(position);
        self.refresh_streaming(true);
    }

    pub fn regenerate_chunks(&mut self, horizontal_radius: i32, vertical_radius: i32) {
        self.horizontal_radius = horizontal_radius.max(0);
        self.vertical_radius = vertical_radius.max(0);

        let removed = self.voxel_world.clear();
        self.physics.clear_chunk_colliders();
        self.triangle_counts.clear();
        self.chunk_updates
            .extend(removed.into_iter().map(ChunkRenderUpdate::Remove));
        self.refresh_streaming(true);
    }

    pub fn set_physics_paused(&mut self, paused: bool) {
        self.physics.set_paused(paused);
    }

    pub fn set_gravity_y(&mut self, gravity_y: f32) {
        self.physics.set_gravity_y(gravity_y);
    }

    pub fn horizontal_radius(&self) -> i32 {
        self.horizontal_radius
    }

    pub fn vertical_radius(&self) -> i32 {
        self.vertical_radius
    }

    pub fn debug_snapshot(
        &mut self,
        delta_time: f32,
        frustum_visible_chunks: usize,
        frustum_culled_chunks: usize,
    ) -> DebugSnapshot {
        let fps = if delta_time > 0.0 {
            1.0 / delta_time
        } else {
            0.0
        };

        DebugSnapshot {
            fps,
            frame_ms: delta_time * 1000.0,
            camera_position: self.camera.position(),
            center_chunk: self.voxel_world.current_center(),
            loaded_chunks: self.voxel_world.chunk_count(),
            render_radius_chunks: self.voxel_world.render_chunk_count(),
            physics_keepalive_chunks: self.voxel_world.physics_keepalive_count(),
            frustum_visible_chunks,
            frustum_culled_chunks,
            solid_blocks: self.voxel_world.solid_block_count(),
            rendered_triangles: self.triangle_counts.values().sum(),
            physics_bodies: self.physics.body_count(),
            physics_colliders: self.physics.collider_count(),
            ecs_entities: self.ecs.entity_count(),
            first_entity_name: self.ecs.first_entity_name(),
            physics_paused: self.physics.paused(),
            gravity_y: self.physics.gravity_y(),
        }
    }

    fn spawn_physics_cube_at(&mut self, position: Vec3) {
        let half_extent = 0.6;
        let handle = self.physics.spawn_cube(position, half_extent);
        self.ecs
            .spawn_physics_cube(handle, position, half_extent * 2.0);
    }

    fn refresh_streaming(&mut self, force: bool) {
        let physics_keepalive = self.physics.required_chunk_keepalive(
            PHYSICS_KEEPALIVE_HORIZONTAL_RADIUS,
            PHYSICS_KEEPALIVE_CHUNKS_BELOW,
            PHYSICS_KEEPALIVE_CHUNKS_ABOVE,
        );

        let delta = self.voxel_world.stream_around(
            self.camera.position(),
            self.horizontal_radius,
            self.vertical_radius,
            &physics_keepalive,
            force,
        );

        let mut renderer_removals = delta
            .render_removed
            .into_iter()
            .collect::<HashSet<_>>();

        for position in &delta.physics_removed {
            self.physics.remove_chunk_collider(*position);
        }

        for position in &delta.unloaded {
            self.physics.remove_chunk_collider(*position);
            self.triangle_counts.remove(position);
            renderer_removals.insert(*position);
        }

        self.chunk_updates.extend(
            renderer_removals
                .into_iter()
                .map(ChunkRenderUpdate::Remove),
        );

        let dirty_positions = self
            .voxel_world
            .take_dirty()
            .into_iter()
            .filter(|position| self.voxel_world.contains_chunk(*position))
            .collect::<HashSet<_>>();

        let physics_added = delta
            .physics_added
            .into_iter()
            .filter(|position| self.voxel_world.contains_chunk(*position))
            .collect::<HashSet<_>>();

        let mut mesh_positions = dirty_positions.clone();
        mesh_positions.extend(physics_added.iter().copied());
        mesh_positions.extend(
            delta
                .render_added
                .into_iter()
                .filter(|position| self.voxel_world.contains_chunk(*position)),
        );

        let mut mesh_positions = mesh_positions.into_iter().collect::<Vec<_>>();
        mesh_positions.sort_unstable();

        // O mesher só lê VoxelWorld, portanto cada chunk pode ser processado em
        // paralelo com segurança. GPU e Rapier recebem os resultados em série.
        let generated_meshes = mesh_positions
            .into_par_iter()
            .map(|position| (position, build_chunk_mesh(&self.voxel_world, position)))
            .collect::<Vec<_>>();

        for (position, mesh) in generated_meshes {
            if dirty_positions.contains(&position) || physics_added.contains(&position) {
                if self.voxel_world.is_physics_active(position) {
                    self.physics.upsert_chunk_collider(position, &mesh);
                }

                self.triangle_counts
                    .insert(position, mesh.triangle_count());
            }

            if self.voxel_world.is_render_visible(position) {
                self.chunk_updates
                    .push(ChunkRenderUpdate::Upsert(position, mesh));
            }
        }
    }
}
