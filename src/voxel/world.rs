use std::collections::{HashMap, HashSet};

use glam::{IVec3, Vec3};
use rayon::prelude::*;

use super::{
    block::{AIR, BlockId},
    chunk::{Chunk, ChunkPos, world_to_local},
    generator::generate_chunk,
};

#[derive(Debug, Default)]
pub struct StreamDelta {
    /// Chunks que passaram a existir na memória nesta atualização.
    pub loaded: Vec<ChunkPos>,
    /// Chunks que deixaram de ser necessários tanto para render quanto para física.
    pub unloaded: Vec<ChunkPos>,
    /// Chunks que entraram no raio visual da câmera.
    pub render_added: Vec<ChunkPos>,
    /// Chunks que saíram do raio visual, mas podem continuar carregados para física.
    pub render_removed: Vec<ChunkPos>,
    /// Chunks que passaram a precisar de collider.
    pub physics_added: Vec<ChunkPos>,
    /// Chunks cujo collider pode ser removido.
    pub physics_removed: Vec<ChunkPos>,
}

pub struct VoxelWorld {
    chunks: HashMap<ChunkPos, Chunk>,
    dirty: HashSet<ChunkPos>,
    render_chunks: HashSet<ChunkPos>,
    physics_keepalive: HashSet<ChunkPos>,
    stream_center: Option<ChunkPos>,
    seed: u32,
}

impl VoxelWorld {
    pub fn new(seed: u32) -> Self {
        Self {
            chunks: HashMap::new(),
            dirty: HashSet::new(),
            render_chunks: HashSet::new(),
            physics_keepalive: HashSet::new(),
            stream_center: None,
            seed,
        }
    }

    /// Atualiza o streaming a partir da câmera e dos chunks protegidos pela física.
    ///
    /// A geração dos chunks ausentes é feita em paralelo com Rayon. A função só
    /// retorna quando o lote está pronto, evitando que a física avance sobre um
    /// chunk ainda sem collider.
    pub fn stream_around(
        &mut self,
        world_position: Vec3,
        horizontal_radius: i32,
        vertical_radius: i32,
        physics_keepalive: &HashSet<ChunkPos>,
        force: bool,
    ) -> StreamDelta {
        let center = ChunkPos::from_world_position(world_position);

        if !force
            && self.stream_center == Some(center)
            && self.physics_keepalive.eq(physics_keepalive)
        {
            return StreamDelta::default();
        }

        self.stream_center = Some(center);

        let render_desired = desired_render_chunks(
            center,
            horizontal_radius.max(0),
            vertical_radius.max(0),
        );

        let mut delta = StreamDelta {
            render_added: render_desired
                .difference(&self.render_chunks)
                .copied()
                .collect(),
            render_removed: self
                .render_chunks
                .difference(&render_desired)
                .copied()
                .collect(),
            physics_added: physics_keepalive
                .difference(&self.physics_keepalive)
                .copied()
                .collect(),
            physics_removed: self
                .physics_keepalive
                .difference(physics_keepalive)
                .copied()
                .collect(),
            ..StreamDelta::default()
        };

        self.render_chunks = render_desired;
        self.physics_keepalive = physics_keepalive.clone();

        let mut loaded_desired = self.render_chunks.clone();
        loaded_desired.extend(self.physics_keepalive.iter().copied());

        let existing = self.chunks.keys().copied().collect::<Vec<_>>();
        for position in existing {
            if loaded_desired.contains(&position) {
                continue;
            }

            self.chunks.remove(&position);
            self.dirty.remove(&position);
            delta.unloaded.push(position);
            self.mark_loaded_neighbors_dirty(position);
        }

        let mut missing = loaded_desired
            .into_iter()
            .filter(|position| !self.chunks.contains_key(position))
            .collect::<Vec<_>>();

        // A ordenação deixa a aplicação dos resultados determinística e prioriza
        // os chunks próximos da câmera, mesmo que a geração ocorra em paralelo.
        missing.sort_unstable_by_key(|position| chunk_distance_squared(*position, center));

        let seed = self.seed;
        let generated = missing
            .into_par_iter()
            .map(|position| (position, generate_chunk(position, seed)))
            .collect::<Vec<_>>();

        for (position, chunk) in generated {
            self.chunks.insert(position, chunk);
            delta.loaded.push(position);
            self.mark_dirty_with_neighbors(position);
        }

        delta
    }

    pub fn clear(&mut self) -> Vec<ChunkPos> {
        let removed = self.chunks.keys().copied().collect();
        self.chunks.clear();
        self.dirty.clear();
        self.render_chunks.clear();
        self.physics_keepalive.clear();
        self.stream_center = None;
        removed
    }

