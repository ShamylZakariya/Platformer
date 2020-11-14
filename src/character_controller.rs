use std::time::Duration;
use winit::event::*;

// ---------------------------------------------------------------------------------------------------------------------

const CHARACTER_CYCLE_DEFAULT: &str = "default";
const CHARACTER_CYCLE_SHOOT: &str = "shoot";
const CHARACTER_CYCLE_WALK_0: &str = "walk_0";
const CHARACTER_CYCLE_WALK_1: &str = "walk_1";
const CHARACTER_CYCLE_WALK_2: &str = "walk_2";
const CHARACTER_CYCLE_FLY_0: &str = "fly_0";
const CHARACTER_CYCLE_FLY_1: &str = "fly_1";
const CHARACTER_CYCLE_WALL: &str = "wall";

#[derive(Debug)]
pub struct CharacterState {
    pub position: cgmath::Point2<f32>,
    pub cycle: &'static str,
}

impl CharacterState {
    fn new(position: &cgmath::Point2<f32>) -> Self {
        CharacterState {
            position: *position,
            cycle: CHARACTER_CYCLE_DEFAULT,
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
struct InputState {
    move_left_pressed: bool,
    move_right_pressed: bool,
    jump_pressed: bool,
    fire_pressed: bool,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            move_left_pressed: false,
            move_right_pressed: false,
            jump_pressed: false,
            fire_pressed: false,
        }
    }
}

fn input_accumulator(negative: bool, positive: bool) -> f32 {
    return if negative { -1.0 } else { 0.0 } + if positive { 1.0 } else { 0.0 };
}

#[derive(Debug)]
pub struct CharacterController {
    input_state: InputState,
    pub character_state: CharacterState,
}

impl CharacterController {
    pub fn new(position: &cgmath::Point2<f32>) -> Self {
        Self {
            input_state: Default::default(),
            character_state: CharacterState::new(position),
        }
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let pressed = state == ElementState::Pressed;
        match key {
            VirtualKeyCode::W => {
                self.input_state.jump_pressed = pressed;
                true
            }
            VirtualKeyCode::A => {
                self.input_state.move_left_pressed = pressed;
                true
            }
            VirtualKeyCode::D => {
                self.input_state.move_right_pressed = pressed;
                true
            }
            VirtualKeyCode::Space => {
                self.input_state.fire_pressed = pressed;
                true
            }
            _ => false,
        }
    }

    pub fn update(&mut self, dt: Duration) -> &CharacterState {
        let dt = dt.as_secs_f32();
        let delta_position = cgmath::Vector2::new(
            input_accumulator(
                self.input_state.move_left_pressed,
                self.input_state.move_right_pressed,
            ),
            input_accumulator(false, self.input_state.jump_pressed),
        );

        self.character_state.position += delta_position * dt;
        &self.character_state
    }
}
