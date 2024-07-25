#![allow(dead_code)]

use gilrs::Gilrs;
use state::constants::{ORIGINAL_WINDOW_HEIGHT, ORIGINAL_WINDOW_WIDTH};

use structopt::StructOpt;
use winit::{
    dpi::LogicalSize,
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
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

#[derive(Clone, StructOpt, Debug)]
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

    /// If set, don't simulate gameboy's slow/sludgy pcd pixels
    #[structopt(long)]
    pub no_hysteresis: bool,

    /// Disables music
    #[structopt(short, long)]
    pub no_music: bool,
}

// ---------------------------------------------------------------------------------------------------------------------

fn run_deprecated(opt: Options) {
    let mut gilrs = Gilrs::new().unwrap();
    for (_id, gamepad) in gilrs.gamepads() {
        log::info!("{} is {:?}", gamepad.name(), gamepad.power_info());
    }

    let event_loop = EventLoop::new().unwrap();
    let mut window_attrs = Window::default_attributes()
        .with_title("Gargoyle's Quest")
        .with_decorations(true);

    if opt.gameboy {
        let size = LogicalSize::new(ORIGINAL_WINDOW_WIDTH * 4, ORIGINAL_WINDOW_HEIGHT * 4);
        window_attrs = window_attrs.with_inner_size(size);
    }

    let window = event_loop.create_window(window_attrs).unwrap();
    let mut app_state = state::app_state::AppState::new(&window, opt).unwrap();
    let mut last_render_time = std::time::Instant::now();
    let mut frame_index: u32 = 0;

    let _ = event_loop.run(move |event, control_flow| {
        while let Some(event) = gilrs.next_event() {
            app_state.gamepad_input(event);
        }

        app_state.event(&event);

        match event {
            Event::AboutToWait => {
                // we have to explicitly request a redraw
                app_state.window().request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == app_state.window().id() => {
                if !app_state.input(event) {
                    match event {
                        WindowEvent::RedrawRequested => {
                            let now = std::time::Instant::now();
                            let dt = now - last_render_time;
                            last_render_time = now;

                            app_state.update(now, dt, frame_index);

                            match app_state.gpu.surface.get_current_texture() {
                                Ok(output) => {
                                    let mut encoder = app_state.gpu.device.create_command_encoder(
                                        &wgpu::CommandEncoderDescriptor {
                                            label: Some("Render Encoder"),
                                        },
                                    );
                                    app_state.render(&mut encoder, &output, frame_index as usize);
                                    app_state
                                        .gpu
                                        .queue
                                        .submit(std::iter::once(encoder.finish()));
                                    output.present();

                                    frame_index = frame_index.wrapping_add(1);
                                }
                                Err(wgpu::SurfaceError::Lost) => {
                                    let size = app_state.gpu.size();
                                    app_state.resize(size);
                                }
                                // The system is out of memory, we should probably quit
                                Err(wgpu::SurfaceError::OutOfMemory) => control_flow.exit(),
                                // All other errors (Outdated, Timeout) should be resolved by the next frame
                                Err(e) => log::error!("{:?}", e),
                            }
                        }

                        WindowEvent::CloseRequested => control_flow.exit(),
                        WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    physical_key: PhysicalKey::Code(KeyCode::Escape),
                                    ..
                                },
                            ..
                        } => control_flow.exit(),

                        WindowEvent::Resized(physical_size) => {
                            app_state.resize(*physical_size);
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
    env_logger::init();
    let opt = Options::from_args();

    run_deprecated(opt);
}
