# Platformer
A remake of the first level of the delightful 1990 Gamebody platformer [Gargoyle's Quest](https://en.wikipedia.org/wiki/Gargoyle%27s_Quest).

![Screeenshot](README_assets/ggq.gif)

## Dependencies

Most dependencies are managed by `cargo`, but men plan and god laughs, so `cmake` is required to build `shaderc`, and on linux `libudev` is required to build `gilrs`. Probably more, but I developed this on Ubuntu with a ton of dev libs already installed.


```bash
# play the game
cargo run

# start at an arbitrary checkpoint
# checkpoint 0 is level start, 1 is about halfway, and 2 is the boss
cargo run -- -c 1

# play with original gameboy aspect ratio and viewport zoom
cargo run -- --gameboy
```
## Controls
- **A/D** Move left and right
- **W** Jump, hold to jump higher. Press again while in-air to hover briefly.
- **Space** Fire

**Note**: Gamepad input is supported, and *much more fun*.

## TODO:

1. Clippy warnings
2. Update to `wgpu-rs` 0.7
3. Postprocessing for grimy LCD effects

## Why?

To learn [wgpu](https://github.com/gfx-rs/wgpu), and to get more experience in Rust - which is why I didn't use any of the the eminently capable rust game engines out there like [Amethyst](https://amethyst.rs/) or [Bevy](https://bevyengine.org/).