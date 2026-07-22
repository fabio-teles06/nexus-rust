use std::{
    error::Error,
    thread::JoinHandle,
    time::{Duration, Instant},
};

use engine_network::TransportError;
use sandbox_server::{SandboxClientTransport, start_integrated_server};
use sandbox_shared::ClientMessage;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::client::SandboxClient;

const CLIENT_UPDATE_INTERVAL: Duration = Duration::from_millis(16);

pub(crate) struct SandboxApp {
    client: SandboxClient,
    window: Option<Window>,

    server_thread: Option<JoinHandle<Result<(), TransportError>>>,

    next_update: Instant,
    shutdown_sent: bool,
}

impl SandboxApp {
    pub(crate) fn new() -> Result<Self, TransportError> {
        let (transport, server_thread) = start_integrated_server();

        let mut client = SandboxClient::new(transport);

        client.send(ClientMessage::Join {
            player_name: "Fabio".to_string(),
        })?;

        Ok(Self {
            client,
            window: None,
            server_thread: Some(server_thread),
            next_update: Instant::now(),
            shutdown_sent: false,
        })
    }

    pub(crate) fn join_server(&mut self) -> Result<(), Box<dyn Error>> {
        self.request_shutdown();

        let Some(server_thread) = self.server_thread.take() else {
            return Ok(());
        };

        match server_thread.join() {
            Ok(result) => {
                result?;
            }

            Err(_) => {
                return Err("a thread do servidor entrou em pânico".into());
            }
        }

        Ok(())
    }

    fn update(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();

        if now < self.next_update {
            return;
        }

        if let Err(error) = self.client.update(now) {
            eprintln!("[cliente] erro durante atualização: {error}");

            self.request_shutdown();
            event_loop.exit();
            return;
        }

        if !self.client.connected() {
            event_loop.exit();
            return;
        }

        self.next_update = now + CLIENT_UPDATE_INTERVAL;
    }

    fn handle_keyboard(&mut self, event_loop: &ActiveEventLoop, key: KeyCode, state: ElementState) {
        if key == KeyCode::Escape && state == ElementState::Pressed {
            self.request_shutdown();
            event_loop.exit();
            return;
        }

        let changed = self.client.handle_key(key, state);

        /*
         * Envia imediatamente quando a tecla muda,
         * reduzindo a latência do input.
         */
        if changed {
            if let Err(error) = self.client.send_current_input(Instant::now()) {
                eprintln!("[cliente] erro ao enviar input: {error}");

                self.request_shutdown();
                event_loop.exit();
            }
        }
    }

    fn handle_focus_lost(&mut self, event_loop: &ActiveEventLoop) {
        /*
         * Se o jogador usar Alt+Tab enquanto segura W,
         * talvez o evento Released não seja recebido.
         *
         * Por isso zeramos o input quando a janela
         * perde o foco.
         */
        if !self.client.clear_input() {
            return;
        }

        if let Err(error) = self.client.send_current_input(Instant::now()) {
            eprintln!("[cliente] erro ao limpar input: {error}");

            self.request_shutdown();
            event_loop.exit();
        }
    }

    fn request_shutdown(&mut self) {
        if self.shutdown_sent {
            return;
        }

        self.shutdown_sent = true;

        self.client.clear_input();

        /*
         * Tenta enviar velocidade zero antes do shutdown.
         */
        let _ = self.client.send_current_input(Instant::now());

        let _ = self.client.send(ClientMessage::Shutdown);
    }
}

impl ApplicationHandler for SandboxApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("Ferrum Engine - Sandbox")
            .with_inner_size(LogicalSize::new(960.0, 540.0));

        match event_loop.create_window(attributes) {
            Ok(window) => {
                self.window = Some(window);
                self.next_update = Instant::now();

                println!("[cliente] janela criada");
                println!("[cliente] use W/A/S/D ou as setas");
                println!("[cliente] pressione ESC para sair");
            }

            Err(error) => {
                eprintln!("[cliente] não foi possível criar a janela: {error}");

                self.request_shutdown();
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let correct_window = self.window.as_ref().map(|window| window.id()) == Some(window_id);

        if !correct_window {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                self.request_shutdown();
                event_loop.exit();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                /*
                 * Não precisamos dos eventos de repetição.
                 * Guardamos o estado Pressed até receber Released.
                 */
                if event.repeat {
                    return;
                }

                let PhysicalKey::Code(key_code) = event.physical_key else {
                    return;
                };

                self.handle_keyboard(event_loop, key_code, event.state);
            }

            WindowEvent::Focused(false) => {
                self.handle_focus_lost(event_loop);
            }

            WindowEvent::RedrawRequested => {
                /*
                 * O wgpu será chamado aqui futuramente.
                 */
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.update(event_loop);

        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_update));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.request_shutdown();
    }
}
