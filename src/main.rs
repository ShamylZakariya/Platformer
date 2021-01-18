#![allow(dead_code)]

use futures::executor::block_on;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod camera;
mod entities;
mod entity;
mod event_dispatch;
mod gamestate;
mod geom;
mod input;
mod map;
mod sprite;
mod texture;
mod tileset;

// ---------------------------------------------------------------------------------------------------------------------

// ---------------------------------------------------------------------------------------------------------------------

fn main() {
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Gargoyle's Quest")
        .build(&event_loop)
        .unwrap();
    let gpu = block_on(gamestate::gpu_state::GpuState::new(&window));
    let mut state = gamestate::AppState::new(&window, gpu);
    let mut last_render_time = std::time::Instant::now();

    event_loop.run(move |event, _, control_flow| {
        state.event(&window, &event);

        match event {
            Event::RedrawRequested(_) => {
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;
                state.update(&window, dt);
                state.render(&window);
            }
            Event::MainEventsCleared => {
                // we have to explicitly request a redraw
                window.request_redraw();
            }
            Event::WindowEvent { window_id, event } if window_id == window.id() => {
                if !state.input(&window, &event) {
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
                            state.resize(&window, physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            state.resize(&window, *new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    });
}
