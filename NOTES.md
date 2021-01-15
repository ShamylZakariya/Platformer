CURRENTLY:
    - currently, firebrand is "rate limited" in firing. But in game, firebrand can shoot
        rapidly when the fireball hits something close. So we need, instead a new approach:
            - don't fire until there are no active fireballs less than DISTANCE from firebrand
            - In the game Firebrand could shoot ~1 per sec, so it's FIREBALL_VEL * 1

TODO:
    - traits can have default/overridable implementations. THis can clean up Entity a LOT
    - cgmath (0.18?) has swizzling. I need to upgrade, can clean up my code a LOT.
    - Then start adding enemies?

BUGS:
    - I can wallhold on a vertical spike, lol.
    - white single-pixel lines between sprites at some offsets, likely do to pixel snapping
        - could outset sprites a half pixel (what's apixel at different scales?)
        - could make each "layer" an indexed mesh
            - this will break texture mapping right?

LOW PRIORITY TODO:
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.
