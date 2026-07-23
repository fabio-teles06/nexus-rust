use engine_core::{ClientId, FixedTicker, Tick};
use engine_network::{ConnectionEvent, ServerTransport, TransportError};

pub enum Outgoing<M> { To(ClientId, M), Broadcast(M) }

pub trait ServerGame {
    type ClientMessage: Send + 'static;
    type ServerMessage: Clone + Send + 'static;
    fn connected(&mut self, client_id: ClientId);
    fn disconnected(&mut self, client_id: ClientId);
    fn handle_message(&mut self, client_id: ClientId, message: Self::ClientMessage);
    fn update(&mut self, tick: Tick);
    fn drain_outgoing(&mut self) -> Vec<Outgoing<Self::ServerMessage>>;
    fn is_running(&self) -> bool;
}

pub struct ServerRuntime<G, T> { game: G, transport: T, tick_rate: u32 }
impl<G, T> ServerRuntime<G, T>
where
    G: ServerGame,
    T: ServerTransport<Incoming = G::ClientMessage, Outgoing = G::ServerMessage>,
{
    pub fn new(game: G, transport: T, tick_rate: u32) -> Self { Self { game, transport, tick_rate } }
    pub fn run(mut self) -> Result<(), TransportError> {
        let mut ticker = FixedTicker::new(self.tick_rate);
        while self.game.is_running() {
            while let Some(event) = self.transport.try_receive()? {
                match event {
                    ConnectionEvent::Connected(id) => self.game.connected(id),
                    ConnectionEvent::Disconnected(id) => self.game.disconnected(id),
                    ConnectionEvent::Message(id, message) => self.game.handle_message(id, message),
                }
            }
            self.game.update(ticker.current_tick());
            for outgoing in self.game.drain_outgoing() {
                match outgoing {
                    Outgoing::To(id, message) => {
                        if !matches!(self.transport.send(id, message), Ok(()) | Err(TransportError::UnknownClient(_))) {
                            return Err(TransportError::Disconnected);
                        }
                    }
                    Outgoing::Broadcast(message) => self.transport.broadcast(message)?,
                }
            }
            ticker.wait_for_next_tick();
        }
        Ok(())
    }
}
