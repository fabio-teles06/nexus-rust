use engine_network::{ClientTransport, TransportError};

pub struct ClientRuntime<T: ClientTransport> { transport: T }
impl<T: ClientTransport> ClientRuntime<T> {
    pub fn new(transport: T) -> Self { Self { transport } }
    pub fn send(&mut self, message: T::Outgoing) -> Result<(), TransportError> { self.transport.send(message) }
    pub fn transport_mut(&mut self) -> &mut T { &mut self.transport }
    pub fn client_id(&self) -> engine_core::ClientId where T: ClientTransport { self.transport.client_id() }
}
