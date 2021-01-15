CURRENTLY:
    - cgmath (0.18?) has swizzling. I need to upgrade, can clean up my code a LOT.
    - does cgmath prelude clean things up?
    - 0.18 has point2, point3 functions

    .into()


TODO:
    - Then start adding enemies?

BUGS:
    - I can wallhold on a vertical spike, lol.
        - this happens when firebrand - invulnerable from a previous contact - alights on them.
    - white single-pixel lines between sprites at some offsets, likely do to pixel snapping
        - could outset sprites a half pixel (what's apixel at different scales?)
        - could make each "layer" an indexed mesh
            - this will break texture mapping right?

LOW PRIORITY TODO:
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.
