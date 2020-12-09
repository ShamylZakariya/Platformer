CURRENTLY:
    Character Controller
        - needs ceiling ratchet collisions
            - the only tiles with ratchet collisions are half-height, with collider geometry being top-half.


BUGS:
    - find_character_footing only registers collision with tile directly under character, whereas probe() approach collides with up to two tiles left/up/right


TODO:
    - Right now we've added + Copy to the iterator trait def for SpriteCollection::draw_sprites, I don't see why this should be necessary.
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.
