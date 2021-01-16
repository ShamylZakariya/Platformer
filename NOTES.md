CURRENTLY:
    - Entity Spawn Point
    Each spawn point has enough info to spawn an enemy, e.g, flying fish, etc
    Spawn point (and all entities) receive became_visible, became_hidden messages
    Each spawned entity knows the id of the spawn point, and sends a "became inactive" message when they die
    If spawn point becomes visible and its entity is not active, it spawns one

    - write tests for rect_rect_intersects

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
