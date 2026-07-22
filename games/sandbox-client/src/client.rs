use std::{
    collections::HashMap,
    time::Instant,
};

use engine_client::ClientRuntime;
use engine_ecs::prelude::*;
use engine_network::TransportError;
use sandbox_server::SandboxClientTransport;
use sandbox_shared::{
    ClientMessage,
    NetworkId,
};

use crate::{
    components::{
        ClientEntityKind,
        ClientNetworkId,
        LocalPlayer,
    },
    input::{
        InputState,
        InputTransmission,
    },
};

pub(crate) struct SandboxClient {
    pub(crate) runtime:
        ClientRuntime<SandboxClientTransport>,

    pub(crate) world: World,

    pub(crate) entities:
        HashMap<NetworkId, Entity>,

    pub(crate) local_player: Option<NetworkId>,
    pub(crate) connected: bool,
}

impl SandboxClient {
    pub(crate) fn new(
        transport: SandboxClientTransport,
    ) -> Self {
        let mut world = World::new();

        world.insert_resource(InputState::default());
        world.insert_resource(
            InputTransmission::default(),
        );

        Self {
            runtime: ClientRuntime::new(transport),
            world,
            entities: HashMap::new(),
            local_player: None,
            connected: true,
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

    pub(crate) fn print_local_player(&mut self) {
        let mut query = self.world.query_filtered::<
            (
                &ClientNetworkId,
                &ClientEntityKind,
                &Transform,
            ),
            With<LocalPlayer>,
        >();

        for (network_id, kind, transform) in
            query.iter(&self.world)
        {
            println!(
                "[cliente] jogador: id={} tipo={:?} posição={:?}",
                network_id.0.0,
                kind.0,
                transform.translation
            );
        }
    }
}