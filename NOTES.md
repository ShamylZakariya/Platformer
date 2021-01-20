CURRENTLY:
    - Enemies

TODO:
    collision::Space::get_static_sprite should return Option<&Sprite> not a copy

BUGS:
    - white single-pixel lines between sprites at some offsets, likely do to pixel snapping
        - could outset sprites a half pixel (what's apixel at different scales?)
        - could make each "layer" an indexed mesh
            - this will break texture mapping right?

LOW PRIORITY TODO:
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.
