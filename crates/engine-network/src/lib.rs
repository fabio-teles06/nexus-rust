use engine_core::ClientId;
use std::{
    error::Error,
    fmt,
    sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError, sync_channel},
};

#[derive(Debug)]
pub enum TransportError {
    QueueFull,
    Disconnected,
    UnknownClient(ClientId),
}

impl fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueueFull => {
                write!(formatter, "a fila de mensagens está cheia")
            }

            Self::Disconnected => {
                write!(formatter, "o transporte foi desconectado")
            }

            Self::UnknownClient(client_id) => {
                write!(formatter, "cliente desconhecido: {}", client_id.0)
            }
        }
    }
}

impl Error for TransportError {}

/// Transporte utilizado pelo cliente.
///
/// `Outgoing` são mensagens enviadas ao servidor.
/// `Incoming` são mensagens recebidas do servidor.
pub trait ClientTransport: Send {
    type Outgoing: Send + 'static;
    type Incoming: Send + 'static;

    fn send(&mut self, message: Self::Outgoing) -> Result<(), TransportError>;

    fn try_receive(&mut self) -> Result<Option<Self::Incoming>, TransportError>;
}

/// Transporte utilizado pelo servidor.
pub trait ServerTransport: Send {
    type Incoming: Send + 'static;
    type Outgoing: Send + 'static;

    fn send(&mut self, client_id: ClientId, message: Self::Outgoing) -> Result<(), TransportError>;

    fn try_receive(&mut self) -> Result<Option<(ClientId, Self::Incoming)>, TransportError>;
}

/// Lado do cliente no transporte em memória.
pub struct LocalClientTransport<ClientMessage, ServerMessage> {
    client_id: ClientId,

    to_server: SyncSender<(ClientId, ClientMessage)>,
    from_server: Receiver<ServerMessage>,
}

/// Lado do servidor no transporte em memória.
pub struct LocalServerTransport<ClientMessage, ServerMessage> {
    client_id: ClientId,

    from_client: Receiver<(ClientId, ClientMessage)>,
    to_client: SyncSender<ServerMessage>,
}

/// Cria os dois lados de uma conexão local.
///
/// O cliente e o servidor podem ficar em threads diferentes,
/// mas não precisam abrir portas de rede.
pub fn local_transport_pair<ClientMessage, ServerMessage>(
    client_id: ClientId,
    capacity: usize,
) -> (
    LocalClientTransport<ClientMessage, ServerMessage>,
    LocalServerTransport<ClientMessage, ServerMessage>,
)
where
    ClientMessage: Send + 'static,
    ServerMessage: Send + 'static,
{
    let capacity = capacity.max(1);

    let (client_to_server_tx, client_to_server_rx) = sync_channel(capacity);

    let (server_to_client_tx, server_to_client_rx) = sync_channel(capacity);

    let client = LocalClientTransport {
        client_id,
        to_server: client_to_server_tx,
        from_server: server_to_client_rx,
    };

    let server = LocalServerTransport {
        client_id,
        from_client: client_to_server_rx,
        to_client: server_to_client_tx,
    };

    (client, server)
}

impl<ClientMessage, ServerMessage> ClientTransport
    for LocalClientTransport<ClientMessage, ServerMessage>
where
    ClientMessage: Send + 'static,
    ServerMessage: Send + 'static,
{
    type Outgoing = ClientMessage;
    type Incoming = ServerMessage;

    fn send(&mut self, message: Self::Outgoing) -> Result<(), TransportError> {
        match self.to_server.try_send((self.client_id, message)) {
            Ok(()) => Ok(()),

            Err(TrySendError::Full(_)) => Err(TransportError::QueueFull),

            Err(TrySendError::Disconnected(_)) => Err(TransportError::Disconnected),
        }
    }

    fn try_receive(&mut self) -> Result<Option<Self::Incoming>, TransportError> {
        match self.from_server.try_recv() {
            Ok(message) => Ok(Some(message)),
            Err(TryRecvError::Empty) => Ok(None),

            Err(TryRecvError::Disconnected) => Err(TransportError::Disconnected),
        }
    }
}

impl<ClientMessage, ServerMessage> ServerTransport
    for LocalServerTransport<ClientMessage, ServerMessage>
where
    ClientMessage: Send + 'static,
    ServerMessage: Send + 'static,
{
    type Incoming = ClientMessage;
    type Outgoing = ServerMessage;

    fn send(&mut self, client_id: ClientId, message: Self::Outgoing) -> Result<(), TransportError> {
        if client_id != self.client_id {
            return Err(TransportError::UnknownClient(client_id));
        }

        match self.to_client.try_send(message) {
            Ok(()) => Ok(()),

            Err(TrySendError::Full(_)) => Err(TransportError::QueueFull),

            Err(TrySendError::Disconnected(_)) => Err(TransportError::Disconnected),
        }
    }

    fn try_receive(&mut self) -> Result<Option<(ClientId, Self::Incoming)>, TransportError> {
        match self.from_client.try_recv() {
            Ok(message) => Ok(Some(message)),
            Err(TryRecvError::Empty) => Ok(None),

            Err(TryRecvError::Disconnected) => Err(TransportError::Disconnected),
        }
    }
}
