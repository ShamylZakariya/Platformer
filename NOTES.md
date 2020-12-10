CURRENTLY:
    Character Controller
        - SIMPLICATE, SIMPLICATE, SIMPLICATE
            find_character_footing has code duplication, perhaps we can use lambdas or a loop
        - implement correct gravity speed
        - implement jump dynamics
        - implement kickback from contact with spikes, etc
        - grab onto walls
        - fly (w/ timer)
        - implement water
        - implement correct edge overlap


BUGS:


TODO:
    - sprite.rs is too big - make it a module which re-exports various smaller sub components
        - https://doc.rust-lang.org/book/ch07-04-bringing-paths-into-scope-with-the-use-keyword.html
    - SpriteDesc can have integerial position, and drop extent because we only support 1x1 sprites
    - Right now we've added + Copy to the iterator trait def for SpriteCollection::draw_sprites, I don't see why this should be necessary.
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.
