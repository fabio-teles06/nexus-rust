use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::renderer::Renderer;

#[derive(Default)]
pub struct App {
    renderer: Option<Renderer>,
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            return;
        }

        let window_attributes = Window::default_attributes()
            .with_title("Voxel Prototype")
            .with_inner_size(LogicalSize::new(1280, 720));

        let window = match event_loop.create_window(window_attributes) {
            Ok(window) => Arc::new(window),

            Err(error) => {
                eprintln!("Erro ao criar a janela: {error}");
                event_loop.exit();
                return;
            }
        };

        let renderer = pollster::block_on(Renderer::new(
            event_loop.owned_display_handle(),
            Arc::clone(&window),
        ));

        match renderer {
            Ok(renderer) => {
                self.renderer = Some(renderer);
                window.request_redraw();
            }

            Err(error) => {
                eprintln!("Erro ao iniciar o renderer: {error}");
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
        let Some(renderer) = self.renderer.as_mut() else {
            return;
        };

        if window_id != renderer.window_id() {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }

            WindowEvent::KeyboardInput { event, .. } => {
                let escape_pressed =
                    event.state == ElementState::Pressed
                        && event.physical_key
                            == PhysicalKey::Code(KeyCode::Escape);

                if escape_pressed {
                    event_loop.exit();
                }
            }

            WindowEvent::Resized(size) => {
                renderer.resize(size);
            }

            WindowEvent::RedrawRequested => {
                renderer.render();
                renderer.request_redraw();
            }

            _ => {}
        }
    }
}