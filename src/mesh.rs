use std::mem;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x3, // position
        1 => Float32x3 // color
    ];

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub const CUBE_VERTICES: &[Vertex] = &[
    // Frente
    Vertex {
        position: [-0.5, -0.5, 0.5],
        color: [0.3, 0.8, 0.3],
    },
    Vertex {
        position: [0.5, -0.5, 0.5],
        color: [0.3, 0.8, 0.3],
    },
    Vertex {
        position: [0.5, 0.5, 0.5],
        color: [0.4, 1.0, 0.4],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5],
        color: [0.4, 1.0, 0.4],
    },
    // Trás
    Vertex {
        position: [0.5, -0.5, -0.5],
        color: [0.2, 0.6, 0.2],
    },
    Vertex {
        position: [-0.5, -0.5, -0.5],
        color: [0.2, 0.6, 0.2],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5],
        color: [0.3, 0.7, 0.3],
    },
    Vertex {
        position: [0.5, 0.5, -0.5],
        color: [0.3, 0.7, 0.3],
    },
    // Esquerda
    Vertex {
        position: [-0.5, -0.5, -0.5],
        color: [0.25, 0.65, 0.25],
    },
    Vertex {
        position: [-0.5, -0.5, 0.5],
        color: [0.25, 0.65, 0.25],
    },
    Vertex {
        position: [-0.5, 0.5, 0.5],
        color: [0.35, 0.75, 0.35],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5],
        color: [0.35, 0.75, 0.35],
    },
    // Direita
    Vertex {
        position: [0.5, -0.5, 0.5],
        color: [0.2, 0.55, 0.2],
    },
    Vertex {
        position: [0.5, -0.5, -0.5],
        color: [0.2, 0.55, 0.2],
    },
    Vertex {
        position: [0.5, 0.5, -0.5],
        color: [0.3, 0.7, 0.3],
    },
    Vertex {
        position: [0.5, 0.5, 0.5],
        color: [0.3, 0.7, 0.3],
    },
    // Cima
    Vertex {
        position: [-0.5, 0.5, 0.5],
        color: [0.5, 1.0, 0.5],
    },
    Vertex {
        position: [0.5, 0.5, 0.5],
        color: [0.5, 1.0, 0.5],
    },
    Vertex {
        position: [0.5, 0.5, -0.5],
        color: [0.45, 0.9, 0.45],
    },
    Vertex {
        position: [-0.5, 0.5, -0.5],
        color: [0.45, 0.9, 0.45],
    },
    // Baixo
    Vertex {
        position: [-0.5, -0.5, -0.5],
        color: [0.15, 0.4, 0.15],
    },
    Vertex {
        position: [0.5, -0.5, -0.5],
        color: [0.15, 0.4, 0.15],
    },
    Vertex {
        position: [0.5, -0.5, 0.5],
        color: [0.2, 0.5, 0.2],
    },
    Vertex {
        position: [-0.5, -0.5, 0.5],
        color: [0.2, 0.5, 0.2],
    },
];


pub const CUBE_INDICES: &[u16] = &[
    // Frente
    0, 1, 2,
    0, 2, 3,

    // Trás
    4, 5, 6,
    4, 6, 7,

    // Esquerda
    8, 9, 10,
    8, 10, 11,

    // Direita
    12, 13, 14,
    12, 14, 15,

    // Cima
    16, 17, 18,
    16, 18, 19,

    // Baixo
    20, 21, 22,
    20, 22, 23,
];