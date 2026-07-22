use engine_network::{
    ClientTransport, LocalClientTransport, TcpClientTransport,
    TransportError,
};
use sandbox_shared::{ClientMessage, ServerMessage};

pub(crate) enum ClientConnection {
    Local(LocalClientTransport<ClientMessage, ServerMessage>),
    Remote(TcpClientTransport<ClientMessage, ServerMessage>),
}

impl ClientConnection {
    pub fn local(
        transport: LocalClientTransport<ClientMessage, ServerMessage>,
    ) -> Self {
        Self::Local(transport)
    }

    pub fn remote(address: &str) -> Result<Self, TransportError> {
        Ok(Self::Remote(TcpClientTransport::connect(
            address,
            1024,
        )?))
    }
}

impl ClientTransport for ClientConnection {
    type Outgoing = ClientMessage;
    type Incoming = ServerMessage;

    fn send(
        &mut self,
        message: Self::Outgoing,
    ) -> Result<(), TransportError> {
        match self {
            Self::Local(transport) => transport.send(message),
            Self::Remote(transport) => transport.send(message),
        }
    }

    fn try_receive(
        &mut self,
    ) -> Result<Option<Self::Incoming>, TransportError> {
        match self {
            Self::Local(transport) => transport.try_receive(),
            Self::Remote(transport) => transport.try_receive(),
        }
    }
}
