use std::collections::{HashMap, HashSet, VecDeque};

use glam::{IVec3, Vec3};
use rayon::prelude::*;

use super::{
    block::{AIR, BlockId},
    chunk::{Chunk, ChunkPos, world_to_local},
    generator::generate_chunk,
};

#[derive(Debug, Default)]
pub struct StreamDelta {
    pub loaded: Vec<ChunkPos>,
    pub unloaded: Vec<ChunkPos>,
    pub render_added: Vec<ChunkPos>,
    pub render_removed: Vec<ChunkPos>,
    pub physics_added: Vec<ChunkPos>,
    pub physics_removed: Vec<ChunkPos>,
}

pub struct VoxelWorld {
    chunks: HashMap<ChunkPos, Chunk>,
    dirty: HashSet<ChunkPos>,
    render_chunks: HashSet<ChunkPos>,
    physics_keepalive: HashSet<ChunkPos>,
    render_offsets: Vec<IVec3>,
    render_radius_config: (i32, i32),
    pending_generation: VecDeque<ChunkPos>,
    stream_center: Option<ChunkPos>,
    seed: u32,
    total_solid_blocks: usize,
}

impl VoxelWorld {
    pub fn new(seed: u32) -> Self {
        Self {
            chunks: HashMap::new(),
            dirty: HashSet::new(),
            render_chunks: HashSet::new(),
            physics_keepalive: HashSet::new(),
            render_offsets: vec![IVec3::ZERO],
            render_radius_config: (0, 0),
            pending_generation: VecDeque::new(),
            stream_center: None,
            seed,
            total_solid_blocks: 0,
        }
    }

    /// Atualiza os conjuntos desejados e gera no máximo `generation_budget`
    /// chunks nesta chamada. O lote selecionado continua sendo processado em
    /// paralelo com Rayon, mas o custo total de cada frame fica limitado.
    pub fn stream_around(
        &mut self,
        world_position: Vec3,
        horizontal_radius: i32,
        vertical_radius: i32,
        physics_keepalive: &HashSet<ChunkPos>,
        force: bool,
        generation_budget: usize,
    ) -> StreamDelta {
        let center = ChunkPos::from_world_position(world_position);
        let horizontal_radius = horizontal_radius.max(0);
        let vertical_radius = vertical_radius.max(0);
        let radius_config = (horizontal_radius, vertical_radius);
        let shape_changed = self.render_radius_config != radius_config;
        if shape_changed {
            self.render_offsets = build_render_offsets(horizontal_radius, vertical_radius);
            self.render_radius_config = radius_config;
        }

        let sets_changed = force
            || shape_changed
            || self.stream_center != Some(center)
            || !self.physics_keepalive.eq(physics_keepalive);

        let mut delta = StreamDelta::default();

        if sets_changed {
            self.stream_center = Some(center);

            let render_desired = desired_render_chunks(center, &self.render_offsets);

            delta.render_added = render_desired
                .difference(&self.render_chunks)
                .copied()
                .collect();
            delta.render_removed = self
                .render_chunks
                .difference(&render_desired)
                .copied()
                .collect();
            delta.physics_added = physics_keepalive
                .difference(&self.physics_keepalive)
                .copied()
                .collect();
            delta.physics_removed = self
                .physics_keepalive
                .difference(physics_keepalive)
                .copied()
                .collect();

            self.render_chunks = render_desired;
            self.physics_keepalive = physics_keepalive.clone();

            let existing = self.chunks.keys().copied().collect::<Vec<_>>();
            for position in existing {
                if self.is_loaded_desired(position) {
                    continue;
                }

                if let Some(chunk) = self.chunks.remove(&position) {
                    self.total_solid_blocks -= chunk.solid_count();
                }
                self.dirty.remove(&position);
                delta.unloaded.push(position);
                self.mark_loaded_neighbors_dirty(position);
            }

            self.rebuild_generation_queue(center);
        }

        let mut batch = Vec::with_capacity(generation_budget.min(self.pending_generation.len()));
        while batch.len() < generation_budget {
            let Some(position) = self.pending_generation.pop_front() else {
                break;
            };

            // A fila pode conter entradas obsoletas após uma troca rápida de raio.
            if !self.is_loaded_desired(position) || self.chunks.contains_key(&position) {
                continue;
            }

            batch.push(position);
        }

        let seed = self.seed;
        let generated = batch
            .into_par_iter()
            .map(|position| (position, generate_chunk(position, seed)))
            .collect::<Vec<_>>();

        for (position, chunk) in generated {
            self.total_solid_blocks += chunk.solid_count();
            self.chunks.insert(position, chunk);
            delta.loaded.push(position);
            self.mark_dirty_with_neighbors(position);
        }

        delta
    }

