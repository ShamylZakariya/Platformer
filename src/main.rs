use futures::executor::block_on;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder},
};

mod camera;
mod sprite;
mod state;
mod texture;

// ---------------------------------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------------------------------

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut state = block_on(state::State::new(&window));
    let mut last_render_time = std::time::Instant::now();

    event_loop.run(move |event, _, control_flow| match event {
        Event::RedrawRequested(_) => {
            let now = std::time::Instant::now();
            let dt = now - last_render_time;
            last_render_time = now;
            state.update(dt);
            state.render();
        }
        Event::MainEventsCleared => {
            // we have to explicitly request a redraw
            window.request_redraw();
        }
        Event::WindowEvent { window_id, event } if window_id == window.id() => {
            if !state.input(&event) {
                match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::KeyboardInput { input, .. } => match input {
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        } => *control_flow = ControlFlow::Exit,
                        _ => {}
                    },
                    WindowEvent::Resized(physical_size) => {
                        state.resize(physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(*new_inner_size);
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    });
}