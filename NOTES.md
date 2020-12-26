CURRENTLY:
    Character Controller
        BUGS:
            - wallhold shouldn't kick in at top edge of a ledge.
        TODO:
            - implement correct gravity speed
            - implement kickback from contact with spikes, etc
            - implement water
            - Simplify



BUGS:
    - white single-pixel lines between sprites at some offsets, likely do to pixel snapping
    - missing animated background. Can make an alternate bg layer with just the flickering fire tiles and show/hide on a timer

TODO:
    - State::update_ui_display_state should create an immutable UiDisplayState, not mutate an ivar
    - sprite.rs is too big - make it a module which re-exports various smaller sub components
        - https://doc.rust-lang.org/book/ch07-04-bringing-paths-into-scope-with-the-use-keyword.html
    - SpriteDesc can have integerial position, and drop extent because we only support 1x1 sprites
    - Right now we've added + Copy to the iterator trait def for SpriteCollection::draw_sprites, I don't see why this should be necessary.
    - Uniforms struct can be parameterized on the underlying data...but should it? Right now camera::Uniforms is essentially identical to sprite::Uniforms.

TIMINGS
    - WALK
        - 51:29 start
        - 52:08 2 units
        39/62ths of a second per 2 units
    - FALL_START
        - 2:15:16 start
        - 2:15:42 end
        - 26/62th of a second to get up to falling speed
    - FALL_FINAL
        - 2:14:56 start
        - 2:15:09 end
        16/62ths of a second per 2 units
    - JUMP
        - 0:41:04
        - 0:41:23
    - FLIGHT
        - 0:12:583
        - 0:13:583
        - BOB - 2px cycle on y, sinusoidal
            - 0:13:016
            - 0:13:249
    - WALLGRAB JUMP
        - 0:15:016
        - 0:15:183
        - travel diagonally up and away from wall for .167 seconds, then finish jump upwardly

