#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct BlockId(pub u16);

pub const AIR: BlockId = BlockId(0);
pub const STONE: BlockId = BlockId(1);
pub const DIRT: BlockId = BlockId(2);
pub const GRASS: BlockId = BlockId(3);
pub const BEDROCK: BlockId = BlockId(4);

impl BlockId {
    pub const fn is_air(self) -> bool {
        self.0 == AIR.0
    }

    pub const fn is_solid(self) -> bool {
        !self.is_air()
    }

    pub const fn color(self) -> [f32; 3] {
        match self.0 {
            1 => [0.48, 0.50, 0.54],
            2 => [0.42, 0.25, 0.12],
            3 => [0.24, 0.67, 0.22],
            4 => [0.18, 0.18, 0.22],
            _ => [1.0, 0.0, 1.0],
        }
    }
}
