use std::{
    collections::HashSet,
    time::{Duration, Instant},
};

use engine_ecs::prelude::*;
use engine_network::TransportError;
use sandbox_shared::{ClientMessage, PlayerInput};
use winit::{event::ElementState, keyboard::KeyCode};

use crate::client::SandboxClient;

const INPUT_HEARTBEAT_INTERVAL: Duration = Duration::from_millis(100);

/// Estado atual das teclas pressionadas.
///
/// É um Resource porque representa um estado global do cliente.
#[derive(Resource, Debug, Default)]
pub(crate) struct InputState {
    pressed_keys: HashSet<KeyCode>,
}

impl InputState {
    pub fn set_key(&mut self, key: KeyCode, state: ElementState) -> bool {
        if !is_movement_key(key) {
            return false;
        }

        match state {
            ElementState::Pressed => self.pressed_keys.insert(key),

            ElementState::Released => self.pressed_keys.remove(&key),
        }
    }

    pub fn clear(&mut self) -> bool {
        if self.pressed_keys.is_empty() {
            return false;
        }

        self.pressed_keys.clear();
        true
    }

    pub fn direction(&self) -> Vec3 {
        let forward = self.is_pressed(KeyCode::KeyW) || self.is_pressed(KeyCode::ArrowUp);

        let backward = self.is_pressed(KeyCode::KeyS) || self.is_pressed(KeyCode::ArrowDown);

        let left = self.is_pressed(KeyCode::KeyA) || self.is_pressed(KeyCode::ArrowLeft);

        let right = self.is_pressed(KeyCode::KeyD) || self.is_pressed(KeyCode::ArrowRight);

        let x = bool_to_axis(right) - bool_to_axis(left);

        let z = bool_to_axis(backward) - bool_to_axis(forward);

        Vec3::new(x, 0.0, z).normalize_or_zero()
    }

    fn is_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }
}

/// Controla quando e qual input foi enviado.
#[derive(Resource, Debug)]
pub(crate) struct InputTransmission {
    next_sequence: u32,
    last_sent_direction: Vec3,
    last_sent_at: Option<Instant>,
}

impl Default for InputTransmission {
    fn default() -> Self {
        Self {
            next_sequence: 1,
            last_sent_direction: Vec3::ZERO,
            last_sent_at: None,
        }
    }
}

impl InputTransmission {
    fn should_send(&self, direction: Vec3, now: Instant) -> bool {
        let direction_changed = direction != self.last_sent_direction;

        let heartbeat_expired = self.last_sent_at.is_none_or(|last_sent_at| {
            now.duration_since(last_sent_at) >= INPUT_HEARTBEAT_INTERVAL
        });

        direction_changed || heartbeat_expired
    }

    fn sequence(&self) -> u32 {
        self.next_sequence
    }

    fn mark_sent(&mut self, direction: Vec3, now: Instant) {
        self.last_sent_direction = direction;
        self.last_sent_at = Some(now);
        self.next_sequence = self.next_sequence.wrapping_add(1);
    }
}

impl SandboxClient {
    pub(crate) fn handle_key(&mut self, key: KeyCode, state: ElementState) -> bool {
        self.world.resource_mut::<InputState>().set_key(key, state)
    }

    pub(crate) fn clear_input(&mut self) -> bool {
        self.world.resource_mut::<InputState>().clear()
    }

    /// Envia o estado atual quando:
    ///
    /// - alguma tecla mudou;
    /// - ou o heartbeat venceu.
    pub(crate) fn send_current_input(&mut self, now: Instant) -> Result<(), TransportError> {
        /*
         * Não envia input antes do servidor confirmar
         * a criação do jogador.
         */
        if self.local_player.is_none() {
            return Ok(());
        }

        let direction = self.world.resource::<InputState>().direction();

        let (should_send, sequence) = {
            let transmission = self.world.resource::<InputTransmission>();

            (
                transmission.should_send(direction, now),
                transmission.sequence(),
            )
        };

        if !should_send {
            return Ok(());
        }

        self.send(ClientMessage::Input(PlayerInput {
            sequence,
            direction: direction.to_array(),
        }))?;

        self.world
            .resource_mut::<InputTransmission>()
            .mark_sent(direction, now);

        Ok(())
    }
}

fn bool_to_axis(value: bool) -> f32 {
    if value { 1.0 } else { 0.0 }
}

fn is_movement_key(key: KeyCode) -> bool {
    matches!(
        key,
        KeyCode::KeyW
            | KeyCode::KeyA
            | KeyCode::KeyS
            | KeyCode::KeyD
            | KeyCode::ArrowUp
            | KeyCode::ArrowDown
            | KeyCode::ArrowLeft
            | KeyCode::ArrowRight
    )
}
