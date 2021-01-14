CURRENTLY:
    - currently, firebrand is "rate limited" in firing. But in game, firebrand can shoot
        rapidly when the fireball hits something close. So we need, instead a new approach:
            - keep rate limiting for upper bound, e.g., if it's been more than 1 second, can shoot
            - if less than a second since last shot, are any fireballs active? Then no shoot.

    - firebrand needs to show "shooot" sprites when firing (what's the timing for that sprite)
        MISSING WALLGRAB SHOOT SPRITE
        DOES FIREBRAND STAND STILL WHILE SHOOTING OR IS THERE A WALK_SHOOT SPRITE


TODO:
    - Then start adding enemies?

BUGS:
    - white single-pixel lines between sprites at some offsets, likely do to pixel snapping
        - could outset sprites a half pixel (what's apixel at different scales?)
        - could make each "layer" an indexed mesh
            - this will break texture mapping right?

LOW PRIORITY TODO:
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.
