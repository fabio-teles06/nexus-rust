use std::collections::{HashMap, HashSet};

use glam::{IVec3, Vec3};

use super::{
    block::{AIR, BlockId},
    chunk::{Chunk, ChunkPos, world_to_local},
    generator::generate_chunk,
};

#[derive(Debug, Default)]
pub struct StreamDelta {
    pub loaded: Vec<ChunkPos>,
    pub unloaded: Vec<ChunkPos>,
}

pub struct VoxelWorld {
    chunks: HashMap<ChunkPos, Chunk>,
    dirty: HashSet<ChunkPos>,
    stream_center: Option<ChunkPos>,
    seed: u32,
}

impl VoxelWorld {
    pub fn new(seed: u32) -> Self {
        Self {
            chunks: HashMap::new(),
            dirty: HashSet::new(),
            stream_center: None,
            seed,
        }
    }

    pub fn stream_around(
        &mut self,
        world_position: Vec3,
        horizontal_radius: i32,
        vertical_radius: i32,
        force: bool,
    ) -> StreamDelta {
        let center = ChunkPos::from_world_position(world_position);
        if !force && self.stream_center == Some(center) {
            return StreamDelta::default();
        }

        self.stream_center = Some(center);
        let mut desired = HashSet::new();

        for y in -vertical_radius..=vertical_radius {
            for z in -horizontal_radius..=horizontal_radius {
                for x in -horizontal_radius..=horizontal_radius {
                    desired.insert(ChunkPos::new(center.x + x, center.y + y, center.z + z));
                }
            }
        }

        let existing: Vec<_> = self.chunks.keys().copied().collect();
        let mut delta = StreamDelta::default();

        for position in existing {
            if !desired.contains(&position) {
                self.chunks.remove(&position);
                self.dirty.remove(&position);
                delta.unloaded.push(position);
                self.mark_loaded_neighbors_dirty(position);
            }
        }

        for position in desired {
            if self.chunks.contains_key(&position) {
                continue;
            }

            let chunk = generate_chunk(position, self.seed);
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
        self.stream_center = None;
        removed
    }

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

        if !chunk.set(world_to_local(world), block) {
            return false;
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

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn solid_block_count(&self) -> usize {
        self.chunks.values().map(Chunk::solid_count).sum()
    }

    pub fn current_center(&self) -> Option<ChunkPos> {
        self.stream_center
    }

    pub fn take_dirty(&mut self) -> Vec<ChunkPos> {
        self.dirty.drain().collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_chunks_on_the_vertical_axis() {
        let mut world = VoxelWorld::new(123);
        let delta = world.stream_around(Vec3::new(0.0, 24.0, 0.0), 0, 2, true);

        assert_eq!(delta.loaded.len(), 5);
        assert!(delta.loaded.iter().any(|position| position.y == -1));
        assert!(delta.loaded.iter().any(|position| position.y == 3));
    }

    #[test]
    fn moving_vertically_changes_the_stream_center() {
        let mut world = VoxelWorld::new(123);
        world.stream_around(Vec3::new(0.0, 1.0, 0.0), 0, 0, true);
        let lower = world.current_center();

        world.stream_around(Vec3::new(0.0, 33.0, 0.0), 0, 0, false);
        let upper = world.current_center();

        assert_ne!(lower, upper);
        assert_eq!(upper, Some(ChunkPos::new(0, 2, 0)));
    }
}
