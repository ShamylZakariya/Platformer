CURRENTLY:
    - implement kickback from contact with spikes, etc
        - kickback is away and up from direction facing, has nothing to do with direction of injury
    - Injury starts at 6:649 -> 7:549 ( duration 0.9 seconds)
        Sprites:
            - 6:649 - shoot_2
            - 6:749 - injured
            - 6:816 - shoot_1
            - 6:883 - injured
            - 6:949 - flight 3 (because falling?)
        Movement:
            - 6:649 to 6:816 traveling diagonal up-back, approx 0.5 sprite distance x&y
            - 6:816 -> on is falling

TODO:
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
