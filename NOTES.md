CURRENTLY:
    FireSprite
        - needs to do the left/right/left/right march
        - needs to die when shot twice


TODO:
    - Refactor State.rs into components, e.g. RenderState, EntityState, etc
    - CameraController ought to own the camera?
    - Enemies

BUGS:
    - white single-pixel lines between sprites at some offsets, likely do to pixel snapping
        - could outset sprites a half pixel (what's apixel at different scales?)
        - could make each "layer" an indexed mesh
            - this will break texture mapping right?

LOW PRIORITY TODO:
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.
