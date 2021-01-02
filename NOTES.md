CURRENTLY:
    Falling Tiles
        - need to plumb pixel density in, because the 2px drop on contact is at present hardcoded as 2/16

TODO:
    - Character can be refactored into an Entity impl.
    - implement kickback from contact with spikes, etc
    - Simplify

BUGS:
    - white single-pixel lines between sprites at some offsets, likely do to pixel snapping
        - could outset sprites a half pixel (what's apixel at different scales?)
        - could make each "layer" an indexed mesh
            - this will break texture mapping right?
    - missing animated background. Can make an alternate bg layer with just the flickering fire tiles and show/hide on a timer

TODO:
    - sprite.rs is too big - make it a module which re-exports various smaller sub components
        - https://doc.rust-lang.org/book/ch07-04-bringing-paths-into-scope-with-the-use-keyword.html
    - SpriteDesc can have integerial position, and drop extent because we only support 1x1 sprites?
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.

