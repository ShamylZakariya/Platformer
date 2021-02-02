CURRENTLY:
    - Boss fish
        - can fish be shot again while blinking???
        - what is the fish's hit points?
        - DONE how long does fish flash when injured and what's blink period?
        - DONE camera shake while floor rises
        - DONE how fast does floor rise?
        - DONE does door wait for floor to finish rising?
            no
        - DONE how fast does door open?
            - DONE door opens from left and right, and leaves a smidge visible


TODO:
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
