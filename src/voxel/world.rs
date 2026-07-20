use super::block::{AIR, BlockId, DIRT, GRASS, STONE};

#[derive(Debug)]
pub struct World {
    width: i32,
    height: i32,
    depth: i32,
    blocks: Vec<BlockId>,
}

impl World {
    pub fn new(width: i32, height: i32, depth: i32) -> Self {
        assert!(width > 0);
        assert!(height > 0);
        assert!(depth > 0);

        let block_count = width as usize * height as usize * depth as usize;

        Self {
            width,
            height,
            depth,
            blocks: vec![AIR; block_count],
        }
    }

    pub fn demo() -> Self {
        let mut world = Self::new(32, 16, 32);

        for z in 0..world.depth {
            for x in 0..world.width {
                let variation = ((x * 13 + z * 7) % 3).abs();

                let surface_y = 3 + variation;

                for y in 0..=surface_y {
                    let block_id = if y == surface_y {
                        GRASS
                    } else if y >= surface_y - 2 {
                        DIRT
                    } else {
                        STONE
                    };

                    world.set(x, y, z, block_id);
                }
            }
        }

        world
    }

    pub fn size(&self) -> [i32; 3] {
        [self.width, self.height, self.depth]
    }

    pub fn contains(&self, x: i32, y: i32, z: i32) -> bool {
        x >= 0 && x < self.width && y >= 0 && y < self.height && z >= 0 && z < self.depth
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        let Some(index) = self.index(x, y, z) else {
            return AIR;
        };

        self.blocks[index]
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, block: BlockId) -> bool {
        let Some(index) = self.index(x, y, z) else {
            return false;
        };

        if self.blocks[index] == block {
            return false;
        }

        self.blocks[index] = block;
        true
    }

    fn index(&self, x: i32, y: i32, z: i32) -> Option<usize> {
        if !self.contains(x, y, z) {
            return None;
        }

        let index = (y * self.width * self.depth + z * self.width + x) as usize;
        Some(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_empty_world() {
        let world = World::new(4, 4, 4);

        assert_eq!(world.get(0, 0, 0), AIR);
        assert_eq!(world.get(3, 3, 3), AIR);
    }

    #[test]
    fn can_change_block() {
        let mut world = World::new(4, 4, 4);

        assert!(world.set(2, 1, 3, STONE));
        assert_eq!(world.get(2, 1, 3), STONE);
    }

    #[test]
    fn rejects_out_of_bounds() {
        let mut world = World::new(4, 4, 4);

        assert!(!world.set(-1, 0, 0, STONE));
        assert!(!world.set(0, -1, 0, STONE));
        assert!(!world.set(0, 0, -1, STONE));
        assert!(!world.set(4, 0, 0, STONE));
        assert!(!world.set(0, 4, 0, STONE));
        assert!(!world.set(0, 0, 4, STONE));

        assert_eq!(world.get(-1, 0, 0), AIR);
        assert_eq!(world.get(0, -1, 0), AIR);
        assert_eq!(world.get(0, 0, -1), AIR);
        assert_eq!(world.get(4, 0, 0), AIR);
        assert_eq!(world.get(0, 4, 0), AIR);
        assert_eq!(world.get(0, 0, 4), AIR);
    }
}
