use std::time::Duration;

use anyhow::Result;

use crate::camera;
use crate::sprite;
use crate::tileset;

pub trait Entity {
    fn init(&mut self, sprite: &sprite::SpriteDesc, tile: &tileset::Tile);
    fn update(&mut self, dt: Duration);
    fn is_alive(&self) -> bool;
    fn draw<'a, 'b>(
        &'a self,
        render_pass: &'b mut wgpu::RenderPass<'a>,
        camera_uniforms: &'a camera::Uniforms,
        sprite_uniforms: &'a sprite::Uniforms,
    ) where
        'a: 'b;
}

pub fn instantiate(
    classname: &str,
    sprite: &sprite::SpriteDesc,
    tile: &tileset::Tile,
) -> Result<Box<dyn Entity>> {
    if let Some(mut e) = match classname {
        "FallingBridge" => Some(Box::new(FallingBridge::default())),
        _ => None,
    } {
        e.init(sprite, tile);
        Ok(e)
    } else {
        anyhow::bail!("Unrecognized entity class \"{}\"", classname)
    }
}

// ---------------------------------------------------------------------------------------------------------------------

struct FallingBridge {}

impl Default for FallingBridge {
    fn default() -> Self {
        Self {}
    }
}

impl Entity for FallingBridge {
    fn init(&mut self, sprite: &sprite::SpriteDesc, tile: &tileset::Tile) {
        println!("FallingBridge::init sprite: {:?} tile: {:?}", sprite, tile);
    }

    fn update(&mut self, _dt: Duration) {}

    fn is_alive(&self) -> bool {
        true
    }

    fn draw<'a, 'b>(
        &'a self,
        render_pass: &'b mut wgpu::RenderPass<'a>,
        camera_uniforms: &'a camera::Uniforms,
        sprite_uniforms: &'a sprite::Uniforms,
    ) where
        'a: 'b {

    }
}
