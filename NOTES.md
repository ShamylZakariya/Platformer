CURRENTLY:
    - Postprocessing
    - Draw pixel grid
        - We know that the pixel grid will be pixels_per_unit * viewport_scale vertical lines across, aligned to window edges
        - The horizontal lines are tougher since we maintain square pixels. The projection aspect ratio can be used as a vertical offset. When, for example, the fract(aspect()) == 0.25, the vertical alignment of the pixel grid is off by about 0.25 pixels



BUGS:
    - brief flicker of non-faded scene at startup, just one frame but it's visible

TODO:
    - Postprocessing shader to make Gameboy looking graphics
        - We need a color attachment texture, see  encoder.begin_render_pass in GameState and GameUi, both take the frame color attachment. We can presumably make a texture view like we do for depth, and then make a later pass which does take the frame color attachment which runs a shader transform.
