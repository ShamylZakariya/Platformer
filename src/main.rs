#![allow(dead_code)]

use gilrs::Gilrs;
use state::constants::{ORIGINAL_WINDOW_HEIGHT, ORIGINAL_WINDOW_WIDTH};

use structopt::StructOpt;
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::*,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, KeyCode, NamedKey, PhysicalKey},
    window::{Icon, Window, WindowId},
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

// ---------------------------------------------------------------------------------------------------------------------

struct Application<'window> {
    options: Options,
    close_requested: bool,
    last_render_time: std::time::Instant,
    frame_idx: u64,
    icon: Icon,
    app_state: Option<state::app_state::AppState<'window>>,
    window: Option<Window>,
}

impl<'window> Application<'window> {
    fn new(options: Options) -> Self {
        Self {
            options,
            close_requested: false,
            last_render_time: std::time::Instant::now(),
            frame_idx: 0,
            icon: Self::load_icon(include_bytes!("../res/icon.png")),
            app_state: None,
            window: None,
        }
    }

    fn load_icon(bytes: &[u8]) -> winit::window::Icon {
        let (icon_rgba, icon_width, icon_height) = {
            let image = image::load_from_memory(bytes).unwrap().into_rgba8();
            let (width, height) = image.dimensions();
            let rgba = image.into_raw();
            (rgba, width, height)
        };
        Icon::from_rgba(icon_rgba, icon_width, icon_height).expect("Failed to open icon")
    }
}

impl<'window> ApplicationHandler for Application<'window> {
    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        log::info!("new_events: {cause:?}");
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_title("Gargoyle's Quest")
            .with_window_icon(Some(self.icon.clone()));

        self.window = Some(event_loop.create_window(window_attributes).unwrap());

        let app_state =
            state::app_state::AppState::new(self.window.as_ref().unwrap(), self.options.clone())
                .unwrap();

        // FIXME: This doesn't work, as it causes Application to be self-referential
        // self.app_state.replace(app_state);
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        log::info!("{event:?}");

        if let Some(window) = self.window.as_ref() {
            if window_id != window.id() {
                return;
            }
        } else {
            return;
        }

        match event {
            WindowEvent::CloseRequested => {
                self.close_requested = true;
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        logical_key: key,
                        state: ElementState::Pressed,
                        ..
                    },
                ..
            } => match key.as_ref() {
                Key::Named(NamedKey::Escape) => {
                    self.close_requested = true;
                }
                _ => (),
            },
            WindowEvent::RedrawRequested => {
                let window = self.window.as_ref().unwrap();
                window.pre_present_notify();
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // event_loop.set_control_flow(ControlFlow::Poll);

        if self.close_requested {
            event_loop.exit();
        } else {
            // This SHOULD be enough to keep drawing...
            self.window.as_ref().unwrap().request_redraw();
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------

fn main() {
    env_logger::init();
    let opt = Options::from_args();

    if true {
        run_deprecated(opt);
    } else {
        let event_loop = EventLoop::new().unwrap();
        let _ = event_loop.run_app(&mut Application::new(opt));
    }
}
