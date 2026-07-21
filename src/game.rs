use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

use glam::{Mat4, Vec3};
use rayon::prelude::*;

use crate::{
    camera::Camera,
    ecs_scene::EcsScene,
    input::InputState,
    mesh::{InstanceData, MeshData},
    physics::PhysicsWorld,
    voxel::{ChunkPos, VoxelWorld, build_chunk_mesh},
};

const PHYSICS_KEEPALIVE_HORIZONTAL_RADIUS: i32 = 1;
const PHYSICS_KEEPALIVE_CHUNKS_BELOW: i32 = 2;
const PHYSICS_KEEPALIVE_CHUNKS_ABOVE: i32 = 1;
const CHUNK_GENERATION_BUDGET_PER_FRAME: usize = 24;
const CHUNK_MESH_BUDGET_PER_FRAME: usize = 20;

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
    pub pending_generation_chunks: usize,
    pub pending_mesh_chunks: usize,
    pub frustum_visible_chunks: usize,
    pub frustum_culled_chunks: usize,
    pub solid_blocks: usize,
    pub rendered_triangles: usize,
    pub physics_bodies: usize,
    pub physics_colliders: usize,
    pub physics_waiting_for_chunks: bool,
    pub stream_ms: f32,
    pub mesh_ms: f32,
    pub physics_ms: f32,
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
    pending_meshes: HashSet<ChunkPos>,
    triangle_counts: HashMap<ChunkPos, usize>,
    total_rendered_triangles: usize,
    physics_accumulator: f32,
    physics_waiting_for_chunks: bool,
    last_stream_ms: f32,
    last_mesh_ms: f32,
    last_physics_ms: f32,
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
            pending_meshes: HashSet::new(),
            triangle_counts: HashMap::new(),
            total_rendered_triangles: 0,
            physics_accumulator: 0.0,
            physics_waiting_for_chunks: true,
            last_stream_ms: 0.0,
            last_mesh_ms: 0.0,
            last_physics_ms: 0.0,
        };

        for offset in [-2.0_f32, 0.0, 2.0] {
            game.spawn_physics_cube_at(Vec3::new(offset, 45.0 + offset.abs(), 0.0));
        }
        game.refresh_streaming(true);
        game
    }

    pub fn update(&mut self, input: &mut InputState, delta_time: f32) {
        self.camera.update(input, delta_time);
        self.refresh_streaming(false);

        const FIXED_STEP: f32 = 1.0 / 60.0;
        let physics_started = Instant::now();
        if self.physics_waiting_for_chunks {
            // Não acumulamos tempo durante o carregamento: isso evita um pico de
            // catch-up assim que os colliders ficam prontos.
            self.physics_accumulator = 0.0;
        } else {
            self.physics_accumulator += delta_time.min(0.1);
            while self.physics_accumulator >= FIXED_STEP {
                self.physics.step(FIXED_STEP);
                self.physics_accumulator -= FIXED_STEP;
            }
        }
        self.last_physics_ms = physics_started.elapsed().as_secs_f32() * 1000.0;

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

    pub fn dynamic_instances(&mut self) -> &[InstanceData] {
        self.ecs.render_instances()
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
        self.pending_meshes.clear();
        self.triangle_counts.clear();
        self.total_rendered_triangles = 0;
        self.physics_waiting_for_chunks = true;
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
        &self,
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
            pending_generation_chunks: self.voxel_world.pending_generation_count(),
            pending_mesh_chunks: self.pending_meshes.len(),
            frustum_visible_chunks,
            frustum_culled_chunks,
            solid_blocks: self.voxel_world.solid_block_count(),
            rendered_triangles: self.total_rendered_triangles,
            physics_bodies: self.physics.body_count(),
            physics_colliders: self.physics.collider_count(),
            physics_waiting_for_chunks: self.physics_waiting_for_chunks,
            stream_ms: self.last_stream_ms,
            mesh_ms: self.last_mesh_ms,
            physics_ms: self.last_physics_ms,
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

        let stream_started = Instant::now();
        let delta = self.voxel_world.stream_around(
            self.camera.position(),
            self.horizontal_radius,
            self.vertical_radius,
            &physics_keepalive,
            force,
            CHUNK_GENERATION_BUDGET_PER_FRAME,
        );
        self.last_stream_ms = stream_started.elapsed().as_secs_f32() * 1000.0;

        let mut renderer_removals = delta
            .render_removed
            .iter()
            .copied()
            .collect::<HashSet<_>>();

        for position in &delta.physics_removed {
            self.physics.remove_chunk_collider(*position);
        }

        for position in &delta.unloaded {
            self.physics.remove_chunk_collider(*position);
            self.pending_meshes.remove(position);
            self.remove_triangle_count(*position);
            renderer_removals.insert(*position);
        }

        for position in &renderer_removals {
            self.remove_triangle_count(*position);
        }
        self.chunk_updates.extend(
            renderer_removals
                .into_iter()
                .map(ChunkRenderUpdate::Remove),
        );

        self.pending_meshes.extend(
            self.voxel_world
                .take_dirty()
                .into_iter()
                .filter(|position| self.voxel_world.contains_chunk(*position)),
        );
        self.pending_meshes.extend(
            delta
                .physics_added
                .into_iter()
                .filter(|position| self.voxel_world.contains_chunk(*position)),
        );
        self.pending_meshes.extend(
            delta
                .render_added
                .into_iter()
                .filter(|position| self.voxel_world.contains_chunk(*position)),
        );

        let voxel_world = &self.voxel_world;
        self.pending_meshes.retain(|position| {
            voxel_world.contains_chunk(*position)
                && (voxel_world.is_render_visible(*position)
                    || voxel_world.is_physics_active(*position))
        });

        let center = self.voxel_world.current_center().unwrap_or(ChunkPos::new(0, 0, 0));
        let mut mesh_positions = self.pending_meshes.iter().copied().collect::<Vec<_>>();
        mesh_positions.sort_unstable_by_key(|position| {
            let physics_priority = if self.voxel_world.is_physics_active(*position) {
                0_i32
            } else {
                1_i32
            };
            (physics_priority, chunk_distance_squared(*position, center))
        });
        mesh_positions.truncate(CHUNK_MESH_BUDGET_PER_FRAME);

        for position in &mesh_positions {
            self.pending_meshes.remove(position);
        }

        let mesh_started = Instant::now();
        let generated_meshes = mesh_positions
            .into_par_iter()
            .map(|position| (position, build_chunk_mesh(&self.voxel_world, position)))
            .collect::<Vec<_>>();
        self.last_mesh_ms = mesh_started.elapsed().as_secs_f32() * 1000.0;

        for (position, mesh) in generated_meshes {
            if self.voxel_world.is_physics_active(position) {
                self.physics.upsert_chunk_collider(position, &mesh);
            }

            if self.voxel_world.is_render_visible(position) {
                self.set_triangle_count(position, mesh.triangle_count());
                self.chunk_updates
                    .push(ChunkRenderUpdate::Upsert(position, mesh));
            }
        }

        let voxel_world = &self.voxel_world;
        let physics = &self.physics;
        self.physics_waiting_for_chunks = voxel_world.physics_chunks().any(|position| {
            !voxel_world.contains_chunk(position) || !physics.has_processed_chunk(position)
        });
    }

    fn set_triangle_count(&mut self, position: ChunkPos, count: usize) {
        let previous = self.triangle_counts.insert(position, count).unwrap_or(0);
        self.total_rendered_triangles = self
            .total_rendered_triangles
            .saturating_sub(previous)
            .saturating_add(count);
    }

    fn remove_triangle_count(&mut self, position: ChunkPos) {
        if let Some(previous) = self.triangle_counts.remove(&position) {
            self.total_rendered_triangles = self.total_rendered_triangles.saturating_sub(previous);
        }
    }
}

#[inline]
fn chunk_distance_squared(position: ChunkPos, center: ChunkPos) -> i64 {
    let x = i64::from(position.x - center.x);
    let y = i64::from(position.y - center.y);
    let z = i64::from(position.z - center.z);
    x * x + y * y + z * z
}
