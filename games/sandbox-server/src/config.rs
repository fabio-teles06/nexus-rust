use engine_ecs::prelude::Vec3;

pub const SERVER_TICK_RATE: u32 = 60;
pub const LOCAL_TRANSPORT_CAPACITY: usize = 1024;
pub const NETWORK_TRANSPORT_CAPACITY: usize = 1024;

pub const PLAYER_SPEED: f32 = 5.0;
pub const ARENA_HALF_SIZE: f32 = 12.0;
pub const ORB_COLLECT_DISTANCE: f32 = 1.25;
pub const TARGET_SCORE: u32 = 5;

pub const ORB_SPAWN_POINTS: [Vec3; 8] = [
    Vec3::new(0.0, 0.0, 0.0),
    Vec3::new(7.0, 0.0, 7.0),
    Vec3::new(-7.0, 0.0, 7.0),
    Vec3::new(7.0, 0.0, -7.0),
    Vec3::new(-7.0, 0.0, -7.0),
    Vec3::new(0.0, 0.0, 8.0),
    Vec3::new(8.0, 0.0, 0.0),
    Vec3::new(-8.0, 0.0, 0.0),
];
