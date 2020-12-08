CURRENTLY:
    Character Controller
        - needs to mark all colliders; we often miss spikes inset into walls because the wall immediately above or below triggers response. We can do this by calling collision callback without performing movement adjustment if that's already been applied
        - needs ceiling ratchet collisions
            - the only tiles with ratchet collisions are half-height, with collider geometry being top-half.


BUGS:
    - 27.1,11 - fall left off ledge; while falling push right, when character y == occluder y, eventually weird jump occurs


TODO:
    - Right now we've added + Copy to the iterator trait def for SpriteCollection::draw_sprites, I don't see why this should be necessary.
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.
