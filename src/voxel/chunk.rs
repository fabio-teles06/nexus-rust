use glam::{IVec3, Vec3};

use super::block::{AIR, BlockId};

pub const CHUNK_SIZE: i32 = 16;
pub const CHUNK_VOLUME: usize = (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChunkPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl ChunkPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub fn from_world_position(position: Vec3) -> Self {
        Self::from_world_block(position.floor().as_ivec3())
    }

    pub fn from_world_block(block: IVec3) -> Self {
        Self {
            x: block.x.div_euclid(CHUNK_SIZE),
            y: block.y.div_euclid(CHUNK_SIZE),
            z: block.z.div_euclid(CHUNK_SIZE),
        }
    }

    pub fn world_origin(self) -> IVec3 {
        IVec3::new(
            self.x * CHUNK_SIZE,
            self.y * CHUNK_SIZE,
            self.z * CHUNK_SIZE,
        )
    }

    pub fn neighbors(self) -> [Self; 6] {
        [
            Self::new(self.x + 1, self.y, self.z),
            Self::new(self.x - 1, self.y, self.z),
            Self::new(self.x, self.y + 1, self.z),
            Self::new(self.x, self.y - 1, self.z),
            Self::new(self.x, self.y, self.z + 1),
            Self::new(self.x, self.y, self.z - 1),
        ]
    }
}

#[derive(Debug, Clone)]
pub struct Chunk {
    blocks: Vec<BlockId>,
    solid_count: usize,
}

impl Chunk {
    pub fn empty() -> Self {
        Self {
            blocks: vec![AIR; CHUNK_VOLUME],
            solid_count: 0,
        }
    }

    pub fn filled(block: BlockId) -> Self {
        Self {
            blocks: vec![block; CHUNK_VOLUME],
            solid_count: if block.is_solid() { CHUNK_VOLUME } else { 0 },
        }
    }

    #[inline]
    pub fn get(&self, local: IVec3) -> BlockId {
        self.blocks[index(local)]
    }

    #[inline]
    pub fn set(&mut self, local: IVec3, block: BlockId) -> bool {
        let index = index(local);
        let previous = self.blocks[index];
        if previous == block {
            return false;
        }

        if previous.is_solid() {
            self.solid_count -= 1;
        }
        if block.is_solid() {
            self.solid_count += 1;
        }

        self.blocks[index] = block;
        true
    }

    #[inline]
    pub const fn solid_count(&self) -> usize {
        self.solid_count
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.solid_count == 0
    }

    #[inline]
    pub const fn is_full(&self) -> bool {
        self.solid_count == CHUNK_VOLUME
    }
}

#[inline]
pub fn world_to_local(block: IVec3) -> IVec3 {
    IVec3::new(
        block.x.rem_euclid(CHUNK_SIZE),
        block.y.rem_euclid(CHUNK_SIZE),
        block.z.rem_euclid(CHUNK_SIZE),
    )
}

#[inline]
fn index(local: IVec3) -> usize {
    debug_assert!((0..CHUNK_SIZE).contains(&local.x));
    debug_assert!((0..CHUNK_SIZE).contains(&local.y));
    debug_assert!((0..CHUNK_SIZE).contains(&local.z));

    (local.x + local.z * CHUNK_SIZE + local.y * CHUNK_SIZE * CHUNK_SIZE) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::STONE;

    #[test]
    fn converts_negative_world_coordinates() {
        let block = IVec3::new(-1, -17, -16);
        assert_eq!(ChunkPos::from_world_block(block), ChunkPos::new(-1, -2, -1));
        assert_eq!(world_to_local(block), IVec3::new(15, 15, 0));
    }

    #[test]
    fn vertical_chunks_have_distinct_origins() {
        assert_eq!(ChunkPos::new(0, 2, 0).world_origin(), IVec3::new(0, 32, 0));
        assert_eq!(ChunkPos::new(0, -2, 0).world_origin(), IVec3::new(0, -32, 0));
    }

    #[test]
    fn caches_solid_count() {
        let mut chunk = Chunk::empty();
        assert_eq!(chunk.solid_count(), 0);
        chunk.set(IVec3::ZERO, STONE);
        assert_eq!(chunk.solid_count(), 1);
        chunk.set(IVec3::ZERO, AIR);
        assert_eq!(chunk.solid_count(), 0);
    }
}
