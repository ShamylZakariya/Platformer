CURRENTLY:
    - I made sprite::Uniforms hold the camera info. I should instead have camera::Uniforms with the view/proj matrices and sprite::Uniforms hold the color/position vec4s. Then just have one be bound to slot 0 and the other to slot 1


    - Add parameter to Map's spritedesc generator to set the depth of the generated sprite. We need the Water sprite
    (which needs alpha cutout, btw) to draw *atop* the Fish sprite, which draws atop the background. So
        - foreground: depth: 0
        - background: depth: 1
        - fish, etc depth : 0.5
    - expand Uniforms struct to have things like offset, alpha, etc. It's general now for all sprite rendering.

TODO:

