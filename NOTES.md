CURRENTLY:
    Character Controller
        - doesn't collide with ceilings
        - doesn't handle ratchet collisions


BUGS:
    - At character position 14,8, walking backwards to the NW slope, firebrand jumps forward and up one tile. I suspect firebrand collides with the tile he just stepped off of and since moving backwards he's pushed to that tile's right() - see src/character_controller.rs:239


TODO:
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.

