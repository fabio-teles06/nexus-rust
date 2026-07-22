use engine_network::{ClientTransport, TransportError};

pub struct ClientRuntime<T>
where
    T: ClientTransport,
{
    transport: T,
}

impl<T> ClientRuntime<T>
where
    T: ClientTransport,
{
    pub fn new(transport: T) -> Self {
        Self { transport }
    }

    pub fn send(&mut self, message: T::Outgoing) -> Result<(), TransportError> {
        self.transport.send(message)
    }

    pub fn poll(&mut self) -> Result<Vec<T::Incoming>, TransportError> {
        let mut messages = Vec::new();

        while let Some(message) = self.transport.try_receive()? {
            messages.push(message);
        }

        Ok(messages)
    }

    pub fn transport(&self) -> &T {
        &self.transport
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    pub fn into_transport(self) -> T {
        self.transport
    }
}
