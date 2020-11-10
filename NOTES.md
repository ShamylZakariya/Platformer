CURRENTLY:
    - Add parameter to Map's spritedesc generator to set the depth of the generated sprite. We need the Water sprite
    (which needs alpha cutout, btw) to draw *atop* the Fish sprite, which draws atop the background. So
        - foreground: depth: 0
        - background: depth: 1
        - fish, etc depth : 0.5
    - expand Uniforms struct to have things like offset, alpha, etc. It's general now for all sprite rendering.

TODO:

