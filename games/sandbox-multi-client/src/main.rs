use anyhow::{Result, anyhow};
use engine_client::ClientRuntime;
use engine_core::ClientId;
use engine_network::ClientTransport;
use sandbox_server::start_integrated_server;
use sandbox_shared::{ClientMessage, PlayerInput, ServerMessage};
use std::{thread, time::Duration};

fn main() -> Result<()> {
    let (hub, server) = start_integrated_server();
    let mut a = ClientRuntime::new(hub.connect(ClientId(1))?);
    let mut b = ClientRuntime::new(hub.connect(ClientId(2))?);
    a.send(ClientMessage::Join {
        player_name: "Alice".into(),
    })?;
    b.send(ClientMessage::Join {
        player_name: "Bob".into(),
    })?;
    for sequence in 1..=90u32 {
        a.send(ClientMessage::Input(PlayerInput {
            sequence,
            direction: [1., 0., 0.],
        }))?;
        b.send(ClientMessage::Input(PlayerInput {
            sequence,
            direction: [-1., 0., 0.],
        }))?;
        drain("Alice", a.transport_mut());
        drain("Bob", b.transport_mut());
        thread::sleep(Duration::from_millis(34));
    }
    a.send(ClientMessage::ShutdownServer)?;
    drop(a);
    drop(b);
    match server.join() {
        Ok(r) => r?,
        Err(_) => return Err(anyhow!("thread do servidor entrou em pânico")),
    }
    Ok(())
}
fn drain(name: &str, transport: &mut impl ClientTransport<Incoming = ServerMessage>) {
    loop {
        match transport.try_receive() {
            Ok(Some(ServerMessage::SnapshotBatch {
                server_tick,
                entities,
            })) => println!(
                "{name}: tick {} recebeu {} entidades",
                server_tick.0,
                entities.len()
            ),
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => break,
        }
    }
}
