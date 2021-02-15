CURRENTLY:
TODO:
    GameController
        - move MessageDIspatcher to AppState
        - Add GameController
        - Send messages for things like passing a checkpoint (will need a new entity presumably) and death, etc, which are processed by GameController

    - Event::FirebrandStatusChanged should probably just carry a firebrand::CharacterState
    - Postprocessing shader to make Gameboy looking graphics
        - We need a color attachment texture, see  encoder.begin_render_pass in GameState and GameUi, both take the frame color attachment. We can presumably make a texture view like we do for depth, and then make a later pass which does take the frame color attachment which runs a shader transform.

BUGS:
    - white single-pixel lines between sprites at some offsets, likely do to pixel snapping
        - could outset sprites a half pixel (what's apixel at different scales?)

LOW PRIORITY:
    - Refactor geom.rs, I don't like that module's existence.
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms. Could make a Uniform<camera::UniformData> or something like that, will need trait constraints for btytemuck::Pod and Zeroable
