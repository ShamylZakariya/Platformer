#![allow(dead_code)]

use gilrs::Gilrs;
use state::constants::{ORIGINAL_WINDOW_HEIGHT, ORIGINAL_WINDOW_WIDTH};
use structopt::StructOpt;
use winit::{
    dpi::LogicalSize,
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod audio;
mod camera;
mod collision;
mod entities;
mod entity;
mod event_dispatch;
mod input;
mod map;
mod sprite;
mod state;
mod texture;
mod tileset;
mod util;

// ---------------------------------------------------------------------------------------------------------------------

#[derive(StructOpt, Debug)]
pub struct Options {
    /// Display a debug overlay
    #[structopt(short, long)]
    pub debug_overlay: bool,

    /// Play gargoyle's quest with original gameboy viewport
    #[structopt(short, long)]
    pub gameboy: bool,

    /// Starts gameplay at specified checkpoint
    #[structopt(short, long)]
    pub checkpoint: Option<u32>,

    /// Number of lives to give player
    #[structopt(short, long, default_value = "3")]
    pub lives: u32,

    /// Palette to use; options are "gameboy", "mist", "nostalgia", and "nymph"
    #[structopt(short, long, default_value = "gameboy")]
    pub palette: String,

    /// Disables music
    #[structopt(short, long)]
    pub no_music: bool,
}

// ---------------------------------------------------------------------------------------------------------------------

fn run(opt: Options) {
    let mut builder = WindowBuilder::new()
        .with_title("Gargoyle's Quest")
        .with_decorations(true);

    if opt.gameboy {
        let size = LogicalSize::new(ORIGINAL_WINDOW_WIDTH * 4, ORIGINAL_WINDOW_HEIGHT * 4);
        builder = builder.with_inner_size(size);
    }

    let mut gilrs = Gilrs::new().unwrap();
    for (_id, gamepad) in gilrs.gamepads() {
        println!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }

    let event_loop = EventLoop::new();
    let window = builder.build(&event_loop).unwrap();

    let gpu = pollster::block_on(state::gpu_state::GpuState::new(&window));
    let mut app_state = state::app_state::AppState::new(&window, gpu, opt).unwrap();
    let mut last_render_time = std::time::Instant::now();
    let mut frame_index: u32 = 0;

    event_loop.run(move |event, _, control_flow| {
        control_flow.set_poll();

        while let Some(event) = gilrs.next_event() {
            app_state.gamepad_input(event);
        }

        app_state.event(&window, &event);

        match event {
            Event::RedrawRequested(window_id) if window_id == window.id() => {
                let now = std::time::Instant::now();
                let dt = now - last_render_time;
                last_render_time = now;

                app_state.update(&window, now, dt, frame_index);

                match app_state.gpu.surface.get_current_texture() {
                    Ok(output) => {
                        let mut encoder = app_state.gpu.device.create_command_encoder(
                            &wgpu::CommandEncoderDescriptor {
                                label: Some("Render Encoder"),
                            },
                        );
                        app_state.render(&window, &mut encoder, &output, frame_index);
                        app_state
                            .gpu
                            .queue
                            .submit(std::iter::once(encoder.finish()));
                        output.present();

                        frame_index = frame_index.wrapping_add(1);
                    }
                    Err(wgpu::SurfaceError::Lost) => {
                        let size = app_state.gpu.size();
                        app_state.resize(&window, size);
                    }
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }
            }
            Event::MainEventsCleared => {
                // we have to explicitly request a redraw
                window.request_redraw();
            }
            Event::WindowEvent {
                window_id,
                ref event,
            } if window_id == window.id() => {
                if !app_state.input(&window, event) {
                    match event {
                        WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                        WindowEvent::KeyboardInput {
                            input:
                                KeyboardInput {
                                    state: ElementState::Pressed,
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                            ..
                        } => *control_flow = ControlFlow::Exit,

                        WindowEvent::Resized(physical_size) => {
                            app_state.resize(&window, *physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            app_state.resize(&window, **new_inner_size);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    });
}

fn main() {
    let opt = Options::from_args();
    run(opt);
}
