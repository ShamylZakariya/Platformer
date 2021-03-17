CURRENTLY:
    - Update cargo deps. wgpu-rs 0.7 breaks EVERYTHING.

BUGS:

TODO:
    - Postprocessing shader to make Gameboy looking graphics
        - We need a color attachment texture, see  encoder.begin_render_pass in GameState and GameUi, both take the frame color attachment. We can presumably make a texture view like we do for depth, and then make a later pass which does take the frame color attachment which runs a shader transform.
    - make it possible to instantiate an entity via <object> layer in tmx, instead of using sprites. Because right now we need to create a dedicated sprite for each spawn point, where each specialization specifies the entity to create. We could use object layer info for this more gracefully.
