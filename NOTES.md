CURRENTLY:
    - Boss fish
        - Needs to switch to open-mouth sprites when about to shoot
        - Doesn't stay exactly inside the arena - I've seen it go too far left
        - Does the arena close to keep Firebrand in? Or do we scroll-lock the stage?
        - When dying and the floor rises & door opens, Firebrand needs to be immobilized
        - When passing through door stage needs to fade to white -- this could be considered part of the in-game GUI?

TODO:
    - I don't like how we create EntityComponent in GameState; maybe have the EntityComponent::new method "do the right thing" with Entity to create the right draw components
    - Implement game UI

BUGS:
    - can Firebrand jump in water?
    - white single-pixel lines between sprites at some offsets, likely do to pixel snapping
        - could outset sprites a half pixel (what's apixel at different scales?)
        - could make each "layer" an indexed mesh
            - this will break texture mapping right?

LOW PRIORITY TODO:
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.
