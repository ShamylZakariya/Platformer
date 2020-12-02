CURRENTLY:
    Character Controller
        - needs ceiling collisions
        - needs ceiling ratchet collisions
            - the only tiles with ratchet collisions are half-height, with collider geometry being top-half.


BUGS:


TODO:
    - Right now we're copying the character_controller HasSet of debug sprites to a vec to draw. This is stupid. Make thed raw call take an iterator or be parameterized on a collection or something.
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.
    - Draw functions which take uniform buffers should actually just take the uniform objects so we're typesafe

