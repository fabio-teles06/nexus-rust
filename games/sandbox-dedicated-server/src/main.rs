use std::{env, error::Error};

use sandbox_server::run_dedicated_server;

fn main() -> Result<(), Box<dyn Error>> {
    let address = parse_bind_address()?;

    println!("Nexus Arena - servidor dedicado");
    println!("Feche esta janela ou use Ctrl+C para encerrar.");
    println!();

    run_dedicated_server(&address)?;

    Ok(())
}

fn parse_bind_address() -> Result<String, Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut address = "0.0.0.0:4000".to_string();

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--bind" => {
                address = args
                    .next()
                    .ok_or("--bind precisa de IP:PORTA")?;
            }

            "--help" | "-h" => {
                println!(
                    "cargo run -p sandbox-dedicated-server -- --bind 0.0.0.0:4000"
                );
                std::process::exit(0);
            }

            other => {
                return Err(
                    format!("argumento desconhecido: {other}").into()
                );
            }
        }
    }

    Ok(address)
}
