CURRENTLY:
    YAK SHAVING:
        - make collision::Space take a "collider" struct instead of sprites, but make a convenience From<> impl to easy convert a sprite to a collider. Drop the static/dynamic difference, have a field on collider which says static or dynamic to optimize lookups, but have all intersection tests run against both.
            STEPS:
                - DONE Create Collider struct
                - DONE Update collision::Space to use Collider struct
                - DONE Update all the shit that passes sprites to Space
                - DONE move sprite/collision.rs to ./collision.rs
                - DONE move firebrand's probe() methods to collision
                - make a is_dynamic:bool field on Collider, and have Space sort things out.

            NOTES:
                - this breaks the nice overlapping_sprites visualization, but whatever


        - Refactor geom.rs, I don't like that module's existence.
            - move the line_line and other intersection tests into the collision module
            - move Bounds and lerp, hermite, clamp into a util module
        - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms. Could make a Uniform<camera::UniformData> or something like that, will need trait constraints for btytemuck::Pod and Zeroable

TODO:
    - We need the fade in, fade out animation. Best way to do it is via postprocessing shader.
        - When firebrand dies we have 3 seconds of the death animation, followed by fade to white over 1 second, 1 second of white, 1 second fade back to normal and blink the ready text.
    - Postprocessing shader to make Gameboy looking graphics
        - We need a color attachment texture, see  encoder.begin_render_pass in GameState and GameUi, both take the frame color attachment. We can presumably make a texture view like we do for depth, and then make a later pass which does take the frame color attachment which runs a shader transform.

BUGS:

LOW PRIORITY:
    - make it possible to instantiate an entity via <object> layer in tmx, instead of using sprites. Because right now we need to create a dedicated sprite for each spawn point, where each specialization specifies the entity to create. We could use object layer info for this more gracefully.
