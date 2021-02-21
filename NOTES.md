CURRENTLY:

TODO:
    - Event::FirebrandStatusChanged should probably just carry a firebrand::CharacterState
    - We need the fade in, fade out animation. Best way to di it is via postprocessing shader.
        - When firebrand dies we have 3 seconds of the death animation, followed by fade to white over 1 second, 1 second of white, 1 second fade back to normal and blink the ready text.
    - Postprocessing shader to make Gameboy looking graphics
        - We need a color attachment texture, see  encoder.begin_render_pass in GameState and GameUi, both take the frame color attachment. We can presumably make a texture view like we do for depth, and then make a later pass which does take the frame color attachment which runs a shader transform.

BUGS:
    - white single-pixel lines between sprites at some offsets, likely do to pixel snapping
        - Outset sprites a half pixel (what's apixel at different scales?)
        - need to outset in the vertex shader, but we will need an extra field on each vertex to clarify the "direction" to outset e.g. (-1,-1) for top left. VS can determine how "far" to extrude based on view matrix. - Will likely need to pass context size?

LOW PRIORITY:
    - Refactor geom.rs, I don't like that module's existence.
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms. Could make a Uniform<camera::UniformData> or something like that, will need trait constraints for btytemuck::Pod and Zeroable
    - make it possible to instantiate an entity via <object> layer in tmx, instead of using sprites. Because right now we need to create a dedicated sprite for each spawn point, where each specialization specifies the entity to create. We could use object layer info for this more gracefully.
