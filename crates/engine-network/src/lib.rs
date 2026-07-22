use engine_core::ClientId;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::HashMap,
    error::Error,
    fmt,
    io::{ErrorKind, Read, Write},
    marker::PhantomData,
    net::{TcpListener, TcpStream, ToSocketAddrs},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
        mpsc::{
            Receiver, SyncSender, TryRecvError, TrySendError, sync_channel,
        },
    },
    thread,
};

const MAX_PACKET_SIZE: usize = 1024 * 1024;

#[derive(Debug)]
pub enum TransportError {
    QueueFull,
    Disconnected,
    UnknownClient(ClientId),
    Io(String),
    Serialization(String),
    InvalidPacket(String),
}

impl fmt::Display for TransportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueueFull => write!(formatter, "a fila de mensagens está cheia"),
            Self::Disconnected => write!(formatter, "o transporte foi desconectado"),
            Self::UnknownClient(client_id) => {
                write!(formatter, "cliente desconhecido: {}", client_id.0)
            }
            Self::Io(message) => write!(formatter, "erro de rede: {message}"),
            Self::Serialization(message) => {
                write!(formatter, "erro de serialização: {message}")
            }
            Self::InvalidPacket(message) => write!(formatter, "pacote inválido: {message}"),
        }
    }
}

impl Error for TransportError {}

pub trait ClientTransport: Send {
    type Outgoing: Send + 'static;
    type Incoming: Send + 'static;

    fn send(&mut self, message: Self::Outgoing) -> Result<(), TransportError>;

    fn try_receive(&mut self) -> Result<Option<Self::Incoming>, TransportError>;
}

pub trait ServerTransport: Send {
    type Incoming: Send + 'static;
    type Outgoing: Send + 'static;

    fn send(
        &mut self,
        client_id: ClientId,
        message: Self::Outgoing,
    ) -> Result<(), TransportError>;

    fn try_receive(
        &mut self,
    ) -> Result<Option<(ClientId, Self::Incoming)>, TransportError>;
}

// -----------------------------------------------------------------------------
// Transporte local
// -----------------------------------------------------------------------------

pub struct LocalClientTransport<ClientMessage, ServerMessage> {
    client_id: ClientId,
    to_server: SyncSender<(ClientId, ClientMessage)>,
    from_server: Receiver<ServerMessage>,
}

pub struct LocalServerTransport<ClientMessage, ServerMessage> {
    client_id: ClientId,
    from_client: Receiver<(ClientId, ClientMessage)>,
    to_client: SyncSender<ServerMessage>,
}

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

    (
        LocalClientTransport {
            client_id,
            to_server: client_to_server_tx,
            from_server: server_to_client_rx,
        },
        LocalServerTransport {
            client_id,
            from_client: client_to_server_rx,
            to_client: server_to_client_tx,
        },
    )
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

    fn send(
        &mut self,
        client_id: ClientId,
        message: Self::Outgoing,
    ) -> Result<(), TransportError> {
        if client_id != self.client_id {
            return Err(TransportError::UnknownClient(client_id));
        }

        match self.to_client.try_send(message) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(TransportError::QueueFull),
            Err(TrySendError::Disconnected(_)) => Err(TransportError::Disconnected),
        }
    }

    fn try_receive(
        &mut self,
    ) -> Result<Option<(ClientId, Self::Incoming)>, TransportError> {
        match self.from_client.try_recv() {
            Ok(message) => Ok(Some(message)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(TransportError::Disconnected),
        }
    }
}

// -----------------------------------------------------------------------------
// Transporte TCP
// -----------------------------------------------------------------------------

pub struct TcpClientTransport<Outgoing, Incoming> {
    outgoing: SyncSender<Vec<u8>>,
    incoming: Receiver<Incoming>,
    _outgoing: PhantomData<Outgoing>,
}

