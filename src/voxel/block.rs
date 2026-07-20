#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BlockId(pub u16);

pub const AIR: BlockId = BlockId(0);
pub const STONE: BlockId = BlockId(1);
pub const DIRT: BlockId = BlockId(2);
pub const GRASS: BlockId = BlockId(3);

impl BlockId {
    pub const fn is_air(self) -> bool {
        self.0 == AIR.0
    }

    pub const fn is_solid(self) -> bool {
        !self.is_air()
    }

    pub const fn name(self) -> &'static str {
        match self.0 {
            0 => "Air",
            1 => "Stone",
            2 => "Dirt",
            3 => "Grass",
            _ => "Unknown",
        }
    }

    pub const fn color(self) -> [f32; 3] {
        match self.0 {
            // Ar não será renderizado.
            0 => [0.0, 0.0, 0.0],

            // Pedra
            1 => [0.48, 0.50, 0.54],

            // Terra
            2 => [0.45, 0.28, 0.14],

            // Grama
            3 => [0.25, 0.68, 0.24],

            // Bloco desconhecido: magenta para evidenciar o erro.
            _ => [1.0, 0.0, 1.0],
        }
    }
}
