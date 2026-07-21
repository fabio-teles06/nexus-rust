use glam::IVec3;

use super::{
    block::{BEDROCK, DIRT, GRASS, STONE},
    chunk::{CHUNK_SIZE, Chunk, ChunkPos},
};

const BEDROCK_Y: i32 = -32;
const MIN_TERRAIN_HEIGHT: i32 = 4;
const MAX_TERRAIN_HEIGHT: i32 = 44;

pub fn generate_chunk(position: ChunkPos, seed: u32) -> Chunk {
    let origin = position.world_origin();
    let top = origin.y + CHUNK_SIZE - 1;

    // Chunks completamente acima do terreno ou abaixo da camada de bedrock
    // não precisam executar o gerador bloco a bloco.
    if origin.y > MAX_TERRAIN_HEIGHT || top < BEDROCK_Y {
        return Chunk::empty();
    }

    // Entre a bedrock e a menor altura possível do terreno, todo o chunk é pedra.
    if origin.y > BEDROCK_Y && top < MIN_TERRAIN_HEIGHT {
        return Chunk::filled(STONE);
    }

    let mut chunk = Chunk::empty();

    // A altura depende apenas de X/Z. Calculá-la uma vez por coluna reduz as
    // chamadas trigonométricas de 4096 para 256 por chunk.
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let world_x = origin.x + x;
            let world_z = origin.z + z;
            let height = terrain_height(world_x, world_z, seed);

            let local_min_y = (BEDROCK_Y - origin.y).max(0);
            let local_max_y = (height - origin.y).min(CHUNK_SIZE - 1);
            if local_min_y > local_max_y {
                continue;
            }

            for y in local_min_y..=local_max_y {
                let world_y = origin.y + y;
                let block = if world_y == BEDROCK_Y {
                    BEDROCK
                } else if world_y == height {
                    GRASS
                } else if world_y >= height - 3 {
                    DIRT
                } else {
                    STONE
                };

                chunk.set(IVec3::new(x, y, z), block);
            }
        }
    }

    chunk
}

#[inline]
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
