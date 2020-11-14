CURRENTLY:
    - add CharacterController which mutates the firebrand sprite
    - need to have shader snap character position to nearest pixel. this should be easy, we know that 1 "unit" is one tile, so just pass in a tile size uniform (e.g., 16x16 pixels) and snap tile offset position to 1/16ths

BUGS:

TODO:
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.

