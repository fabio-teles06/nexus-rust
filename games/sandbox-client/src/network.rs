use engine_ecs::prelude::*;
use engine_network::{
    ClientTransport,
    TransportError,
};
use sandbox_shared::ServerMessage;

use crate::client::SandboxClient;

impl SandboxClient {
    pub(crate) fn poll_server(
        &mut self,
    ) -> Result<(), TransportError> {
        loop {
            let received =
                self.runtime.transport_mut().try_receive();

            match received {
                Ok(Some(message)) => {
                    self.handle_server_message(message);
                }

                Ok(None) => {
                    return Ok(());
                }

                Err(TransportError::Disconnected) => {
                    self.connected = false;

                    println!(
                        "[cliente] conexão encerrada"
                    );

                    return Ok(());
                }

                Err(error) => {
                    return Err(error);
                }
            }

            if !self.connected {
                return Ok(());
            }
        }
    }

    fn handle_server_message(
        &mut self,
        message: ServerMessage,
    ) {
        match message {
            ServerMessage::Welcome {
                client_id,
                player_entity,
                player_name,
            } => {
                self.local_player = Some(player_entity);

                println!(
                    "[cliente] conectado como {} | cliente={} | entidade={}",
                    player_name,
                    client_id.0,
                    player_entity.0
                );

                self.mark_local_player_if_spawned();
            }

            ServerMessage::SpawnEntity {
                network_id,
                kind,
                transform,
            } => {
                self.spawn_or_update_entity(
                    network_id,
                    kind,
                    transform,
                );
            }

            ServerMessage::UpdateTransform {
                network_id,
                server_tick,
                transform,
            } => {
                self.update_entity_transform(
                    network_id,
                    transform,
                );

                /*
                 * Evita imprimir 30 vezes por segundo.
                 */
                if self.local_player == Some(network_id)
                    && server_tick.0 % 15 == 0
                {
                    println!(
                        "[cliente] tick={} posição={:?}",
                        server_tick.0,
                        Vec3::from_array(
                            transform.translation,
                        )
                    );
                }
            }

            ServerMessage::DespawnEntity {
                network_id,
            } => {
                self.despawn_entity(network_id);
            }

            ServerMessage::ServerTick { tick } => {
                println!(
                    "[cliente] tick do servidor: {}",
                    tick.0
                );
            }

            ServerMessage::Stopped => {
                println!(
                    "[cliente] servidor desligado"
                );

                self.connected = false;
            }
        }
    }
}