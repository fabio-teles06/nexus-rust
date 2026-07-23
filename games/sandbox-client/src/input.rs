use bevy_ecs::prelude::*;
use engine_ecs::prelude::*;
use sandbox_shared::PlayerInput;
use std::{collections::{HashSet, VecDeque}, time::{Duration, Instant}};
use winit::{event::ElementState, keyboard::KeyCode};

pub(crate) const CLIENT_FIXED_DELTA: f32 = 1.0 / 30.0;
#[derive(Resource, Default)] pub(crate) struct InputState { pressed: HashSet<KeyCode> }
impl InputState {
    pub fn set(&mut self,key:KeyCode,state:ElementState)->bool{if !matches!(key,KeyCode::KeyW|KeyCode::KeyA|KeyCode::KeyS|KeyCode::KeyD|KeyCode::ArrowUp|KeyCode::ArrowDown|KeyCode::ArrowLeft|KeyCode::ArrowRight){return false;}match state{ElementState::Pressed=>self.pressed.insert(key),ElementState::Released=>self.pressed.remove(&key)}}
    pub fn clear(&mut self)->bool{let changed=!self.pressed.is_empty();self.pressed.clear();changed}
    pub fn direction(&self)->Vec3{let x=(self.pressed.contains(&KeyCode::KeyD)||self.pressed.contains(&KeyCode::ArrowRight))as i32-(self.pressed.contains(&KeyCode::KeyA)||self.pressed.contains(&KeyCode::ArrowLeft))as i32;let z=(self.pressed.contains(&KeyCode::KeyS)||self.pressed.contains(&KeyCode::ArrowDown))as i32-(self.pressed.contains(&KeyCode::KeyW)||self.pressed.contains(&KeyCode::ArrowUp))as i32;Vec3::new(x as f32,0.,z as f32).normalize_or_zero()}
}
#[derive(Debug,Clone,Copy)] pub(crate) struct PredictedInput { pub input: PlayerInput, pub delta_seconds: f32 }
#[derive(Resource)] pub(crate) struct PredictionState { pub next_sequence:u32,pub pending:VecDeque<PredictedInput>,pub accumulator:Duration,pub last_frame:Instant }
impl Default for PredictionState{fn default()->Self{Self{next_sequence:1,pending:VecDeque::new(),accumulator:Duration::ZERO,last_frame:Instant::now()}}}
