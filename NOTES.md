CURRENTLY:
    Character Controller
        - needs ceiling ratchet collisions
            - the only tiles with ratchet collisions are half-height, with collider geometry being top-half.
        - implement jump dynamics
        - implement kickback from contact with spikes, etc
        - grab onto walls
        - fly (w/ timer)


BUGS:


TODO:
    - sprite.rs is too big - make it a module which re-exports various smaller sub components
    - SpriteDesc can have integerial position, and drop extent because we only support 1x1 sprites
    - Right now we've added + Copy to the iterator trait def for SpriteCollection::draw_sprites, I don't see why this should be necessary.
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.
