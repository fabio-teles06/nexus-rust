use anyhow::{anyhow, Result};
use engine_core::ClientId;
use engine_network::TransportError;
use engine_render::{RenderFrameStatus, Renderer};
use sandbox_server::start_integrated_server;
use sandbox_shared::ClientMessage;
use std::{
    sync::Arc,
    thread::JoinHandle,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::client::SandboxClient;

pub(crate) struct SandboxApp {
    client: SandboxClient,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    server_thread: Option<JoinHandle<Result<(), TransportError>>>,
    next_update: Instant,
    shutdown_sent: bool,
}

impl SandboxApp {
    pub fn new() -> Result<Self> {
        let (hub, server_thread) = start_integrated_server();
        let transport = hub.connect(ClientId(1))?;
        let mut client = SandboxClient::new(transport);

        client.send(ClientMessage::Join {
            player_name: "Fabio".into(),
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

    pub fn join_server(&mut self) -> Result<()> {
        self.shutdown();

        if let Some(thread) = self.server_thread.take() {
            match thread.join() {
                Ok(result) => result?,
                Err(_) => return Err(anyhow!("thread do servidor entrou em pânico")),
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
            eprintln!("erro do cliente: {error}");
            self.shutdown();
            event_loop.exit();
            return;
        }

        if !self.client.connected() {
            event_loop.exit();
            return;
        }

        self.next_update = now + Duration::from_millis(8);
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
            RenderFrameStatus::Lost | RenderFrameStatus::ValidationError => {
                self.shutdown();
                event_loop.exit();
            }
        }
    }

    fn shutdown(&mut self) {
        if self.shutdown_sent {
            return;
        }

        self.shutdown_sent = true;
        let _ = self.client.send(ClientMessage::ShutdownServer);
    }
}

impl ApplicationHandler for SandboxApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("Ferrum Engine Advanced")
            .with_inner_size(LogicalSize::new(1280.0, 720.0));

        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                eprintln!("erro ao criar janela: {error}");
                event_loop.exit();
                return;
            }
        };

        match Renderer::new(window.clone(), event_loop.owned_display_handle()) {
            Ok(renderer) => {
                self.renderer = Some(renderer);
                self.window = Some(window);
            }
            Err(error) => {
                eprintln!("erro ao iniciar o renderer: {error}");
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
        if self.window.as_ref().map(|window| window.id()) != Some(window_id) {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                self.shutdown();
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size);
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.repeat {
                    return;
                }

                let PhysicalKey::Code(key) = event.physical_key else {
                    return;
                };

                if key == KeyCode::Escape && event.state == ElementState::Pressed {
                    self.shutdown();
                    event_loop.exit();
                } else {
                    self.client.handle_key(key, event.state);
                }
            }
            WindowEvent::Focused(false) => {
                self.client.clear_input();
            }
            WindowEvent::RedrawRequested => self.render(event_loop),
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        self.update(event_loop);
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_update));
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        self.shutdown();
    }
}
