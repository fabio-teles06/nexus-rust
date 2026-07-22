use std::{collections::HashMap, time::Instant};

use engine_client::ClientRuntime;
use engine_ecs::prelude::*;
use engine_network::TransportError;
use sandbox_shared::{
    ClientMessage, NetworkId, PlayerScoreSnapshot,
};

use crate::{
    components::{
        ClientEntityKind, ClientNetworkId, LocalPlayer,
    },
    connection::ClientConnection,
    input::{InputState, InputTransmission},
};

pub(crate) struct SandboxClient {
    pub(crate) runtime: ClientRuntime<ClientConnection>,
    pub(crate) world: World,
    pub(crate) entities: HashMap<NetworkId, Entity>,
    pub(crate) local_player: Option<NetworkId>,
    pub(crate) connected: bool,
    pub(crate) scoreboard: Vec<PlayerScoreSnapshot>,
    pub(crate) target_score: u32,
    pub(crate) winner: Option<String>,
}

impl SandboxClient {
    pub(crate) fn new(transport: ClientConnection) -> Self {
        let mut world = World::new();

        world.insert_resource(InputState::default());
        world.insert_resource(InputTransmission::default());

        Self {
            runtime: ClientRuntime::new(transport),
            world,
            entities: HashMap::new(),
            local_player: None,
            connected: true,
            scoreboard: Vec::new(),
            target_score: 5,
            winner: None,
        }
    }

    pub(crate) fn send(
        &mut self,
        message: ClientMessage,
    ) -> Result<(), TransportError> {
        self.runtime.send(message)
    }

    pub(crate) fn update(
        &mut self,
        now: Instant,
    ) -> Result<(), TransportError> {
        self.poll_server()?;

        if self.connected {
            self.send_current_input(now)?;
        }

        Ok(())
    }

    pub(crate) fn connected(&self) -> bool {
        self.connected
    }

    pub(crate) fn window_title(&self) -> String {
        let scores = self
            .scoreboard
            .iter()
            .map(|player| {
                format!("{}:{}", player.player_name, player.score)
            })
            .collect::<Vec<_>>()
            .join(" | ");

        match &self.winner {
            Some(winner) => format!(
                "Nexus Arena — {winner} venceu! — {scores}"
            ),
            None if scores.is_empty() => {
                "Nexus Arena — conectando...".to_string()
            }
            None => format!(
                "Nexus Arena — primeiro a {} — {}",
                self.target_score, scores
            ),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn print_local_player(&mut self) {
        let mut query = self.world.query_filtered::<
            (
                &ClientNetworkId,
                &ClientEntityKind,
                &Transform,
            ),
            With<LocalPlayer>,
        >();

        for (network_id, kind, transform) in query.iter(&self.world) {
            println!(
                "[cliente] jogador: id={} tipo={:?} posição={:?}",
                network_id.0.0,
                kind.0,
                transform.translation
            );
        }
    }
}
