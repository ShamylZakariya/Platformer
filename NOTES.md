TODO:

CAMERA:
    Switch to ortho projection. First pass didn't go well. I think the camera controller needs to be cleaned up first - it should:
    - only move along x/y
    - be at a fixed Z, such as 0
    - look along +z
    - no pan/tilt/etc

SPRITES:
    - Needs triangular sprites to make slopes. Can make SpriteDesc::triangle