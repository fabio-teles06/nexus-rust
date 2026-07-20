mod app;
mod camera;
mod depth;
mod input;
mod mesh;
mod renderer;
mod voxel;

use app::App;
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    env_logger::init();

    let event_loop =
        EventLoop::new().expect("Não foi possível criar o event loop");

    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::default();

    event_loop
        .run_app(&mut app)
        .expect("Erro durante a execução");
}