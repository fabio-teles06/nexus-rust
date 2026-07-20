use std::{sync::Arc, time::Instant};

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

use crate::{input::InputState, renderer::Renderer};

pub struct App {
    renderer: Option<Renderer>,

    input: InputState,

    last_frame: Instant,

    mouse_captured: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            renderer: None,
            input: InputState::default(),
            last_frame: Instant::now(),
            mouse_captured: false,
        }
    }
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
                self.mouse_captured = renderer.set_cursor_captured(true);

                self.last_frame = Instant::now();

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
                let PhysicalKey::Code(key_code) = event.physical_key else {
                    return;
                };

                if key_code == KeyCode::Escape
                    && event.state == ElementState::Pressed
                    && !event.repeat
                {
                    if self.mouse_captured {
                        renderer.set_cursor_captured(false);

                        self.mouse_captured = false;
                        self.input.clear();
                    } else {
                        event_loop.exit();
                    }
                }

                self.input.process_key(key_code, event.state);
            }

            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if !self.mouse_captured {
                    self.mouse_captured = renderer.set_cursor_captured(true);

                    self.input.clear();
                    self.last_frame = Instant::now();
                }
            }

            WindowEvent::Focused(false) => {
                self.input.clear();

                renderer.set_cursor_captured(false);

                self.mouse_captured = false;
            }

            WindowEvent::Resized(size) => {
                renderer.resize(size);
            }

            WindowEvent::RedrawRequested => {
                let now = Instant::now();

                let delta_time = now.duration_since(self.last_frame).as_secs_f32().min(0.05);

                self.last_frame = now;

                renderer.update(&mut self.input, delta_time);

                renderer.render();
                renderer.request_redraw();
            }

            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    )
    {
        if !self.mouse_captured {
            return;
        }

        if let DeviceEvent::MouseMotion { delta } = event {
            self.input.add_mouse_delta(delta);
        }
    }
}
