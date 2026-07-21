use glam::{IVec3, Vec3};

use crate::mesh::{MeshData, Vertex};

use super::{
    block::BlockId,
    chunk::{CHUNK_SIZE, Chunk, ChunkPos},
    world::VoxelWorld,
};

const MASK_SIZE: usize = (CHUNK_SIZE * CHUNK_SIZE) as usize;

#[derive(Clone, Copy)]
enum FaceDirection {
    PositiveX,
    NegativeX,
    PositiveY,
    NegativeY,
    PositiveZ,
    NegativeZ,
}

impl FaceDirection {
    const ALL: [Self; 6] = [
        Self::PositiveX,
        Self::NegativeX,
        Self::PositiveY,
        Self::NegativeY,
        Self::PositiveZ,
        Self::NegativeZ,
    ];

    const fn neighbor(self) -> IVec3 {
        match self {
            Self::PositiveX => IVec3::new(1, 0, 0),
            Self::NegativeX => IVec3::new(-1, 0, 0),
            Self::PositiveY => IVec3::new(0, 1, 0),
            Self::NegativeY => IVec3::new(0, -1, 0),
            Self::PositiveZ => IVec3::new(0, 0, 1),
            Self::NegativeZ => IVec3::new(0, 0, -1),
        }
    }

    const fn normal(self) -> Vec3 {
        match self {
            Self::PositiveX => Vec3::new(1.0, 0.0, 0.0),
            Self::NegativeX => Vec3::new(-1.0, 0.0, 0.0),
            Self::PositiveY => Vec3::new(0.0, 1.0, 0.0),
            Self::NegativeY => Vec3::new(0.0, -1.0, 0.0),
            Self::PositiveZ => Vec3::new(0.0, 0.0, 1.0),
            Self::NegativeZ => Vec3::new(0.0, 0.0, -1.0),
        }
    }

    const fn shade(self) -> f32 {
        match self {
            Self::PositiveX => 0.82,
            Self::NegativeX => 0.76,
            Self::PositiveY => 1.0,
            Self::NegativeY => 0.55,
            Self::PositiveZ => 0.90,
            Self::NegativeZ => 0.72,
        }
    }

    #[inline]
    fn local(self, slice: i32, u: i32, v: i32) -> IVec3 {
        match self {
            Self::PositiveX | Self::NegativeX => IVec3::new(slice, v, u),
            Self::PositiveY | Self::NegativeY => IVec3::new(u, slice, v),
            Self::PositiveZ | Self::NegativeZ => IVec3::new(u, v, slice),
        }
    }
}

/// Greedy meshing por chunk.
///
/// Faces coplanares adjacentes do mesmo bloco são combinadas em um único quad.
/// Em terreno plano isso reduz milhares de triângulos para poucas dezenas.
pub fn build_chunk_mesh(world: &VoxelWorld, chunk_position: ChunkPos) -> MeshData {
    let Some(chunk) = world.chunk(chunk_position) else {
        return MeshData::default();
    };

    if chunk.is_empty() {
        return MeshData::default();
    }

    let mut mesh = MeshData::with_capacity(1024, 1536);
    let origin = chunk_position.world_origin();
    let mut mask = [None; MASK_SIZE];

    for direction in FaceDirection::ALL {
        for slice in 0..CHUNK_SIZE {
            fill_mask(
                &mut mask,
                world,
                chunk,
                origin,
                direction,
                slice,
            );
            emit_greedy_slice(&mut mesh, &mut mask, origin, direction, slice);
        }
    }

    mesh
}

fn fill_mask(
    mask: &mut [Option<BlockId>; MASK_SIZE],
    world: &VoxelWorld,
    chunk: &Chunk,
    origin: IVec3,
    direction: FaceDirection,
    slice: i32,
) {
    mask.fill(None);

    for v in 0..CHUNK_SIZE {
        for u in 0..CHUNK_SIZE {
            let local = direction.local(slice, u, v);
            let block = chunk.get(local);
            if block.is_air() {
                continue;
            }

            let neighbor_local = local + direction.neighbor();
            let neighbor = if is_inside_chunk(neighbor_local) {
                chunk.get(neighbor_local)
            } else {
                world.get_block(origin + neighbor_local)
            };

            if neighbor.is_air() {
                mask[mask_index(u, v)] = Some(block);
            }
        }
    }
}

