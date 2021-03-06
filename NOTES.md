CURRENTLY:
    YAK SHAVING:
        - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms. Could make a Uniform<camera::UniformData> or something like that, will need trait constraints for btytemuck::Pod and Zeroable
        - Upgrade various cargo deps. Known API breakages using wgpu-rs 0.7, and saw a few more in other modules. So, upgrade them one-at-a-time.

TODO:
    - Fade in and out should be done in sprite shaders since we want to be able to fade out the game, but leave ui (text/etc) unaffected.
        - add a palette lookup texture - 1 * however many colors gameboy had
        - add a palette shift uniform
    - Postprocessing shader to make Gameboy looking graphics
        - We need a color attachment texture, see  encoder.begin_render_pass in GameState and GameUi, both take the frame color attachment. We can presumably make a texture view like we do for depth, and then make a later pass which does take the frame color attachment which runs a shader transform.
    - Update cargo deps. wgpu-rs 0.7 breaks EVERYTHING.

BUGS:

LOW PRIORITY:
    - make it possible to instantiate an entity via <object> layer in tmx, instead of using sprites. Because right now we need to create a dedicated sprite for each spawn point, where each specialization specifies the entity to create. We could use object layer info for this more gracefully.
