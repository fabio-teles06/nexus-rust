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
}

impl Chunk {
    pub fn empty() -> Self {
        Self {
            blocks: vec![AIR; CHUNK_VOLUME],
        }
    }

    pub fn get(&self, local: IVec3) -> BlockId {
        self.blocks[index(local)]
    }

    pub fn set(&mut self, local: IVec3, block: BlockId) -> bool {
        let index = index(local);
        if self.blocks[index] == block {
            return false;
        }
        self.blocks[index] = block;
        true
    }

    pub fn solid_count(&self) -> usize {
        self.blocks.iter().filter(|block| block.is_solid()).count()
    }
}

pub fn world_to_local(block: IVec3) -> IVec3 {
    IVec3::new(
        block.x.rem_euclid(CHUNK_SIZE),
        block.y.rem_euclid(CHUNK_SIZE),
        block.z.rem_euclid(CHUNK_SIZE),
    )
}

fn index(local: IVec3) -> usize {
    debug_assert!((0..CHUNK_SIZE).contains(&local.x));
    debug_assert!((0..CHUNK_SIZE).contains(&local.y));
    debug_assert!((0..CHUNK_SIZE).contains(&local.z));

    (local.x + local.z * CHUNK_SIZE + local.y * CHUNK_SIZE * CHUNK_SIZE) as usize
}


#[cfg(test)]
mod tests {
    use super::*;

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
}