    pub fn clear(&mut self) -> Vec<ChunkPos> {
        let removed = self.chunks.keys().copied().collect();
        self.chunks.clear();
        self.dirty.clear();
        self.render_chunks.clear();
        self.physics_keepalive.clear();
        self.render_offsets.clear();
        self.render_offsets.push(IVec3::ZERO);
        self.render_radius_config = (0, 0);
        self.pending_generation.clear();
        self.stream_center = None;
        self.total_solid_blocks = 0;
        removed
    }

    #[inline]
    pub fn get_block(&self, world: IVec3) -> BlockId {
        let chunk_position = ChunkPos::from_world_block(world);
        let Some(chunk) = self.chunks.get(&chunk_position) else {
            return AIR;
        };
        chunk.get(world_to_local(world))
    }

    #[allow(dead_code)]
    pub fn set_block(&mut self, world: IVec3, block: BlockId) -> bool {
        let chunk_position = ChunkPos::from_world_block(world);
        let Some(chunk) = self.chunks.get_mut(&chunk_position) else {
            return false;
        };

        let previous = chunk.get(world_to_local(world));
        if !chunk.set(world_to_local(world), block) {
            return false;
        }

        if previous.is_solid() {
            self.total_solid_blocks -= 1;
        }
        if block.is_solid() {
            self.total_solid_blocks += 1;
        }

        self.mark_dirty_with_neighbors(chunk_position);
        true
    }

    pub fn chunk(&self, position: ChunkPos) -> Option<&Chunk> {
        self.chunks.get(&position)
    }

    pub fn contains_chunk(&self, position: ChunkPos) -> bool {
        self.chunks.contains_key(&position)
    }

    pub fn is_render_visible(&self, position: ChunkPos) -> bool {
        self.render_chunks.contains(&position)
    }

    pub fn is_physics_active(&self, position: ChunkPos) -> bool {
        self.physics_keepalive.contains(&position)
    }

    pub fn physics_chunks(&self) -> impl Iterator<Item = ChunkPos> + '_ {
        self.physics_keepalive.iter().copied()
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn render_chunk_count(&self) -> usize {
        self.render_chunks.len()
    }

    pub fn physics_keepalive_count(&self) -> usize {
        self.physics_keepalive.len()
    }

    pub fn solid_block_count(&self) -> usize {
        self.total_solid_blocks
    }

    pub fn pending_generation_count(&self) -> usize {
        self.pending_generation.len()
    }

    pub fn current_center(&self) -> Option<ChunkPos> {
        self.stream_center
    }

    pub fn take_dirty(&mut self) -> Vec<ChunkPos> {
        self.dirty.drain().collect()
    }

    fn rebuild_generation_queue(&mut self, center: ChunkPos) {
        let mut missing = self
            .render_chunks
            .iter()
            .chain(self.physics_keepalive.iter())
            .copied()
            .filter(|position| !self.chunks.contains_key(position))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();

        missing.sort_unstable_by_key(|position| {
            let physics_priority = if self.physics_keepalive.contains(position) {
                0_i32
            } else {
                1_i32
            };
            (physics_priority, chunk_distance_squared(*position, center))
        });

        self.pending_generation = missing.into();
    }

