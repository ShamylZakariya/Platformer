CURRENTLY:
    Audio
    We probably want some kind of audio controller object, created in AppState and passed down to GameState, GameUi, etc like GpuState
    - It has methods to play single shot sounds (fire, bump, ting, etc) on left/center/right stereo
    - It has methods to play/pause/resume music track
    - It has methods to play an "interrupting" music track (the drawer opening sound, the got a powerup sound)

BUGS:
    - Short flash of scene at game start as palette shift takes an update cycle to apply via game_controller::handle_message
    - Powerups stopped blinking!?

TODO:
    - Postprocessing shader to make Gameboy looking graphics
        - We need a color attachment texture, see  encoder.begin_render_pass in GameState and GameUi, both take the frame color attachment. We can presumably make a texture view like we do for depth, and then make a later pass which does take the frame color attachment which runs a shader transform.
    - Update cargo deps. wgpu-rs 0.7 breaks EVERYTHING.
    - make it possible to instantiate an entity via <object> layer in tmx, instead of using sprites. Because right now we need to create a dedicated sprite for each spawn point, where each specialization specifies the entity to create. We could use object layer info for this more gracefully.
