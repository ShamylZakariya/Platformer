TODO:
    - put camera at 0z and expect all stage contents to be in depth range 0->1,
    and ensure depth clip handles elements at 0 and 1 depths

SPRITES:
    - needs world space collision detection
    - if point testing is in SpriteMesh, we can't use rust's tests to verify because of dep on wgpu
    - make a SpriteHitTest struct?
