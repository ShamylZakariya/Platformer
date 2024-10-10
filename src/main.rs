#![allow(dead_code)]

use gilrs::Gilrs;
use state::constants::{ORIGINAL_WINDOW_HEIGHT, ORIGINAL_WINDOW_WIDTH};

use structopt::StructOpt;
use winit::{dpi::LogicalSize, event::*, event_loop::EventLoop, window::Window};

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
    pub no_sludgy: bool,

    /// Disables music
    #[structopt(short, long)]
    pub no_music: bool,
}

// ---------------------------------------------------------------------------------------------------------------------

struct WinitApp {
    app: Option<state::app_state::AppState>,
    gamepad_input: Option<Gilrs>,
    options: Options,
    last_render_time: std::time::Instant,
    frame_index: u32,
}

impl WinitApp {
    fn new(options: Options) -> Self {
        Self {
            app: None,
            gamepad_input: None,
            options,
            last_render_time: std::time::Instant::now(),
            frame_index: 0,
        }
    }
}

impl winit::application::ApplicationHandler for WinitApp {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let mut window_attrs = Window::default_attributes().with_title("Gargoyle's Quest");
        if self.options.gameboy {
            let size = LogicalSize::new(ORIGINAL_WINDOW_WIDTH * 4, ORIGINAL_WINDOW_HEIGHT * 4);
            window_attrs = window_attrs.with_inner_size(size);
        }

        let window = event_loop.create_window(window_attrs).unwrap();

        self.app = Some(state::app_state::AppState::new(window, self.options.clone()).unwrap());

        let gamepad_input = Gilrs::new().unwrap();
        for (_id, gamepad) in gamepad_input.gamepads() {
            log::info!("{} is {:?}", gamepad.name(), gamepad.power_info());
        }
        self.gamepad_input = Some(gamepad_input);
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let app = self.app.as_mut().unwrap();

        while let Some(event) = self.gamepad_input.as_mut().unwrap().next_event() {
            app.gamepad_input(event);
        }

        if app.window().id() == window_id {
            app.event(&event, event_loop);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = self.app.as_ref().unwrap().window();
        window.request_redraw();
    }
}

async fn run(options: Options) {
    let event_loop = EventLoop::new().unwrap();
    let mut app = WinitApp::new(options);
    let _ = event_loop.run_app(&mut app);
}

fn main() {
    env_logger::init();
    let options = Options::from_args();

    pollster::block_on(run(options));
}
