use crate::mesh::Vertex;

use super::world::World;

#[derive(Debug, Default)]
pub struct VoxelMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl VoxelMesh {
    pub fn face_count(&self) -> usize {
        self.indices.len() / 6
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty()
    }
}

#[derive(Debug, Clone, Copy)]
struct Face {
    /// Posição do bloco vizinho.
    neighbor: [i32; 3],

    /// Quatro cantos da face em coordenadas locais.
    corners: [[f32; 3]; 4],

    /// Escurecimento simples para distinguir as faces.
    shade: f32,
}

const FACES: [Face; 6] = [
    // Frente: +Z
    Face {
        neighbor: [0, 0, 1],
        corners: [
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 1.0],
            [0.0, 1.0, 1.0],
        ],
        shade: 0.90,
    },

    // Trás: -Z
    Face {
        neighbor: [0, 0, -1],
        corners: [
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ],
        shade: 0.72,
    },

    // Direita: +X
    Face {
        neighbor: [1, 0, 0],
        corners: [
            [1.0, 0.0, 1.0],
            [1.0, 0.0, 0.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 1.0],
        ],
        shade: 0.82,
    },

    // Esquerda: -X
    Face {
        neighbor: [-1, 0, 0],
        corners: [
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 1.0, 1.0],
            [0.0, 1.0, 0.0],
        ],
        shade: 0.76,
    },

    // Cima: +Y
    Face {
        neighbor: [0, 1, 0],
        corners: [
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
        ],
        shade: 1.0,
    },

    // Baixo: -Y
    Face {
        neighbor: [0, -1, 0],
        corners: [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        shade: 0.55,
    },
];

pub fn build_world_mesh(world: &World) -> VoxelMesh {
    let mut mesh = VoxelMesh::default();

    let [width, height, depth] = world.size();

    /*
     * Centraliza o mundo nos eixos X e Z.
     *
     * Um mundo com 32 blocos ficará aproximadamente entre:
     *
     * X: -16 até +16
     * Z: -16 até +16
     */
    let world_origin_x = -(width as f32) * 0.5;
    let world_origin_z = -(depth as f32) * 0.5;

    for y in 0..height {
        for z in 0..depth {
            for x in 0..width {
                let block = world.get(x, y, z);

                if !block.is_solid() {
                    continue;
                }

                let base_color = block.color();

                for face in FACES {
                    let neighbor_x =
                        x + face.neighbor[0];

                    let neighbor_y =
                        y + face.neighbor[1];

                    let neighbor_z =
                        z + face.neighbor[2];

                    let neighbor = world.get(
                        neighbor_x,
                        neighbor_y,
                        neighbor_z,
                    );

                    /*
                     * Se o bloco vizinho for sólido,
                     * a face está escondida.
                     */
                    if neighbor.is_solid() {
                        continue;
                    }

                    add_face(
                        &mut mesh,
                        [
                            world_origin_x + x as f32,
                            y as f32,
                            world_origin_z + z as f32,
                        ],
                        face,
                        shade_color(
                            base_color,
                            face.shade,
                        ),
                    );
                }
            }
        }
    }

    mesh
}

fn add_face(
    mesh: &mut VoxelMesh,
    block_position: [f32; 3],
    face: Face,
    color: [f32; 3],
) {
    let first_vertex =
        mesh.vertices.len() as u32;

    for corner in face.corners {
        mesh.vertices.push(Vertex {
            position: [
                block_position[0] + corner[0],
                block_position[1] + corner[1],
                block_position[2] + corner[2],
            ],
            color,
        });
    }

    /*
     * Dois triângulos:
     *
     * 0 ─── 3
     * │   ╱ │
     * │ ╱   │
     * 1 ─── 2
     */
    mesh.indices.extend_from_slice(&[
        first_vertex,
        first_vertex + 1,
        first_vertex + 2,

        first_vertex,
        first_vertex + 2,
        first_vertex + 3,
    ]);
}

fn shade_color(
    color: [f32; 3],
    shade: f32,
) -> [f32; 3] {
    [
        color[0] * shade,
        color[1] * shade,
        color[2] * shade,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::{
        block::STONE,
        world::World,
    };

    #[test]
    fn one_block_has_six_faces() {
        let mut world = World::new(1, 1, 1);
        world.set(0, 0, 0, STONE);

        let mesh = build_world_mesh(&world);

        assert_eq!(mesh.face_count(), 6);
        assert_eq!(mesh.vertices.len(), 24);
        assert_eq!(mesh.indices.len(), 36);
    }

    #[test]
    fn adjacent_blocks_hide_internal_faces() {
        let mut world = World::new(2, 1, 1);

        world.set(0, 0, 0, STONE);
        world.set(1, 0, 0, STONE);

        let mesh = build_world_mesh(&world);

        /*
         * Dois cubos teriam 12 faces.
         * As duas faces internas são removidas.
         */
        assert_eq!(mesh.face_count(), 10);
        assert_eq!(mesh.vertices.len(), 40);
        assert_eq!(mesh.indices.len(), 60);
    }

    #[test]
    fn empty_world_has_empty_mesh() {
        let world = World::new(4, 4, 4);

        let mesh = build_world_mesh(&world);

        assert!(mesh.is_empty());
        assert!(mesh.indices.is_empty());
    }
}