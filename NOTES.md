CURRENTLY:
    - offsetting Firebrand's z-index to pass through the door doesn't work. Part of problem is that we encode a z-value to all entities at creation time when loading from map. THose Z values are encoded to the actual mesh. Suggest that entity meshes are made with z-value of 0, and use uniforms to offset to correct target depth


TODO:
    - Firebrand must die when sinking in the boss fish water pit
    - We need the fade in, fade out animation. Best way to di it is via postprocessing shader.
        - When firebrand dies we have 3 seconds of the death animation, followed by fade to white over 1 second, 1 second of white, 1 second fade back to normal and blink the ready text.
    - Postprocessing shader to make Gameboy looking graphics
        - We need a color attachment texture, see  encoder.begin_render_pass in GameState and GameUi, both take the frame color attachment. We can presumably make a texture view like we do for depth, and then make a later pass which does take the frame color attachment which runs a shader transform.

BUGS:
    Sometimes the first firebal shot at a Hoodie passes right through him...

LOW PRIORITY:
    - make collision::Space take a "collider" struct instead of sprites, but make a convenience From<> impl to easy convert a sprite to a collider. Drop the static/dynamic difference, have a field on collider which says static or dynamic to optimize lookups, but have all intersection tests run against both.
    - Refactor geom.rs, I don't like that module's existence.
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms. Could make a Uniform<camera::UniformData> or something like that, will need trait constraints for btytemuck::Pod and Zeroable
    - make it possible to instantiate an entity via <object> layer in tmx, instead of using sprites. Because right now we need to create a dedicated sprite for each spawn point, where each specialization specifies the entity to create. We could use object layer info for this more gracefully.
