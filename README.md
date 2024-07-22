# Platformer
A remake of the first level of the delightful 1990 Gamebody platformer [Gargoyle's Quest](https://en.wikipedia.org/wiki/Gargoyle%27s_Quest).

https://github.com/user-attachments/assets/6b2ec470-99ec-4767-b534-59aec8785461

## Dependencies

Most dependencies are managed by `cargo`, but men plan and god laughs; on linux `libudev` is required to build `gilrs`. Probably more, but I developed this on Ubuntu and Fedora with a ton of dev libs already installed.


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
- **F1** Pause
- **Esc** Quit

**Note**: Gamepad input is supported, and *much more fun*.

## Why?

To learn [wgpu](https://github.com/gfx-rs/wgpu), and to get more experience in Rust - which is why I didn't use any of the the eminently capable rust game engines out there such as [Bevy](https://bevyengine.org/).

## Architecture

See [Architecture](ARCH.md)

## TODO

1. Continuously update to whatever is current `wgpu`
2. Ensure current sprite pipeline is better named to make clear it's for rendering individual quads, even if in a batch of thousands
3. Implement single-quad stage rendering using a sprite table texture which indexes into the spritemap.
