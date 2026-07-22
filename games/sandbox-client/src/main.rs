use engine_client::ClientRuntime;

use sandbox_server::start_integrated_server;

use sandbox_shared::{ClientMessage, ServerMessage};

use std::{error::Error, thread, time::Duration};

fn main() -> Result<(), Box<dyn Error>> {
    println!("[cliente] iniciando jogo");

    let (transport, server_thread) = start_integrated_server();

    let mut client = ClientRuntime::new(transport);

    client.send(ClientMessage::Join {
        player_name: "Fabio".to_string(),
    })?;

    wait_and_print_messages(&mut client)?;

    println!("[cliente] enviando movimento +1");
    client.send(ClientMessage::Move { delta: 1.0 })?;

    wait_and_print_messages(&mut client)?;

    println!("[cliente] enviando movimento +1");
    client.send(ClientMessage::Move { delta: 1.0 })?;

    wait_and_print_messages(&mut client)?;

    println!("[cliente] tentando movimentar +5000");

    client.send(ClientMessage::Move { delta: 5000.0 })?;

    wait_and_print_messages(&mut client)?;

    println!("[cliente] solicitando desligamento");

    client.send(ClientMessage::Shutdown)?;

    wait_and_print_messages(&mut client)?;

    match server_thread.join() {
        Ok(result) => result?,

        Err(_) => {
            return Err("a thread do servidor integrado entrou em pânico".into());
        }
    }

    println!("[cliente] jogo finalizado");

    Ok(())
}

fn wait_and_print_messages(
    client: &mut ClientRuntime<sandbox_server::SandboxClientTransport>,
) -> Result<(), Box<dyn Error>> {
    /*
     * Temporário.
     *
     * Quando adicionarmos uma janela, a leitura será feita
     * continuamente dentro do loop do cliente.
     */
    thread::sleep(Duration::from_millis(100));

    for message in client.poll()? {
        match message {
            ServerMessage::Welcome {
                client_id,
                player_name,
            } => {
                println!("[cliente] conectado como {player_name}, id={}", client_id.0);
            }

            ServerMessage::PlayerPosition { position } => {
                println!(
                    "[cliente] posição confirmada pelo servidor: \
                     {position}"
                );
            }

            ServerMessage::ServerTick { tick } => {
                println!("[cliente] tick do servidor: {}", tick.0);
            }

            ServerMessage::Stopped => {
                println!("[cliente] servidor desligado");
            }
        }
    }

    Ok(())
}
