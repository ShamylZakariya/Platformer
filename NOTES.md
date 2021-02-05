CURRENTLY:
    - Firebrand has to be able to be injured and die (2 hitpoints)
        - injury response to enemies
        - instadeath if sinking below bottom of level in water
TODO:
    - Implement game UI

BUGS:
    - white single-pixel lines between sprites at some offsets, likely do to pixel snapping
        - could outset sprites a half pixel (what's apixel at different scales?)
        - could make each "layer" an indexed mesh
            - this will break texture mapping right?


LOW PRIORITY:
    - Refactor geom.rs, I don't like that module's existence.
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.
