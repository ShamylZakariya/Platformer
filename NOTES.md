CURRENTLY:
    - I made sprite::Uniforms hold the camera info. I should instead have camera::Uniforms with the view/proj matrices and sprite::Uniforms hold the color/position vec4s. Then just have one be bound to slot 0 and the other to slot 1
    - sprite vertex shader should snap position to 1/tile_size so we snap to pixels!
    - Add parameter to Map's spritedesc generator to set the depth of the generated sprite. We need the Water sprite
    (which needs alpha cutout, btw) to draw *atop* the Fish sprite, which draws atop the background. So
        - foreground: depth: 0
        - background: depth: 1
        - fish, etc depth : 0.5
    - expand Uniforms struct to have things like offset, alpha, etc. It's general now for all sprite rendering.

BUGS:
    - setting position on firebrand doesn't take effect - he is just at 1 below bottom of level, or 0,0
        - color does take effect, but it applies to both level and firebrand
        - uniforms are UNIFORM constants for a given render pass! So I need a separate pass per entity, or some other way to spread the position info. Perhaps something like the instance buffer... if each entity had some index to index into an array of offsets. But if we only have a few entities on screen at a time, I can just submit a pass per entity.
        - we know color works, so uniform data is submitted. Not certain yet why position uniform doesn't work, but it's likely related to above issues

    - seems to only be drawing root tile?

