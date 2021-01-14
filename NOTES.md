CURRENTLY:
    - fireball needs collision detection against map
    - firebrand needs to show "shooot" sprites when firing (what's the timing for that sprite)
        - is there a shoot while flying? yes?

TODO:
    - Then start adding enemies?

BUGS:
    - white single-pixel lines between sprites at some offsets, likely do to pixel snapping
        - could outset sprites a half pixel (what's apixel at different scales?)
        - could make each "layer" an indexed mesh
            - this will break texture mapping right?

LOW PRIORITY TODO:
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.
