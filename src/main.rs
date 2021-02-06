#![allow(dead_code)]

use futures::executor::block_on;
use state::constants::{ORIGINAL_WINDOW_HEIGHT, ORIGINAL_WINDOW_WIDTH};
use structopt::StructOpt;
use winit::{
    dpi::LogicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod camera;
mod entities;
mod entity;
mod event_dispatch;
mod geom;
mod input;
mod map;
mod sprite;
mod state;
mod texture;
mod tileset;

// ---------------------------------------------------------------------------------------------------------------------

#[derive(StructOpt, Debug)]
pub struct Options {
    /// Display a debug overlay
    #[structopt(short, long)]
    pub debug_overlay: bool,

    /// Play gargoyle's quest with original gameboy viewport
    #[structopt(short, long)]
    pub gameboy: bool,
}

// ---------------------------------------------------------------------------------------------------------------------

fn main() {
    let opt = Options::from_args();
    let event_loop = EventLoop::new();

    let mut builder = WindowBuilder::new().with_title("Gargoyle's Quest");
    if opt.gameboy {
        let size = LogicalSize::new(ORIGINAL_WINDOW_WIDTH * 4, ORIGINAL_WINDOW_HEIGHT * 4);
        builder = builder.with_inner_size(size);
    }

    let window = builder.build(&event_loop).unwrap();

    let gpu = block_on(state::gpu_state::GpuState::new(&window));
    let mut state = state::app_state::AppState::new(&window, gpu, opt);
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
                        WindowEvent::KeyboardInput { input, .. } => {
                            if let KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            } = input
                            {
                                *control_flow = ControlFlow::Exit
                            }
                        }
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
