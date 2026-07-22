use std::{
    error::Error,
    sync::Arc,
    thread::JoinHandle,
    time::{Duration, Instant},
};

use engine_network::TransportError;
use engine_render::{RenderFrameStatus, Renderer};
use sandbox_server::start_integrated_server;
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

    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,

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
            renderer: None,
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

        /*
         * Solicita ao winit que gere
         * WindowEvent::RedrawRequested.
         */
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn render(&mut self, event_loop: &ActiveEventLoop) {
        let scene = self.client.build_render_scene();

        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

        if let Some(target) = scene.camera_target {
            renderer.set_camera_target(target);
        }

        match renderer.render(&scene.instances) {
            RenderFrameStatus::Presented
            | RenderFrameStatus::Reconfigured
            | RenderFrameStatus::Skipped => {}

            RenderFrameStatus::SurfaceLost => {
                eprintln!("[render] a superfície gráfica foi perdida");

                self.request_shutdown();
                event_loop.exit();
            }

            RenderFrameStatus::ValidationError => {
                eprintln!("[render] erro de validação ao obter o frame");

                self.request_shutdown();
                event_loop.exit();
            }
        }
    }

    fn handle_keyboard(&mut self, event_loop: &ActiveEventLoop, key: KeyCode, state: ElementState) {
        if key == KeyCode::Escape && state == ElementState::Pressed {
            self.request_shutdown();
            event_loop.exit();
            return;
        }

        let changed = self.client.handle_key(key, state);

        if changed {
            if let Err(error) = self.client.send_current_input(Instant::now()) {
                eprintln!("[cliente] erro ao enviar input: {error}");

                self.request_shutdown();
                event_loop.exit();
            }
        }
    }

    fn handle_focus_lost(&mut self, event_loop: &ActiveEventLoop) {
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
            .with_title("Nexus Engine - Sandbox")
            .with_inner_size(LogicalSize::new(1280.0, 720.0));

        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),

            Err(error) => {
                eprintln!("[cliente] erro ao criar janela: {error}");

                self.request_shutdown();
                event_loop.exit();
                return;
            }
        };

        let display_handle = event_loop.owned_display_handle();

        let renderer = match Renderer::new(window.clone(), display_handle) {
            Ok(renderer) => renderer,

            Err(error) => {
                eprintln!("[render] erro ao iniciar renderer: {error}");

                self.request_shutdown();
                event_loop.exit();
                return;
            }
        };

        self.window = Some(window);
        self.renderer = Some(renderer);
        self.next_update = Instant::now();

        println!("[cliente] renderizador iniciado");

        println!("[cliente] use W/A/S/D ou as setas");

        println!("[cliente] pressione ESC para sair");
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

            WindowEvent::Resized(new_size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(new_size);
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
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
                self.render(event_loop);
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