impl<Outgoing, Incoming> TcpClientTransport<Outgoing, Incoming>
where
    Outgoing: Serialize + Send + 'static,
    Incoming: DeserializeOwned + Send + 'static,
{
    pub fn connect(
        address: impl ToSocketAddrs,
        capacity: usize,
    ) -> Result<Self, TransportError> {
        let stream = TcpStream::connect(address)
            .map_err(|error| TransportError::Io(error.to_string()))?;

        stream
            .set_nodelay(true)
            .map_err(|error| TransportError::Io(error.to_string()))?;

        let mut reader = stream
            .try_clone()
            .map_err(|error| TransportError::Io(error.to_string()))?;

        let mut writer = stream;
        let capacity = capacity.max(1);

        let (outgoing_tx, outgoing_rx) = sync_channel::<Vec<u8>>(capacity);
        let (incoming_tx, incoming_rx) = sync_channel::<Incoming>(capacity);

        thread::Builder::new()
            .name("tcp-client-writer".to_string())
            .spawn(move || {
                while let Ok(frame) = outgoing_rx.recv() {
                    if writer.write_all(&frame).is_err() {
                        break;
                    }
                }
            })
            .map_err(|error| TransportError::Io(error.to_string()))?;

        thread::Builder::new()
            .name("tcp-client-reader".to_string())
            .spawn(move || {
                loop {
                    match read_message::<Incoming>(&mut reader) {
                        Ok(message) => {
                            if incoming_tx.send(message).is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            })
            .map_err(|error| TransportError::Io(error.to_string()))?;

        Ok(Self {
            outgoing: outgoing_tx,
            incoming: incoming_rx,
            _outgoing: PhantomData,
        })
    }
}

impl<Outgoing, Incoming> ClientTransport for TcpClientTransport<Outgoing, Incoming>
where
    Outgoing: Serialize + Send + 'static,
    Incoming: DeserializeOwned + Send + 'static,
{
    type Outgoing = Outgoing;
    type Incoming = Incoming;

    fn send(&mut self, message: Self::Outgoing) -> Result<(), TransportError> {
        let frame = encode_message(&message)?;

        match self.outgoing.try_send(frame) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(TransportError::QueueFull),
            Err(TrySendError::Disconnected(_)) => Err(TransportError::Disconnected),
        }
    }

    fn try_receive(&mut self) -> Result<Option<Self::Incoming>, TransportError> {
        match self.incoming.try_recv() {
            Ok(message) => Ok(Some(message)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(TransportError::Disconnected),
        }
    }
}

pub struct TcpServerTransport<Incoming, Outgoing> {
    incoming: Receiver<(ClientId, Incoming)>,
    clients: Arc<Mutex<HashMap<ClientId, SyncSender<Vec<u8>>>>>,
    _outgoing: PhantomData<Outgoing>,
}

impl<Incoming, Outgoing> TcpServerTransport<Incoming, Outgoing>
where
    Incoming: DeserializeOwned + Clone + Send + 'static,
    Outgoing: Serialize + Send + 'static,
{
    pub fn bind(
        address: impl ToSocketAddrs,
        capacity: usize,
    ) -> Result<Self, TransportError> {
        Self::bind_internal(address, capacity, None)
    }

    pub fn bind_with_disconnect(
        address: impl ToSocketAddrs,
        capacity: usize,
        disconnect_message: Incoming,
    ) -> Result<Self, TransportError> {
        Self::bind_internal(address, capacity, Some(disconnect_message))
    }

    fn bind_internal(
        address: impl ToSocketAddrs,
        capacity: usize,
        disconnect_message: Option<Incoming>,
    ) -> Result<Self, TransportError> {
        let listener = TcpListener::bind(address)
            .map_err(|error| TransportError::Io(error.to_string()))?;

        let capacity = capacity.max(1);
        let (incoming_tx, incoming_rx) =
            sync_channel::<(ClientId, Incoming)>(capacity);

        let clients = Arc::new(Mutex::new(HashMap::new()));
        let accept_clients = Arc::clone(&clients);
        let next_client_id = Arc::new(AtomicU32::new(1));

        thread::Builder::new()
            .name("tcp-server-accept".to_string())
            .spawn(move || {
                for accepted in listener.incoming() {
                    let Ok(stream) = accepted else {
                        continue;
                    };

                    if stream.set_nodelay(true).is_err() {
                        continue;
                    }

                    let client_id =
                        ClientId(next_client_id.fetch_add(1, Ordering::Relaxed));

                    let Ok(mut reader) = stream.try_clone() else {
                        continue;
                    };

                    let mut writer = stream;
                    let (client_tx, client_rx) =
                        sync_channel::<Vec<u8>>(capacity);

                    if let Ok(mut connected) = accept_clients.lock() {
                        connected.insert(client_id, client_tx);
                    } else {
                        continue;
                    }

                    println!("[rede] cliente TCP {} conectado", client_id.0);

                    let writer_clients = Arc::clone(&accept_clients);

                    let _ = thread::Builder::new()
                        .name(format!("tcp-server-writer-{}", client_id.0))
                        .spawn(move || {
                            while let Ok(frame) = client_rx.recv() {
                                if writer.write_all(&frame).is_err() {
                                    break;
                                }
                            }

                            if let Ok(mut connected) = writer_clients.lock() {
                                connected.remove(&client_id);
                            }
                        });

                    let reader_clients = Arc::clone(&accept_clients);
                    let reader_incoming = incoming_tx.clone();
                    let disconnect_message = disconnect_message.clone();

                    let _ = thread::Builder::new()
                        .name(format!("tcp-server-reader-{}", client_id.0))
                        .spawn(move || {
                            loop {
                                match read_message::<Incoming>(&mut reader) {
                                    Ok(message) => {
                                        if reader_incoming
                                            .send((client_id, message))
                                            .is_err()
                                        {
                                            break;
                                        }
                                    }
                                    Err(_) => break,
                                }
                            }

                            if let Ok(mut connected) = reader_clients.lock() {
                                connected.remove(&client_id);
                            }

                            if let Some(message) = disconnect_message {
                                let _ = reader_incoming.send((client_id, message));
                            }

                            println!(
                                "[rede] cliente TCP {} desconectado",
                                client_id.0
                            );
                        });
                }
            })
            .map_err(|error| TransportError::Io(error.to_string()))?;

        Ok(Self {
            incoming: incoming_rx,
            clients,
            _outgoing: PhantomData,
        })
    }
}

impl<Incoming, Outgoing> ServerTransport for TcpServerTransport<Incoming, Outgoing>
where
    Incoming: DeserializeOwned + Clone + Send + 'static,
    Outgoing: Serialize + Send + 'static,
{
    type Incoming = Incoming;
    type Outgoing = Outgoing;

    fn send(
        &mut self,
        client_id: ClientId,
        message: Self::Outgoing,
    ) -> Result<(), TransportError> {
        let frame = encode_message(&message)?;

        let sender = self
            .clients
            .lock()
            .map_err(|_| TransportError::Disconnected)?
            .get(&client_id)
            .cloned();

        // O cliente pode ter saído entre a simulação e o envio.
        // Neste caso, descartamos silenciosamente a mensagem.
        let Some(sender) = sender else {
            return Ok(());
        };

        match sender.try_send(frame) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_)) => Err(TransportError::QueueFull),
            Err(TrySendError::Disconnected(_)) => Ok(()),
        }
    }

    fn try_receive(
        &mut self,
    ) -> Result<Option<(ClientId, Self::Incoming)>, TransportError> {
        match self.incoming.try_recv() {
            Ok(message) => Ok(Some(message)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(TransportError::Disconnected),
        }
    }
}

fn encode_message<T: Serialize>(message: &T) -> Result<Vec<u8>, TransportError> {
    let payload = bincode::serialize(message)
        .map_err(|error| TransportError::Serialization(error.to_string()))?;

    if payload.len() > MAX_PACKET_SIZE {
        return Err(TransportError::InvalidPacket(format!(
            "mensagem com {} bytes excede o limite de {}",
            payload.len(),
            MAX_PACKET_SIZE
        )));
    }

    let mut frame = Vec::with_capacity(4 + payload.len());
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(&payload);

    Ok(frame)
}

fn read_message<T: DeserializeOwned>(
    stream: &mut TcpStream,
) -> Result<T, TransportError> {
    let mut length_bytes = [0_u8; 4];

    stream
        .read_exact(&mut length_bytes)
        .map_err(map_read_error)?;

    let length = u32::from_be_bytes(length_bytes) as usize;

    if length == 0 || length > MAX_PACKET_SIZE {
        return Err(TransportError::InvalidPacket(format!(
            "tamanho de pacote inválido: {length}"
        )));
    }

    let mut payload = vec![0_u8; length];

    stream
        .read_exact(&mut payload)
        .map_err(map_read_error)?;

    bincode::deserialize(&payload)
        .map_err(|error| TransportError::Serialization(error.to_string()))
}

fn map_read_error(error: std::io::Error) -> TransportError {
    match error.kind() {
        ErrorKind::UnexpectedEof
        | ErrorKind::ConnectionReset
        | ErrorKind::ConnectionAborted
        | ErrorKind::BrokenPipe
        | ErrorKind::NotConnected => TransportError::Disconnected,

        _ => TransportError::Io(error.to_string()),
    }
}
