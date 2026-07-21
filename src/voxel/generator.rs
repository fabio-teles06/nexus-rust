use glam::IVec3;

use super::{
    block::{BEDROCK, DIRT, GRASS, STONE},
    chunk::{CHUNK_SIZE, Chunk, ChunkPos},
};

pub fn generate_chunk(position: ChunkPos, seed: u32) -> Chunk {
    let mut chunk = Chunk::empty();
    let origin = position.world_origin();

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let local = IVec3::new(x, y, z);
                let world = origin + local;
                let height = terrain_height(world.x, world.z, seed);

                let block = if world.y < -32 {
                    continue;
                } else if world.y == -32 {
                    BEDROCK
                } else if world.y > height {
                    continue;
                } else if world.y == height {
                    GRASS
                } else if world.y >= height - 3 {
                    DIRT
                } else {
                    STONE
                };

                chunk.set(local, block);
            }
        }
    }

    chunk
}

fn terrain_height(x: i32, z: i32, seed: u32) -> i32 {
    let seed_offset = seed as f32 * 0.001;
    let x = x as f32;
    let z = z as f32;

    let broad = (x * 0.024 + seed_offset).sin() * 8.0
        + (z * 0.020 - seed_offset).cos() * 7.0;
    let detail = ((x + z) * 0.055).sin() * 3.0
        + ((x - z) * 0.041).cos() * 2.0;

    (24.0 + broad + detail).round() as i32
}
