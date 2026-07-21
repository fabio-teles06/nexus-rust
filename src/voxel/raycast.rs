use glam::{IVec3, Vec3};

use super::world::World;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RaycastHit {
    pub position: IVec3,
    pub previous_position: IVec3,
    pub normal: IVec3,
    pub distance: f32,
}

pub fn raycast_world(
    world: &World,
    world_origin: Vec3,
    world_direction: Vec3,
    max_distance: f32,
) -> Option<RaycastHit> {
    if max_distance <= 0.0 {
        return None;
    }

    let direction = world_direction.normalize_or_zero();

    if direction == Vec3::ZERO {
        return None;
    }

    let render_origin = Vec3::from_array(world.render_origin());
    let local_origin = world_origin - render_origin;

    let mut voxel = local_origin.floor().as_ivec3();

    let step = IVec3::new(
        axis_step(direction.x),
        axis_step(direction.y),
        axis_step(direction.z),
    );

    let t_delta = Vec3::new(
        axis_delta(direction.x),
        axis_delta(direction.y),
        axis_delta(direction.z),
    );

    let mut t_max = Vec3::new(
        first_boundary_distance(local_origin.x, voxel.x, direction.x),
        first_boundary_distance(local_origin.y, voxel.y, direction.y),
        first_boundary_distance(local_origin.z, voxel.z, direction.z),
    );

    if world.contains(voxel.x, voxel.y, voxel.z) && world.get(voxel.x, voxel.y, voxel.z).is_solid()
    {
        return Some(RaycastHit {
            position: voxel,
            previous_position: voxel,
            normal: IVec3::ZERO,
            distance: 0.0,
        });
    }

    let mut distance = 0.0;

    while distance <= max_distance {
        let normal;

        if t_max.x <= t_max.y && t_max.x <= t_max.z {
            voxel.x += step.x;

            distance = t_max.x;
            t_max.x += t_delta.x;

            normal = IVec3::new(-step.x, 0, 0);
        } else if t_max.y <= t_max.z {
            voxel.y += step.y;

            distance = t_max.y;
            t_max.y += t_delta.y;

            normal = IVec3::new(0, -step.y, 0);
        } else {
            voxel.z += step.z;

            distance = t_max.z;
            t_max.z += t_delta.z;

            normal = IVec3::new(0, 0, -step.z);
        }

        if distance > max_distance {
            break;
        }

        if !world.contains(voxel.x, voxel.y, voxel.z) {
            continue;
        }

        let block = world.get(voxel.x, voxel.y, voxel.z);

        if block.is_solid() {
            return Some(RaycastHit {
                position: voxel,
                previous_position: voxel + normal,

                normal,
                distance,
            });
        }
    }

    None
}

fn axis_step(direction: f32) -> i32 {
    if direction > 0.0 {
        1
    } else if direction < 0.0 {
        -1
    } else {
        0
    }
}

fn axis_delta(direction: f32) -> f32 {
    if direction == 0.0 {
        f32::INFINITY
    } else {
        1.0 / direction.abs()
    }
}

fn first_boundary_distance(origin: f32, voxel: i32, direction: f32) -> f32 {
    if direction > 0.0 {
        let next_boundary = voxel as f32 + 1.0;

        (next_boundary - origin) / direction
    } else if direction < 0.0 {
        let previous_boundary = voxel as f32;

        (origin - previous_boundary) / direction.abs()
    } else {
        f32::INFINITY
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voxel::{block::STONE, world::World};

    #[test]
    fn hits_block_from_front() {
        let mut world = World::new(1, 1, 1);

        world.set(0, 0, 0, STONE);

        let hit = raycast_world(&world, Vec3::new(0.0, 0.5, 3.0), Vec3::NEG_Z, 10.0)
            .expect("O raio deveria atingir o bloco");

        assert_eq!(hit.position, IVec3::new(0, 0, 0),);

        assert_eq!(hit.normal, IVec3::new(0, 0, 1),);
    }

    #[test]
    fn misses_block() {
        let mut world = World::new(1, 1, 1);

        world.set(0, 0, 0, STONE);

        let hit = raycast_world(&world, Vec3::new(5.0, 5.0, 5.0), Vec3::X, 10.0);

        assert!(hit.is_none());
    }

    #[test]
    fn respects_maximum_distance() {
        let mut world = World::new(1, 1, 1);

        world.set(0, 0, 0, STONE);

        let hit = raycast_world(&world, Vec3::new(0.0, 0.5, 10.0), Vec3::NEG_Z, 2.0);

        assert!(hit.is_none());
    }
}