    #[inline]
    fn is_loaded_desired(&self, position: ChunkPos) -> bool {
        self.render_chunks.contains(&position) || self.physics_keepalive.contains(&position)
    }

    fn mark_dirty_with_neighbors(&mut self, position: ChunkPos) {
        if self.chunks.contains_key(&position) {
            self.dirty.insert(position);
        }
        self.mark_loaded_neighbors_dirty(position);
    }

    fn mark_loaded_neighbors_dirty(&mut self, position: ChunkPos) {
        for neighbor in position.neighbors() {
            if self.chunks.contains_key(&neighbor) {
                self.dirty.insert(neighbor);
            }
        }
    }
}

fn build_render_offsets(horizontal_radius: i32, vertical_radius: i32) -> Vec<IVec3> {
    let radius_squared = horizontal_radius * horizontal_radius;
    let mut offsets = Vec::new();

    for y in -vertical_radius..=vertical_radius {
        for z in -horizontal_radius..=horizontal_radius {
            for x in -horizontal_radius..=horizontal_radius {
                if x * x + z * z <= radius_squared {
                    offsets.push(IVec3::new(x, y, z));
                }
            }
        }
    }

    offsets
}

fn desired_render_chunks(center: ChunkPos, offsets: &[IVec3]) -> HashSet<ChunkPos> {
    let mut desired = HashSet::with_capacity(offsets.len());
    desired.extend(offsets.iter().map(|offset| {
        ChunkPos::new(
            center.x + offset.x,
            center.y + offset.y,
            center.z + offset.z,
        )
    }));
    desired
}

#[inline]
fn chunk_distance_squared(position: ChunkPos, center: ChunkPos) -> i64 {
    let x = i64::from(position.x - center.x);
    let y = i64::from(position.y - center.y);
    let z = i64::from(position.z - center.z);
    x * x + y * y + z * z
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_chunks_on_the_vertical_axis() {
        let mut world = VoxelWorld::new(123);
        let delta = world.stream_around(
            Vec3::new(0.0, 24.0, 0.0),
            0,
            2,
            &HashSet::new(),
            true,
            usize::MAX,
        );

        assert_eq!(delta.loaded.len(), 5);
        assert!(delta.loaded.iter().any(|position| position.y == -1));
        assert!(delta.loaded.iter().any(|position| position.y == 3));
    }

    #[test]
    fn generation_budget_is_respected() {
        let mut world = VoxelWorld::new(123);
        let delta = world.stream_around(
            Vec3::ZERO,
            4,
            2,
            &HashSet::new(),
            true,
            3,
        );
        assert_eq!(delta.loaded.len(), 3);
        assert!(world.pending_generation_count() > 0);
    }

    #[test]
    fn moving_vertically_changes_the_stream_center() {
        let mut world = VoxelWorld::new(123);
        world.stream_around(
            Vec3::new(0.0, 1.0, 0.0),
            0,
            0,
            &HashSet::new(),
            true,
            usize::MAX,
        );
        let lower = world.current_center();

        world.stream_around(
            Vec3::new(0.0, 33.0, 0.0),
            0,
            0,
            &HashSet::new(),
            false,
            usize::MAX,
        );
        let upper = world.current_center();

        assert_ne!(lower, upper);
        assert_eq!(upper, Some(ChunkPos::new(0, 2, 0)));
    }

    #[test]
    fn physics_keepalive_prevents_unloading() {
        let mut world = VoxelWorld::new(123);
        let protected = HashSet::from([ChunkPos::new(10, 0, 0)]);

        world.stream_around(Vec3::ZERO, 0, 0, &protected, true, usize::MAX);
        assert!(world.contains_chunk(ChunkPos::new(10, 0, 0)));

        world.stream_around(
            Vec3::new(64.0, 0.0, 0.0),
            0,
            0,
            &protected,
            false,
            usize::MAX,
        );
        assert!(world.contains_chunk(ChunkPos::new(10, 0, 0)));
        assert!(!world.is_render_visible(ChunkPos::new(10, 0, 0)));
    }
}