fn emit_greedy_slice(
    mesh: &mut MeshData,
    mask: &mut [Option<BlockId>; MASK_SIZE],
    origin: IVec3,
    direction: FaceDirection,
    slice: i32,
) {
    let mut v = 0;
    while v < CHUNK_SIZE {
        let mut u = 0;
        while u < CHUNK_SIZE {
            let index = mask_index(u, v);
            let Some(block) = mask[index] else {
                u += 1;
                continue;
            };

            let mut width = 1;
            while u + width < CHUNK_SIZE
                && mask[mask_index(u + width, v)] == Some(block)
            {
                width += 1;
            }

            let mut height = 1;
            'height: while v + height < CHUNK_SIZE {
                for offset in 0..width {
                    if mask[mask_index(u + offset, v + height)] != Some(block) {
                        break 'height;
                    }
                }
                height += 1;
            }

            for clear_v in 0..height {
                for clear_u in 0..width {
                    mask[mask_index(u + clear_u, v + clear_v)] = None;
                }
            }

            emit_quad(
                mesh,
                origin,
                direction,
                slice,
                u,
                v,
                width,
                height,
                block,
            );

            u += width;
        }
        v += 1;
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_quad(
    mesh: &mut MeshData,
    origin: IVec3,
    direction: FaceDirection,
    slice: i32,
    u: i32,
    v: i32,
    width: i32,
    height: i32,
    block: BlockId,
) {
    let origin = origin.as_vec3();
    let u0 = u as f32;
    let u1 = (u + width) as f32;
    let v0 = v as f32;
    let v1 = (v + height) as f32;
    let slice = slice as f32;

    let corners = match direction {
        FaceDirection::PositiveX => {
            let x = slice + 1.0;
            [
                Vec3::new(x, v0, u1),
                Vec3::new(x, v0, u0),
                Vec3::new(x, v1, u0),
                Vec3::new(x, v1, u1),
            ]
        }
        FaceDirection::NegativeX => {
            let x = slice;
            [
                Vec3::new(x, v0, u0),
                Vec3::new(x, v0, u1),
                Vec3::new(x, v1, u1),
                Vec3::new(x, v1, u0),
            ]
        }
        FaceDirection::PositiveY => {
            let y = slice + 1.0;
            [
                Vec3::new(u0, y, v1),
                Vec3::new(u1, y, v1),
                Vec3::new(u1, y, v0),
                Vec3::new(u0, y, v0),
            ]
        }
        FaceDirection::NegativeY => {
            let y = slice;
            [
                Vec3::new(u0, y, v0),
                Vec3::new(u1, y, v0),
                Vec3::new(u1, y, v1),
                Vec3::new(u0, y, v1),
            ]
        }
        FaceDirection::PositiveZ => {
            let z = slice + 1.0;
            [
                Vec3::new(u0, v0, z),
                Vec3::new(u1, v0, z),
                Vec3::new(u1, v1, z),
                Vec3::new(u0, v1, z),
            ]
        }
        FaceDirection::NegativeZ => {
            let z = slice;
            [
                Vec3::new(u1, v0, z),
                Vec3::new(u0, v0, z),
                Vec3::new(u0, v1, z),
                Vec3::new(u1, v1, z),
            ]
        }
    };

    let base = mesh.vertices.len() as u32;
    let color = block.color().map(|channel| channel * direction.shade());
    let normal = direction.normal().to_array();

    for corner in corners {
        mesh.vertices.push(Vertex {
            position: (origin + corner).to_array(),
            normal,
            color,
        });
    }

    mesh.indices.extend_from_slice(&[
        base,
        base + 1,
        base + 2,
        base,
        base + 2,
        base + 3,
    ]);
}

#[inline]
fn is_inside_chunk(local: IVec3) -> bool {
    (0..CHUNK_SIZE).contains(&local.x)
        && (0..CHUNK_SIZE).contains(&local.y)
        && (0..CHUNK_SIZE).contains(&local.z)
}

#[inline]
fn mask_index(u: i32, v: i32) -> usize {
    (u + v * CHUNK_SIZE) as usize
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;
    use crate::voxel::VoxelWorld;

    #[test]
    fn generated_chunk_can_be_meshed() {
        let mut world = VoxelWorld::new(7);
        world.stream_around(
            Vec3::new(0.0, 24.0, 0.0),
            0,
            0,
            &std::collections::HashSet::new(),
            true,
            usize::MAX,
        );
        let position = world.current_center().unwrap();
        let mesh = build_chunk_mesh(&world, position);
        assert_eq!(mesh.indices.len() % 3, 0);
    }
}
