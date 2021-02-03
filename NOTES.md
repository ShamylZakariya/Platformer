CURRENTLY:

TODO:
    - Looks like Firebrand can jump off the water surface, in fact anywhere inside water can jump
    - Implement game UI

BUGS:
    - weirdly, firebrand is injured jumping into the boss arena, like the spikes extent farther to right than they visually render
    - can Firebrand jump in water?
    - white single-pixel lines between sprites at some offsets, likely do to pixel snapping
        - could outset sprites a half pixel (what's apixel at different scales?)
        - could make each "layer" an indexed mesh
            - this will break texture mapping right?

LOW PRIORITY TODO:
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.
