use std::env;

#[derive(Debug, Clone)]
pub(crate) enum LaunchMode {
    Integrated,
    Remote(String),
}

#[derive(Debug, Clone)]
pub(crate) struct ClientArgs {
    pub player_name: String,
    pub mode: LaunchMode,
}

impl ClientArgs {
    pub fn parse() -> Result<Option<Self>, String> {
        let mut args = env::args().skip(1);
        let mut player_name = default_player_name();
        let mut mode = LaunchMode::Integrated;

        while let Some(argument) = args.next() {
            match argument.as_str() {
                "--connect" => {
                    let address = args
                        .next()
                        .ok_or("--connect precisa de IP:PORTA")?;

                    mode = LaunchMode::Remote(address);
                }

                "--name" => {
                    player_name = args
                        .next()
                        .ok_or("--name precisa de um nome")?;
                }

                "--help" | "-h" => {
                    print_help();
                    return Ok(None);
                }

                other => {
                    return Err(format!(
                        "argumento desconhecido: {other}\nUse --help."
                    ));
                }
            }
        }

        Ok(Some(Self {
            player_name,
            mode,
        }))
    }
}

fn default_player_name() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "Jogador".to_string())
}

fn print_help() {
    println!("Nexus Arena");
    println!();
    println!("Modo integrado:");
    println!("  cargo run -p sandbox-client -- --name Fabio");
    println!();
    println!("Conectar a um servidor:");
    println!(
        "  cargo run -p sandbox-client -- --connect 192.168.0.10:4000 --name Fabio"
    );
}
