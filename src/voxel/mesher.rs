use glam::{IVec3, Vec3};

use crate::mesh::{MeshData, Vertex};

use super::{chunk::{CHUNK_SIZE, ChunkPos}, world::VoxelWorld};

#[derive(Clone, Copy)]
struct Face {
    neighbor: IVec3,
    normal: Vec3,
    corners: [Vec3; 4],
    shade: f32,
}

const FACES: [Face; 6] = [
    Face {
        neighbor: IVec3::new(0, 0, 1),
        normal: Vec3::new(0.0, 0.0, 1.0),
        corners: [Vec3::new(0.0, 0.0, 1.0), Vec3::new(1.0, 0.0, 1.0), Vec3::new(1.0, 1.0, 1.0), Vec3::new(0.0, 1.0, 1.0)],
        shade: 0.90,
    },
    Face {
        neighbor: IVec3::new(0, 0, -1),
        normal: Vec3::new(0.0, 0.0, -1.0),
        corners: [Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), Vec3::new(1.0, 1.0, 0.0)],
        shade: 0.72,
    },
    Face {
        neighbor: IVec3::new(1, 0, 0),
        normal: Vec3::new(1.0, 0.0, 0.0),
        corners: [Vec3::new(1.0, 0.0, 1.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0), Vec3::new(1.0, 1.0, 1.0)],
        shade: 0.82,
    },
    Face {
        neighbor: IVec3::new(-1, 0, 0),
        normal: Vec3::new(-1.0, 0.0, 0.0),
        corners: [Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0), Vec3::new(0.0, 1.0, 1.0), Vec3::new(0.0, 1.0, 0.0)],
        shade: 0.76,
    },
    Face {
        neighbor: IVec3::new(0, 1, 0),
        normal: Vec3::new(0.0, 1.0, 0.0),
        corners: [Vec3::new(0.0, 1.0, 1.0), Vec3::new(1.0, 1.0, 1.0), Vec3::new(1.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 0.0)],
        shade: 1.0,
    },
    Face {
        neighbor: IVec3::new(0, -1, 0),
        normal: Vec3::new(0.0, -1.0, 0.0),
        corners: [Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 1.0), Vec3::new(0.0, 0.0, 1.0)],
        shade: 0.55,
    },
];

pub fn build_chunk_mesh(world: &VoxelWorld, chunk_position: ChunkPos) -> MeshData {
    let Some(chunk) = world.chunk(chunk_position) else {
        return MeshData::default();
    };

    if chunk.solid_count() == 0 {
        return MeshData::default();
    }

    let mut mesh = MeshData::default();
    let origin = chunk_position.world_origin();

    for y in 0..CHUNK_SIZE {
        for z in 0..CHUNK_SIZE {
            for x in 0..CHUNK_SIZE {
                let local = IVec3::new(x, y, z);
                let block = chunk.get(local);
                if block.is_air() {
                    continue;
                }

                let world_block = origin + local;
                for face in FACES {
                    if world.get_block(world_block + face.neighbor).is_solid() {
                        continue;
                    }

                    let base = mesh.vertices.len() as u32;
                    let color = block.color().map(|channel| channel * face.shade);
                    let world_base = world_block.as_vec3();

                    for corner in face.corners {
                        mesh.vertices.push(Vertex {
                            position: (world_base + corner).to_array(),
                            normal: face.normal.to_array(),
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
            }
        }
    }

    mesh
}

#[cfg(test)]
mod tests {
    use glam::Vec3;

    use super::*;
    use crate::voxel::VoxelWorld;

    #[test]
    fn generated_chunk_can_be_meshed() {
        let mut world = VoxelWorld::new(7);
        world.stream_around(Vec3::new(0.0, 24.0, 0.0), 0, 0, true);
        let position = world.current_center().unwrap();
        let mesh = build_chunk_mesh(&world, position);
        assert!(mesh.indices.len() % 3 == 0);
    }
}
