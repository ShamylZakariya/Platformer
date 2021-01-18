use cgmath::*;
#[derive(Clone, Debug)]
pub struct DisplayState {
    pub camera_tracks_character: bool,
    pub camera_position: Point3<f32>,
    pub zoom: f32,
    pub character_position: Point2<f32>,
    pub character_cycle: String,
    pub draw_stage_collision_info: bool,
}

impl Default for DisplayState {
    fn default() -> Self {
        DisplayState {
            camera_tracks_character: true,
            camera_position: [0.0, 0.0, 0.0].into(),
            zoom: 1.0,
            character_position: [0.0, 0.0].into(),
            character_cycle: "".to_string(),
            draw_stage_collision_info: true,
        }
    }
}

#[derive(Default)]
pub struct InputState {
    pub camera_tracks_character: Option<bool>,
    pub zoom: Option<f32>,
    pub draw_stage_collision_info: Option<bool>,
    pub draw_entity_debug: Option<bool>,
}

/// To display ImGUI content the client must vend a DisplayState, which will be used to populate the ImGUI context
/// with widgets. After a frame is drawn if any of those widgets generated input (e.g., a slider was dragged) the
/// state of the change is packaged into an InputState, which must be processed to mutate state.
pub trait InputHandler {
    /// Vend a DisplayState mapping current state to something ImGUI can render.
    fn current_display_state(&self) -> DisplayState;
    /// Consume output of the ImGUI render pass to mutate internal state.
    fn process_input(&mut self, input: &InputState);
}
