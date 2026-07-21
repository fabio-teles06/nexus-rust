use std::{sync::Arc, time::Instant};

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorGrabMode, Window, WindowId},
};

use crate::{
    debug_gui::DebugGui,
    game::{ChunkRenderUpdate, Game},
    input::InputState,
    renderer::Renderer,
};

pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    game: Option<Game>,
    gui: Option<DebugGui>,
    input: InputState,
    last_frame: Instant,
    mouse_captured: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            renderer: None,
            game: None,
            gui: None,
            input: InputState::default(),
            last_frame: Instant::now(),
            mouse_captured: false,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let attributes = Window::default_attributes()
            .with_title("Nexus Voxel Prototype")
            .with_inner_size(LogicalSize::new(1280, 720))
            .with_min_inner_size(LogicalSize::new(800, 450));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                log::error!("Erro ao criar janela: {error}");
                event_loop.exit();
                return;
            }
        };

        let mut renderer = match pollster::block_on(Renderer::new(
            event_loop.owned_display_handle(),
            Arc::clone(&window),
        )) {
            Ok(renderer) => renderer,
            Err(error) => {
                log::error!("{error}");
                event_loop.exit();
                return;
            }
        };

        let size = window.inner_size();
        let mut game = Game::new(size.width, size.height);
        sync_chunks(&mut renderer, &mut game);
        renderer.upload_dynamic_mesh(&game.dynamic_mesh());

        let gui = DebugGui::new(
            &window,
            renderer.device(),
            renderer.surface_format(),
            renderer.max_texture_dimension(),
            game.horizontal_radius(),
            game.vertical_radius(),
        );

        self.mouse_captured = set_cursor_captured(&window, true);
        self.last_frame = Instant::now();
        self.window = Some(Arc::clone(&window));
        self.renderer = Some(renderer);
        self.game = Some(game);
        self.gui = Some(gui);
        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(window) = self.window.as_ref() else {
            return;
        };
        if window_id != window.id() {
            return;
        }

        if let WindowEvent::KeyboardInput { event: key, .. } = &event {
            if key.state == ElementState::Pressed && !key.repeat {
                if key.physical_key == PhysicalKey::Code(KeyCode::F1) {
                    let visible = if let Some(gui) = self.gui.as_mut() {
                        gui.toggle()
                    } else {
                        false
                    };
                    self.input.clear();
                    self.mouse_captured = set_cursor_captured(window, !visible);
                    return;
                }

                if key.physical_key == PhysicalKey::Code(KeyCode::Escape) {
                    if self.gui.as_ref().is_some_and(|gui| gui.visible()) {
                        if let Some(gui) = self.gui.as_mut() {
                            gui.hide();
                        }
                        self.mouse_captured = set_cursor_captured(window, true);
                    } else if self.mouse_captured {
                        self.mouse_captured = set_cursor_captured(window, false);
                        self.input.clear();
                    } else {
                        event_loop.exit();
                    }
                    return;
                }
            }
        }

        if self
            .gui
            .as_mut()
            .is_some_and(|gui| gui.on_window_event(window, &event))
        {
            return;
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if self.gui.as_ref().is_some_and(|gui| gui.visible()) {
                    return;
                }
                if let PhysicalKey::Code(code) = event.physical_key {
                    self.input.process_key(code, event.state);
                }
            }
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } => {
                if !self.mouse_captured
                    && !self.gui.as_ref().is_some_and(|gui| gui.visible())
                {
                    self.mouse_captured = set_cursor_captured(window, true);
                }
            }
            WindowEvent::Focused(false) => {
                self.input.clear();
                self.mouse_captured = set_cursor_captured(window, false);
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = self.renderer.as_mut() {
                    renderer.resize(size);
                }
                if let Some(game) = self.game.as_mut() {
                    game.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => self.redraw(),
            _ => {}
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        let DeviceEvent::MouseMotion { delta } = event else {
            return;
        };

        if self
            .gui
            .as_mut()
            .is_some_and(|gui| gui.on_mouse_motion(delta))
        {
            return;
        }

        if self.mouse_captured {
            self.input.add_mouse_delta(delta);
        }
    }
}

impl App {
    fn redraw(&mut self) {
        let (Some(window), Some(renderer), Some(game), Some(gui)) = (
            self.window.as_ref(),
            self.renderer.as_mut(),
            self.game.as_mut(),
            self.gui.as_mut(),
        ) else {
            return;
        };

        let now = Instant::now();
        let delta_time = now
            .duration_since(self.last_frame)
            .as_secs_f32()
            .clamp(0.0001, 0.05);
        self.last_frame = now;

        if !gui.visible() {
            game.update(&mut self.input, delta_time);
        } else {
            self.input.clear();
            game.update(&mut self.input, delta_time);
        }

        sync_chunks(renderer, game);

        let render_stats = renderer.render_stats();
        let snapshot = game.debug_snapshot(
            delta_time,
            render_stats.visible_chunks,
            render_stats.culled_chunks,
        );
        let (gui_frame, actions) = gui.begin_frame(window, &snapshot);

        game.set_physics_paused(actions.physics_paused);
        game.set_gravity_y(actions.gravity_y);
        if actions.spawn_cube {
            game.spawn_cube_in_front();
        }
        if actions.regenerate_chunks {
            game.regenerate_chunks(actions.horizontal_radius, actions.vertical_radius);
            sync_chunks(renderer, game);
        }
        renderer.upload_dynamic_mesh(&game.dynamic_mesh());

        let camera = game.camera_matrix();
        renderer.render(camera, |device, queue, encoder, view, size| {
            gui.paint(device, queue, encoder, view, size, &gui_frame)
        });

        window.request_redraw();
    }
}

fn sync_chunks(renderer: &mut Renderer, game: &mut Game) {
    for update in game.drain_chunk_updates() {
        match update {
            ChunkRenderUpdate::Upsert(position, mesh) => {
                renderer.upsert_chunk(position, &mesh)
            }
            ChunkRenderUpdate::Remove(position) => renderer.remove_chunk(position),
        }
    }
}

fn set_cursor_captured(window: &Window, captured: bool) -> bool {
    if !captured {
        if let Err(error) = window.set_cursor_grab(CursorGrabMode::None) {
            log::warn!("Não foi possível liberar cursor: {error}");
        }
        window.set_cursor_visible(true);
        return false;
    }

    let result = window
        .set_cursor_grab(CursorGrabMode::Locked)
        .or_else(|_| window.set_cursor_grab(CursorGrabMode::Confined));

    match result {
        Ok(()) => {
            window.set_cursor_visible(false);
            true
        }
        Err(error) => {
            log::warn!("Não foi possível capturar cursor: {error}");
            window.set_cursor_visible(true);
            false
        }
    }
}
