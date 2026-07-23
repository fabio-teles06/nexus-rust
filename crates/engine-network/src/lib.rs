use engine_core::ClientId;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, mpsc::{Receiver, SyncSender, TryRecvError, TrySendError, sync_channel}},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TransportError {
    #[error("a fila do transporte está cheia")]
    QueueFull,
    #[error("o transporte foi desconectado")]
    Disconnected,
    #[error("cliente desconhecido: {0:?}")]
    UnknownClient(ClientId),
    #[error("o estado interno do transporte foi envenenado")]
    Poisoned,
}

#[derive(Debug)]
pub enum ConnectionEvent<M> {
    Connected(ClientId),
    Message(ClientId, M),
    Disconnected(ClientId),
}

pub trait ClientTransport: Send {
    type Outgoing: Send + 'static;
    type Incoming: Send + 'static;
    fn send(&mut self, message: Self::Outgoing) -> Result<(), TransportError>;
    fn try_receive(&mut self) -> Result<Option<Self::Incoming>, TransportError>;
    fn client_id(&self) -> ClientId;
}

pub trait ServerTransport: Send {
    type Incoming: Send + 'static;
    type Outgoing: Clone + Send + 'static;
    fn try_receive(&mut self) -> Result<Option<ConnectionEvent<Self::Incoming>>, TransportError>;
    fn send(&mut self, client_id: ClientId, message: Self::Outgoing) -> Result<(), TransportError>;
    fn broadcast(&mut self, message: Self::Outgoing) -> Result<(), TransportError>;
}

pub struct LocalNetworkHub<C, S> {
    events: SyncSender<ConnectionEvent<C>>,
    clients: Arc<Mutex<HashMap<ClientId, SyncSender<S>>>>,
    client_capacity: usize,
}
impl<C, S> Clone for LocalNetworkHub<C, S> {
    fn clone(&self) -> Self { Self { events: self.events.clone(), clients: self.clients.clone(), client_capacity: self.client_capacity } }
}

pub struct LocalClientTransport<C, S> {
    client_id: ClientId,
    events: SyncSender<ConnectionEvent<C>>,
    incoming: Receiver<S>,
    clients: Arc<Mutex<HashMap<ClientId, SyncSender<S>>>>,
}

pub struct LocalServerTransport<C, S> {
    events: Receiver<ConnectionEvent<C>>,
    clients: Arc<Mutex<HashMap<ClientId, SyncSender<S>>>>,
}

pub fn local_network<C, S>(event_capacity: usize, client_capacity: usize) -> (LocalNetworkHub<C, S>, LocalServerTransport<C, S>)
where C: Send + 'static, S: Clone + Send + 'static {
    let (event_tx, event_rx) = sync_channel(event_capacity.max(1));
    let clients = Arc::new(Mutex::new(HashMap::new()));
    (
        LocalNetworkHub { events: event_tx, clients: clients.clone(), client_capacity: client_capacity.max(1) },
        LocalServerTransport { events: event_rx, clients },
    )
}

impl<C, S> LocalNetworkHub<C, S>
where C: Send + 'static, S: Clone + Send + 'static {
    pub fn connect(&self, client_id: ClientId) -> Result<LocalClientTransport<C, S>, TransportError> {
        let (tx, rx) = sync_channel(self.client_capacity);
        self.clients.lock().map_err(|_| TransportError::Poisoned)?.insert(client_id, tx);
        self.events.try_send(ConnectionEvent::Connected(client_id)).map_err(map_send_error)?;
        Ok(LocalClientTransport { client_id, events: self.events.clone(), incoming: rx, clients: self.clients.clone() })
    }
}

impl<C, S> Drop for LocalClientTransport<C, S> {
    fn drop(&mut self) {
        if let Ok(mut clients) = self.clients.lock() { clients.remove(&self.client_id); }
        let _ = self.events.try_send(ConnectionEvent::Disconnected(self.client_id));
    }
}

impl<C, S> ClientTransport for LocalClientTransport<C, S>
where C: Send + 'static, S: Send + 'static {
    type Outgoing = C;
    type Incoming = S;
    fn send(&mut self, message: C) -> Result<(), TransportError> {
        self.events.try_send(ConnectionEvent::Message(self.client_id, message)).map_err(map_send_error)
    }
    fn try_receive(&mut self) -> Result<Option<S>, TransportError> {
        match self.incoming.try_recv() {
            Ok(message) => Ok(Some(message)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(TransportError::Disconnected),
        }
    }
    fn client_id(&self) -> ClientId { self.client_id }
}

impl<C, S> ServerTransport for LocalServerTransport<C, S>
where C: Send + 'static, S: Clone + Send + 'static {
    type Incoming = C;
    type Outgoing = S;
    fn try_receive(&mut self) -> Result<Option<ConnectionEvent<C>>, TransportError> {
        match self.events.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(TransportError::Disconnected),
        }
    }
    fn send(&mut self, client_id: ClientId, message: S) -> Result<(), TransportError> {
        let clients = self.clients.lock().map_err(|_| TransportError::Poisoned)?;
        let sender = clients.get(&client_id).ok_or(TransportError::UnknownClient(client_id))?;
        sender.try_send(message).map_err(map_send_error)
    }
    fn broadcast(&mut self, message: S) -> Result<(), TransportError> {
        let clients = self.clients.lock().map_err(|_| TransportError::Poisoned)?;
        for sender in clients.values() {
            match sender.try_send(message.clone()) {
                Ok(()) | Err(TrySendError::Disconnected(_)) => {}
                Err(TrySendError::Full(_)) => return Err(TransportError::QueueFull),
            }
        }
        Ok(())
    }
}

fn map_send_error<T>(error: TrySendError<T>) -> TransportError {
    match error { TrySendError::Full(_) => TransportError::QueueFull, TrySendError::Disconnected(_) => TransportError::Disconnected }
}
