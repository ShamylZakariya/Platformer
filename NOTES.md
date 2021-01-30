CURRENTLY:
    - Boss fish
        - Needs to switch to open-mouth sprites when about to shootbruta
        - When defeated we need to animate the floor rising up and the door opening (just a fade transition)

TODO:
    - I don't like how we create EntityComponent in GameState; maybe have the EntityComponent::new method "do the right thing" with Entity to create the right draw components
    -
    - Implement game UI

BUGS:
    - can Firebrand jump in water?
    - white single-pixel lines between sprites at some offsets, likely do to pixel snapping
        - could outset sprites a half pixel (what's apixel at different scales?)
        - could make each "layer" an indexed mesh
            - this will break texture mapping right?

LOW PRIORITY TODO:
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.
