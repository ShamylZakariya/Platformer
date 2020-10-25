TODO:

IMGUI:
    start with displaying camera pos, etc
    - renders, but doesn't do aspect - I need to catch a resize event, or something
    - does not consume input

CAMERA:
    Switch to ortho projection. First pass didn't go well. I think the camera controller needs to be cleaned up first - it should:

SPRITES:
    - Needs triangular sprites to make slopes. Can make SpriteDesc::triangle