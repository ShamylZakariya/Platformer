use std::collections::HashMap;

use winit::event::{ElementState, VirtualKeyCode};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonState {
    Pressed,
    Down,
    Released,
    Up,
}

impl Default for ButtonState {
    fn default() -> Self {
        ButtonState::Up
    }
}

impl ButtonState {
    fn transition(&self, key_down: bool) -> ButtonState {
        if key_down {
            match self {
                ButtonState::Pressed => ButtonState::Down,
                ButtonState::Down => ButtonState::Down,
                ButtonState::Released => ButtonState::Pressed,
                ButtonState::Up => ButtonState::Pressed,
            }
        } else {
            match self {
                ButtonState::Pressed => ButtonState::Released,
                ButtonState::Down => ButtonState::Released,
                ButtonState::Released => ButtonState::Up,
                ButtonState::Up => ButtonState::Up,
            }
        }
    }

    pub fn is_active(&self) -> bool {
        match self {
            ButtonState::Pressed | ButtonState::Down => true,
            _ => false,
        }
    }
}

#[derive(Debug)]
pub struct InputState {
    buttons: HashMap<VirtualKeyCode, ButtonState>,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            buttons: HashMap::new(),
        }
    }
}

impl InputState {
    pub fn for_keys(keys: &[VirtualKeyCode]) -> Self {
        let mut buttons = HashMap::new();
        for key in keys {
            buttons.insert(*key, ButtonState::default());
        }

        Self { buttons }
    }

    pub fn register(&mut self, key: VirtualKeyCode) {
        self.buttons.insert(key, ButtonState::default());
    }

    pub fn get_button_state(&self, key: VirtualKeyCode) -> Option<&ButtonState> {
        self.buttons.get(&key)
    }

    pub fn process_keyboard(&mut self, key: VirtualKeyCode, state: ElementState) -> bool {
        let pressed = state == ElementState::Pressed;
        if let Some(button_state) = self.buttons.get(&key) {
            let new_state = button_state.transition(pressed);
            self.buttons.insert(key, new_state);
            true
        } else {
            false
        }
    }

    pub fn update(&mut self) {
        let previous_button_state = std::mem::take(&mut self.buttons);
        for (key, button_state) in previous_button_state {
            self.buttons
                .insert(key, button_state.transition(button_state.is_active()));
        }
    }
}

pub fn input_accumulator(negative: &ButtonState, positive: &ButtonState) -> f32 {
    let mut acc = 0.0;
    match negative {
        ButtonState::Pressed | ButtonState::Down | ButtonState::Released => {
            acc -= 1.0;
        }
        ButtonState::Up => {}
    }
    match positive {
        ButtonState::Pressed | ButtonState::Down | ButtonState::Released => {
            acc += 1.0;
        }
        ButtonState::Up => {}
    }

    acc
}
