use glam::{Mat4, Vec3, Vec4};

use crate::voxel::{CHUNK_SIZE, ChunkPos};

#[derive(Debug, Clone, Copy)]
struct Plane {
    normal: Vec3,
    distance: f32,
}

impl Plane {
    fn from_coefficients(coefficients: Vec4) -> Self {
        let normal = coefficients.truncate();
        let length = normal.length();

        if length <= f32::EPSILON {
            return Self {
                normal: Vec3::ZERO,
                distance: coefficients.w,
            };
        }

        Self {
            normal: normal / length,
            distance: coefficients.w / length,
        }
    }

    fn aabb_is_outside(self, min: Vec3, max: Vec3) -> bool {
        // Vértice do AABB mais distante na direção da normal. Se até este
        // vértice estiver fora, todo o AABB está fora do plano.
        let positive = Vec3::new(
            if self.normal.x >= 0.0 { max.x } else { min.x },
            if self.normal.y >= 0.0 { max.y } else { min.y },
            if self.normal.z >= 0.0 { max.z } else { min.z },
        );

        self.normal.dot(positive) + self.distance < 0.0
    }
}

/// Frustum no padrão DirectX/WebGPU: X e Y em `[-W, W]` e Z em `[0, W]`.
#[derive(Debug, Clone, Copy)]
pub struct Frustum {
    planes: [Plane; 6],
}

impl Frustum {
    pub fn from_view_projection(matrix: Mat4) -> Self {
        let columns = matrix.to_cols_array_2d();
        let row0 = row(&columns, 0);
        let row1 = row(&columns, 1);
        let row2 = row(&columns, 2);
        let row3 = row(&columns, 3);

        Self {
            planes: [
                Plane::from_coefficients(row3 + row0), // esquerda
                Plane::from_coefficients(row3 - row0), // direita
                Plane::from_coefficients(row3 + row1), // baixo
                Plane::from_coefficients(row3 - row1), // cima
                Plane::from_coefficients(row2),        // near: z >= 0
                Plane::from_coefficients(row3 - row2), // far: z <= w
            ],
        }
    }

    pub fn intersects_chunk(self, position: ChunkPos) -> bool {
        let min = position.world_origin().as_vec3();
        let max = min + Vec3::splat(CHUNK_SIZE as f32);
        self.intersects_aabb(min, max)
    }

    pub fn intersects_aabb(self, min: Vec3, max: Vec3) -> bool {
        self.planes
            .iter()
            .all(|plane| !plane.aabb_is_outside(min, max))
    }
}

fn row(columns: &[[f32; 4]; 4], index: usize) -> Vec4 {
    Vec4::new(
        columns[0][index],
        columns[1][index],
        columns[2][index],
        columns[3][index],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_frustum_uses_webgpu_depth_range() {
        let frustum = Frustum::from_view_projection(Mat4::IDENTITY);

        assert!(frustum.intersects_aabb(
            Vec3::new(-0.5, -0.5, 0.1),
            Vec3::new(0.5, 0.5, 0.9),
        ));
        assert!(!frustum.intersects_aabb(
            Vec3::new(-0.5, -0.5, -2.0),
            Vec3::new(0.5, 0.5, -1.0),
        ));
        assert!(!frustum.intersects_aabb(
            Vec3::new(2.0, -0.5, 0.1),
            Vec3::new(3.0, 0.5, 0.9),
        ));
    }
}
