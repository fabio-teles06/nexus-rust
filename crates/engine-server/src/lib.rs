use engine_core::{ClientId, FixedTicker, Tick};

use engine_network::{ServerTransport, TransportError};

/// Interface que um jogo deve implementar no lado servidor.
pub trait ServerGame {
    type ClientMessage: Send + 'static;
    type ServerMessage: Send + 'static;

    /// Processa uma mensagem recebida de um cliente.
    fn handle_message(&mut self, client_id: ClientId, message: Self::ClientMessage);

    /// Atualiza a simulação do mundo.
    fn update(&mut self, tick: Tick);

    /// Retorna as mensagens pendentes que serão enviadas.
    fn drain_outgoing(&mut self) -> Vec<(ClientId, Self::ServerMessage)>;

    /// Informa se o servidor deve continuar executando.
    fn is_running(&self) -> bool;
}

pub struct ServerRuntime<Game, Transport>
where
    Game: ServerGame,
    Transport: ServerTransport<Incoming = Game::ClientMessage, Outgoing = Game::ServerMessage>,
{
    game: Game,
    transport: Transport,
    tick_rate: u32,
}

impl<Game, Transport> ServerRuntime<Game, Transport>
where
    Game: ServerGame,
    Transport: ServerTransport<Incoming = Game::ClientMessage, Outgoing = Game::ServerMessage>,
{
    pub fn new(game: Game, transport: Transport, tick_rate: u32) -> Self {
        assert!(tick_rate > 0);

        Self {
            game,
            transport,
            tick_rate,
        }
    }

    pub fn run(mut self) -> Result<(), TransportError> {
        let mut ticker = FixedTicker::new(self.tick_rate);

        while self.game.is_running() {
            /*
             * Processa todas as mensagens disponíveis antes
             * de atualizar o mundo.
             */
            while let Some((client_id, message)) = self.transport.try_receive()? {
                self.game.handle_message(client_id, message);
            }

            /*
             * Uma mensagem pode ter solicitado o desligamento.
             */
            if self.game.is_running() {
                self.game.update(ticker.current_tick());
            }

            /*
             * Envia todas as respostas produzidas pelo jogo.
             */
            for (client_id, message) in self.game.drain_outgoing() {
                self.transport.send(client_id, message)?;
            }

            if !self.game.is_running() {
                break;
            }

            ticker.wait_for_next_tick();
        }

        Ok(())
    }
}
