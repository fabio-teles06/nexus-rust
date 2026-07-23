use bevy_ecs::prelude::*;
use engine_client::ClientRuntime;
use engine_core::ClientId;
use engine_ecs::{RenderTransform, SimulationTransform};
use engine_network::TransportError;
use glam::Vec3;
use sandbox_server::SandboxClientTransport;
use sandbox_shared::{ClientMessage, NetworkId, PlayerInput};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use crate::{
    assets::initialize_assets_and_scene,
    components::{ClientNetworkId, LocalPlayer},
    input::{CLIENT_FIXED_DELTA, InputState, PredictedInput, PredictionState},
};

const PLAYER_SPEED: f32 = 5.0;

pub(crate) struct SandboxClient {
    pub(crate) runtime: ClientRuntime<SandboxClientTransport>,
    pub(crate) world: World,
    pub(crate) entities: HashMap<NetworkId, Entity>,
    pub(crate) local_player: Option<NetworkId>,
    pub(crate) connected: bool,
}

impl SandboxClient {
    pub fn new(transport: SandboxClientTransport) -> Self {
        let mut world = World::new();
        world.insert_resource(InputState::default());
        world.insert_resource(PredictionState::default());
        initialize_assets_and_scene(&mut world);

        Self {
            runtime: ClientRuntime::new(transport),
            world,
            entities: HashMap::new(),
            local_player: None,
            connected: true,
        }
    }

    pub fn send(&mut self, message: ClientMessage) -> Result<(), TransportError> {
        self.runtime.send(message)
    }

    pub fn handle_key(
        &mut self,
        key: winit::keyboard::KeyCode,
        state: winit::event::ElementState,
    ) -> bool {
        self.world.resource_mut::<InputState>().set(key, state)
    }

    pub fn clear_input(&mut self) -> bool {
        self.world.resource_mut::<InputState>().clear()
    }

    pub fn update(&mut self, now: Instant) -> Result<(), TransportError> {
        self.poll_server()?;
        self.advance_prediction(now)?;
        self.interpolate_remote_entities();
        Ok(())
    }

    fn advance_prediction(&mut self, now: Instant) -> Result<(), TransportError> {
        if self.local_player.is_none() {
            self.world.resource_mut::<PredictionState>().last_frame = now;
            return Ok(());
        }

        {
            let mut state = self.world.resource_mut::<PredictionState>();
            let elapsed = now
                .saturating_duration_since(state.last_frame)
                .min(Duration::from_millis(250));

            state.last_frame = now;
            state.accumulator += elapsed;
        }

        let fixed_duration = Duration::from_secs_f32(CLIENT_FIXED_DELTA);

        while self.world.resource::<PredictionState>().accumulator >= fixed_duration {
            self.world.resource_mut::<PredictionState>().accumulator -= fixed_duration;

            let direction = self.world.resource::<InputState>().direction();
            let sequence = self.world.resource::<PredictionState>().next_sequence;
            let input = PlayerInput {
                sequence,
                direction: direction.to_array(),
            };

            self.send(ClientMessage::Input(input))?;

            {
                let mut prediction = self.world.resource_mut::<PredictionState>();
                prediction.next_sequence = prediction.next_sequence.wrapping_add(1);
                prediction.pending.push_back(PredictedInput {
                    input,
                    delta_seconds: CLIENT_FIXED_DELTA,
                });

                while prediction.pending.len() > 128 {
                    prediction.pending.pop_front();
                }
            }

            self.apply_local_prediction(direction, CLIENT_FIXED_DELTA);
        }

        Ok(())
    }

    fn apply_local_prediction(&mut self, direction: Vec3, delta_seconds: f32) {
        let Some(network_id) = self.local_player else {
            return;
        };
        let Some(&entity) = self.entities.get(&network_id) else {
            return;
        };

        if let Some(mut simulation) = self.world.get_mut::<SimulationTransform>(entity) {
            simulation.0.translation += direction * PLAYER_SPEED * delta_seconds;
        }

        if let Some(simulation) = self.world.get::<SimulationTransform>(entity).copied() {
            if let Some(mut render) = self.world.get_mut::<RenderTransform>(entity) {
                render.0 = simulation.0;
            }
        }
    }

    pub fn client_id(&self) -> ClientId {
        self.runtime.client_id()
    }

    pub fn connected(&self) -> bool {
        self.connected
    }

    pub fn local_position(&self) -> Option<Vec3> {
        let network_id = self.local_player?;
        let entity = *self.entities.get(&network_id)?;
        self.world
            .get::<SimulationTransform>(entity)
            .map(|transform| transform.0.translation)
    }

    #[allow(dead_code)]
    pub(crate) fn validate_local_player_query(&mut self) {
        let mut query = self
            .world
            .query_filtered::<(&ClientNetworkId, &SimulationTransform), With<LocalPlayer>>();
        for _ in query.iter(&self.world) {}
    }
}
