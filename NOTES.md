CURRENTLY:
TODO:
    - Implement game UI
        - Scaling works, but doesn't correctly handle zoom/resize changes - It animates to new position, where it should be immediate. Solution is to animate a value from 0 to 1 for drawer open/shut, and then compute the drawer y from that animation state.
        - Fireballs are drawn atop the drawer, lolol
        - GameStatePeek doesn't have real health/flight info for Firebrand
    - Need "vials", "hearts" and any other power up gubbins
    - Postprocessing shader to make Gameboy looking graphics
        - We need a color attachment texture, see  encoder.begin_render_pass in GameState and GameUi, both take the frame color attachment. We can presumably make a texture view like we do for depth, and then make a later pass which does take the frame color attachment which runs a shader transform.

BUGS:
    - white single-pixel lines between sprites at some offsets, likely do to pixel snapping
        - could outset sprites a half pixel (what's apixel at different scales?)
        - could make each "layer" an indexed mesh
            - this will break texture mapping right?


LOW PRIORITY:
    - Refactor geom.rs, I don't like that module's existence.
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms. Could make a Uniform<camera::UniformData> or something like that, will need trait constraints for btytemuck::Pod and Zeroable
