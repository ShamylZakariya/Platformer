TODO:
    - put camera at 0z and expect all stage contents to be in depth range 0->1,
    and ensure depth clip handles elements at 0 and 1 depths

SPRITES:
    - need texture coords
    - need to be able to rotate/flip; this will affect texture coords as well as collision shape

MAP -> SPRITES:
    - once the above is done, this is little more than a nuanced map() call, but we need to accommodate having background and foreground layers. Can have a single pipeline, but be two SpriteCollections? We only need collision detection for the FG collection.
    - maybe rename it SpriteBatch