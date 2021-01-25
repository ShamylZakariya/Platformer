CURRENTLY:
    - Boss fish can't really get away with being 3x2, it should be made up of a non-rectangular set of sprites, however, sprite::collision::Space will not allow multiple dynamic sprites with same entity_id...

TODO:
    - Boss fish
    - Implement game UI

BUGS:
    - can Firebrand jump in water?
    - white single-pixel lines between sprites at some offsets, likely do to pixel snapping
        - could outset sprites a half pixel (what's apixel at different scales?)
        - could make each "layer" an indexed mesh
            - this will break texture mapping right?

LOW PRIORITY TODO:
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.