    pub fn get_block(&self, world: IVec3) -> BlockId {
        let chunk_position = ChunkPos::from_world_block(world);
        let Some(chunk) = self.chunks.get(&chunk_position) else {
            return AIR;
        };
        chunk.get(world_to_local(world))
    }

    #[allow(dead_code)]
    pub fn set_block(&mut self, world: IVec3, block: BlockId) -> bool {
        let chunk_position = ChunkPos::from_world_block(world);
        let Some(chunk) = self.chunks.get_mut(&chunk_position) else {
            return false;
        };

        if !chunk.set(world_to_local(world), block) {
            return false;
        }

        self.mark_dirty_with_neighbors(chunk_position);
        true
    }

    pub fn chunk(&self, position: ChunkPos) -> Option<&Chunk> {
        self.chunks.get(&position)
    }

    pub fn contains_chunk(&self, position: ChunkPos) -> bool {
        self.chunks.contains_key(&position)
    }

    pub fn is_render_visible(&self, position: ChunkPos) -> bool {
        self.render_chunks.contains(&position)
    }

    pub fn is_physics_active(&self, position: ChunkPos) -> bool {
        self.physics_keepalive.contains(&position)
    }

    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    pub fn render_chunk_count(&self) -> usize {
        self.render_chunks.len()
    }

    pub fn physics_keepalive_count(&self) -> usize {
        self.physics_keepalive.len()
    }

    pub fn solid_block_count(&self) -> usize {
        self.chunks.values().map(Chunk::solid_count).sum()
    }

    pub fn current_center(&self) -> Option<ChunkPos> {
        self.stream_center
    }

    pub fn take_dirty(&mut self) -> Vec<ChunkPos> {
        self.dirty.drain().collect()
    }

    fn mark_dirty_with_neighbors(&mut self, position: ChunkPos) {
        if self.chunks.contains_key(&position) {
            self.dirty.insert(position);
        }
        self.mark_loaded_neighbors_dirty(position);
    }

    fn mark_loaded_neighbors_dirty(&mut self, position: ChunkPos) {
        for neighbor in position.neighbors() {
            if self.chunks.contains_key(&neighbor) {
                self.dirty.insert(neighbor);
            }
        }
    }
}

fn desired_render_chunks(
    center: ChunkPos,
    horizontal_radius: i32,
    vertical_radius: i32,
) -> HashSet<ChunkPos> {
    let mut desired = HashSet::new();
    let radius_squared = horizontal_radius * horizontal_radius;

    for y in -vertical_radius..=vertical_radius {
        for z in -horizontal_radius..=horizontal_radius {
            for x in -horizontal_radius..=horizontal_radius {
                // Raio cilíndrico: economiza chunks nos cantos do quadrado e
                // produz uma distância visual mais uniforme.
                if x * x + z * z > radius_squared {
                    continue;
                }

                desired.insert(ChunkPos::new(center.x + x, center.y + y, center.z + z));
            }
        }
    }

    desired
}

fn chunk_distance_squared(position: ChunkPos, center: ChunkPos) -> i64 {
    let x = i64::from(position.x - center.x);
    let y = i64::from(position.y - center.y);
    let z = i64::from(position.z - center.z);
    x * x + y * y + z * z
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_chunks_on_the_vertical_axis() {
        let mut world = VoxelWorld::new(123);
        let delta = world.stream_around(
            Vec3::new(0.0, 24.0, 0.0),
            0,
            2,
            &HashSet::new(),
            true,
        );

        assert_eq!(delta.loaded.len(), 5);
        assert!(delta.loaded.iter().any(|position| position.y == -1));
        assert!(delta.loaded.iter().any(|position| position.y == 3));
    }

    #[test]
    fn moving_vertically_changes_the_stream_center() {
        let mut world = VoxelWorld::new(123);
        world.stream_around(
            Vec3::new(0.0, 1.0, 0.0),
            0,
            0,
            &HashSet::new(),
            true,
        );
        let lower = world.current_center();

        world.stream_around(
            Vec3::new(0.0, 33.0, 0.0),
            0,
            0,
            &HashSet::new(),
            false,
        );
        let upper = world.current_center();

        assert_ne!(lower, upper);
        assert_eq!(upper, Some(ChunkPos::new(0, 2, 0)));
    }

    #[test]
    fn physics_keepalive_prevents_unloading() {
        let mut world = VoxelWorld::new(123);
        let protected = HashSet::from([ChunkPos::new(10, 0, 0)]);

        world.stream_around(Vec3::ZERO, 0, 0, &protected, true);
        assert!(world.contains_chunk(ChunkPos::new(10, 0, 0)));

        world.stream_around(Vec3::new(64.0, 0.0, 0.0), 0, 0, &protected, false);
        assert!(world.contains_chunk(ChunkPos::new(10, 0, 0)));
        assert!(!world.is_render_visible(ChunkPos::new(10, 0, 0)));
    }
}
