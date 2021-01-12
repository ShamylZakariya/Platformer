CURRENTLY:
    - implement kickback from contact with spikes, etc
    - kickback vel feels weird (too slow?)

TODO:
    - Store Entities in a HashMap<usize, Box<dyn Entity>
    - Simplify
    - Then start adding enemies?

BUGS:
    - white single-pixel lines between sprites at some offsets, likely do to pixel snapping
        - could outset sprites a half pixel (what's apixel at different scales?)
        - could make each "layer" an indexed mesh
            - this will break texture mapping right?

TODO:
    - sprite.rs is too big - make it a module which re-exports various smaller sub components
        - https://doc.rust-lang.org/book/ch07-04-bringing-paths-into-scope-with-the-use-keyword.html
    - SpriteDesc can have integerial position, and drop extent because we only support 1x1 sprites?
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.
