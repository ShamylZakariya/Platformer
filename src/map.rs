use anyhow::{Context, Result};
use cgmath::*;
use sprite::core::*;
use std::path::Path;
use std::{collections::HashMap, io::BufReader};
use std::{fs::File, time::Duration};
use xml::reader::{EventReader, XmlEvent};

use crate::entity;
use crate::sprite::{self, collision};
use crate::state::constants::sprite_masks::*;
use crate::tileset;
use crate::{entities, geom::Bounds};

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct Layer {
    pub id: i32,
    pub name: String,
    pub width: u32,  // tiles wide
    pub height: u32, // tiles tall
    pub tile_data: Vec<u32>,
}

impl Default for Layer {
    fn default() -> Self {
        Layer {
            id: -1,
            name: "".to_string(),
            width: 0,
            height: 0,
            tile_data: vec![],
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ObjectGroupProperty {
    pub name: String,
    pub value: String,
}

impl ObjectGroupProperty {
    fn new(name: &str, value: &str) -> Self {
        Self {
            name: name.trim().to_owned(),
            value: value.trim().to_owned(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ObjectGroupObject {
    pub id: i32,
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub properties: Vec<ObjectGroupProperty>,
}

impl Default for ObjectGroupObject {
    fn default() -> Self {
        Self {
            id: -1,
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            properties: vec![],
        }
    }
}

impl ObjectGroupObject {
    pub fn property(&self, named: &str) -> Option<&str> {
        for prop in self.properties.iter() {
            if prop.name == named {
                return Some(&prop.value);
            }
        }
        None
    }
}

#[derive(Clone, Debug)]
pub struct ObjectGroup {
    pub id: i32,
    pub name: String,
    pub objects: Vec<ObjectGroupObject>,
}

impl Default for ObjectGroup {
    fn default() -> Self {
        Self {
            id: -1,
            name: "".to_owned(),
            objects: vec![],
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------

/// Represents a collection of identical sprites sharing a common animation sequence.
/// In the GGQ level 1, this is used only for the animated burning window fire sprites, which
/// are all the same sprite, animated simultaneously through the same keyframes. Animation is
/// represented as a vec of offsets, representing the offset in the texture to apply to index
/// to a particular sprite image.
#[derive(Debug, Clone)]
pub struct SpriteFlipbookAnimation {
    pub sprites: Vec<Sprite>,
    pub name: String,
    pub offsets: Vec<Vector2<f32>>,
    pub durations: Vec<Duration>,
}

impl SpriteFlipbookAnimation {
    fn new(
        name: &str,
        sprite: Sprite,
        sequence: Vec<&tileset::Tile>,
        tileset: &tileset::TileSet,
    ) -> Self {
        let mut offsets = vec![];
        let mut durations = vec![];

        // ensure our frame sequence is in order by "animation_frame" property
        let mut sequence: Vec<&tileset::Tile> = sequence.iter().copied().collect();
        sequence.sort_by(|a, b| {
            let a_frame = a.get_property("animation_frame").expect(
                "Tiles passed to SpriteAnimationSequence must have \"animation_frame\" property",
            ).parse::<i32>()
            .expect("Expect \"animation_frame\" to parse to i32");

            let b_frame = b.get_property("animation_frame").expect(
                "Tiles passed to SpriteAnimationSequence must have \"animation_frame\" property",
            ).parse::<i32>()
            .expect("Expect \"animation_frame\" to parse to i32");

            a_frame.partial_cmp(&b_frame).unwrap()
        });

        let first_tile = sequence
            .first()
            .expect("Animation sequence must not be empty");
        let first_tile_tex_coords = tileset.get_tex_coords_for_tile(&first_tile);

        for tile in sequence {
            let tex_coords = tileset.get_tex_coords_for_tile(tile);
            offsets.push(tex_coords.origin - first_tile_tex_coords.origin);

            let duration = tile.get_property("animation_duration").expect("Tiles passed to SpriteAnimationSequence must have \"animation_duration\" property")
                .parse::<f32>()
                .expect("Expect \"animation_duration\" to parse as f32");
            durations.push(Duration::from_secs_f32(duration));
        }

        Self {
            name: name.to_string(),
            sprites: vec![sprite],
            offsets,
            durations,
        }
    }
}

// ---------------------------------------------------------------------------------------------------------------------

#[derive(Debug)]
pub struct Map {
    pub tileset: tileset::TileSet,
    tileset_first_gid: u32,
    pub width: u32,
    pub height: u32,
    tile_width: u32,
    tile_height: u32,
    pub layers: Vec<Layer>,
    pub object_groups: Vec<ObjectGroup>,
}

impl Map {
    pub fn new_tmx(tmx_file: &Path) -> Result<Self> {
        let parent_dir = tmx_file
            .parent()
            .context("Expect tmx_file to have parent dir")?;
        let file = File::open(tmx_file)
            .with_context(|| format!("Unable to open {}", tmx_file.display()))?;
        let file = BufReader::new(file);
        let parser = EventReader::new(file);

        let mut tileset: Option<tileset::TileSet> = None;
        let mut tileset_first_gid: Option<u32> = None;
        let mut width: Option<u32> = None;
        let mut height: Option<u32> = None;
        let mut tile_width: Option<u32> = None;
        let mut tile_height: Option<u32> = None;
        let mut layers: Vec<Layer> = vec![];
        let mut current_layer: Option<Layer> = None;
        let mut handle_current_layer_data = false;
        let mut object_groups: Vec<ObjectGroup> = vec![];

        for e in parser {
            match e {
                Ok(XmlEvent::StartElement {
                    name, attributes, ..
                }) => {
                    match name.local_name.as_str() {
                        //
                        // Handle the <map> block
                        //
                        "map" => {
                            for attr in attributes {
                                match attr.name.local_name.as_str() {
                                    "width" => {
                                        width = Some(attr.value.parse().context(
                                            "Expected to parse 'width' attr of <map> to u32.",
                                        )?)
                                    }
                                    "height" => {
                                        height = Some(attr.value.parse().context(
                                            "Expected to parse 'height' attr of <map> to u32.",
                                        )?)
                                    }
                                    "tilewidth" => {
                                        tile_width = Some(attr.value.parse().context(
                                            "Expected to parse 'tilewidth' attr of <map> to u32.",
                                        )?)
                                    }
                                    "tileheight" => {
                                        tile_height = Some(attr.value.parse().context(
                                            "Expected to parse 'tileheight' attr of <map> to u32.",
                                        )?)
                                    }
                                    _ => {}
                                }
                            }
                        }

                        //
                        // Handle the <tileset> block
                        //
                        "tileset" => {
                            for attr in attributes {
                                match attr.name.local_name.as_str() {
                                    "source" => {
                                        let tileset_path = parent_dir.join(attr.value);
                                        tileset = Some(
                                            tileset::TileSet::new_tsx(&tileset_path).with_context(
                                                || {
                                                    format!(
                                                    "Expected to load referenced <tileset> from {}",
                                                    tileset_path.display()
                                                )
                                                },
                                            )?,
                                        );
                                    }
                                    "firstgid" => {
                                        tileset_first_gid = Some(attr.value.parse().context(
                                            "Expected to parse <tileset> 'firstgid' to u32",
                                        )?);
                                    }
                                    _ => {}
                                }
                                if attr.name.local_name == "source" {}
                            }
                        }

                        //
                        // Handle the <layer> block - assigns current_layer
                        //
                        "layer" => {
                            let mut layer = Layer::default();
                            for attr in attributes {
                                match attr.name.local_name.as_str() {
                                    "id" => {
                                        layer.id = attr.value.parse().context(
                                            "Expected to parse 'id' field of <layer> to i32'",
                                        )?
                                    }
                                    "width" => {
                                        layer.width = attr.value.parse().context(
                                            "Expected to parse 'width' field of <layer> to u32'",
                                        )?
                                    }
                                    "height" => {
                                        layer.height = attr.value.parse().context(
                                            "Expected to parse 'height' field of <layer> to u32'",
                                        )?
                                    }
                                    "name" => {
                                        layer.name = attr.value;
                                    }
                                    _ => {}
                                }
                            }
                            // verify required fields were read
                            if layer.id == -1 {
                                anyhow::bail!("<layer> element missing an 'id' attribute.");
                            }
                            if layer.width == 0 {
                                anyhow::bail!("<layer> element missing a 'width' attribute.");
                            }
                            if layer.height == 0 {
                                anyhow::bail!("<layer> element missing a 'height' attribute.");
                            }
                            current_layer = Some(layer);
                        }

                        //
                        // Handle the <data> block - requires that current_layer is Some
                        //
                        "data" => {
                            handle_current_layer_data = false;
                            for attr in attributes {
                                if attr.name.local_name == "encoding" && attr.value == "csv" {
                                    handle_current_layer_data = true;
                                }
                            }
                            if !handle_current_layer_data {
                                anyhow::bail!("Only supported encoding for <data> block is 'csv'");
                            }
                        }

                        //
                        //  Handle <objectgroup>, and nested <object>, <properties> and <property> elements
                        //
                        "objectgroup" => {
                            let mut object_group = ObjectGroup::default();
                            for attr in attributes {
                                match attr.name.local_name.as_str() {
                                    "id" => {
                                        object_group.id = attr.value.parse().context(
                                            "Expected to parse 'id' field of <objectgroup> to i32",
                                        )?
                                    }
                                    "name" => object_group.name = attr.value,
                                    _ => {}
                                }
                            }
                            // verify required fields
                            if object_group.id == -1 {
                                anyhow::bail!("<objectgroup> element missing an 'id' attribute");
                            }
                            if object_group.name.is_empty() {
                                anyhow::bail!("<objectgroup> element missing a 'name' attribute");
                            }

                            object_groups.push(object_group);
                        }

                        "object" => {
                            let mut object = ObjectGroupObject::default();
                            for attr in attributes {
                                match attr.name.local_name.as_str() {
                                    "id" => {
                                        object.id = attr.value.parse().context(
                                            "Expected to parse 'id' field of <object> to i32",
                                        )?
                                    }
                                    "x" => {
                                        object.x = attr.value.parse().context(
                                            "Expected to parse 'x' field of <object> to i32",
                                        )?
                                    }
                                    "y" => {
                                        object.y = attr.value.parse().context(
                                            "Expected to parse 'y' field of <object> to i32",
                                        )?
                                    }
                                    "width" => {
                                        object.width = attr.value.parse().context(
                                            "Expected to parse 'width' field of <object> to u32",
                                        )?
                                    }
                                    "height" => {
                                        object.height = attr.value.parse().context(
                                            "Expected to parse 'height' field of <object> to u32",
                                        )?
                                    }
                                    _ => {}
                                }
                            }
                            // verify required fields
                            if object.id == -1 {
                                anyhow::bail!("<object> element missing an 'id' attribute");
                            }
                            if object.width == 0 {
                                anyhow::bail!("<object> element missing a 'width' attribute");
                            }
                            if object.height == 0 {
                                anyhow::bail!("<object> element missing a 'height' attribute");
                            }

                            object_groups
                                .last_mut()
                                .context("Expect <object> to be nested in <objectgroup>")?
                                .objects
                                .push(object);
                        }

                        "property" => {
                            let mut name: Option<String> = None;
                            let mut value: Option<String> = None;

                            for attr in attributes {
                                match attr.name.local_name.as_str() {
                                    "name" => {
                                        name = Some(attr.value);
                                    }
                                    "value" => {
                                        value = Some(attr.value);
                                    }
                                    _ => {}
                                }
                            }

                            let property = ObjectGroupProperty::new(
                                &name.context(
                                    "Expect <property> element to have a 'name' attribute",
                                )?,
                                &value.context(
                                    "Expect <property> element to have a 'value' attribute",
                                )?,
                            );

                            object_groups
                                .last_mut()
                                .context("Expect <object> to be nested in <objectgroup>")?
                                .objects
                                .last_mut()
                                .context(
                                    "Expect <property> element to be nested in an <object> element",
                                )?
                                .properties
                                .push(property);
                        }

                        _ => {}
                    }
                }
                Ok(XmlEvent::Characters(characters)) if handle_current_layer_data => {
                    if let Some(layer) = &mut current_layer {
                        for line in characters.split_whitespace() {
                            for index in line.split(',') {
                                let index = index.trim();
                                if !index.is_empty() {
                                    let index = index.parse::<u32>().with_context(|| {
                                        format!("Expected to parse '{}' to u32", index)
                                    })?;
                                    layer.tile_data.push(index);
                                }
                            }
                        }
                    } else {
                        anyhow::bail!(
                            "Entered a <data> character section without having an active current_layer."
                        );
                    }
                }
                Ok(XmlEvent::EndElement { name }) => {
                    if name.local_name.as_str() == "layer" {
                        let layer = current_layer.take().context("Expected current_layer to have been populated when finishing <layer> block.")?;
                        let expected_count = layer.width as usize * layer.height as usize;
                        if layer.tile_data.len() != expected_count {
                            anyhow::bail!(
                                "Expected layer tile_data to have {} entries, but got {}",
                                expected_count,
                                layer.tile_data.len()
                            );
                        }
                        layers.push(layer);
                    }
                }
                Err(_) => {}
                _ => {}
            }
        }

        // verify all required fields were loaded
        let tileset = tileset.context("Expected to read <tileset> from tmx file.")?;
        let tileset_first_gid =
            tileset_first_gid.context("Expected to read 'firstgid' attr on <tileset> block")?;
        let width = width.context("Expected to read width attribute on <map>")?;
        let height = height.context("Expected to read height attribute on <map>")?;
        let tile_width = tile_width.context("Expected to read tile_width attribute on <map>")?;
        let tile_height = tile_height.context("Expected to read tile_height attribute on <map>")?;

        Ok(Map {
            tileset,
            tileset_first_gid,
            width,
            height,
            tile_width,
            tile_height,
            layers,
            object_groups,
        })
    }

    /// Returns bounds of map as tuple of (origin,extent)
    pub fn bounds(&self) -> Bounds {
        Bounds::new(
            point2(0.0, 0.0),
            vec2(self.width as f32, self.height as f32),
        )
    }

    /// Returns the layer by the provided name, or None if not found
    pub fn layer_named(&self, name: &str) -> Option<&Layer> {
        for layer in &self.layers {
            if layer.name == name {
                return Some(layer);
            }
        }
        None
    }

    /// Returns the objectgroup by the provided name, or None if not found
    pub fn object_group_named(&self, name: &str) -> Option<&ObjectGroup> {
        for object_group in &self.object_groups {
            if object_group.name == name {
                return Some(object_group);
            }
        }
        None
    }

    /// Finds the ObjectGroup object in the provided layer occupying the same position and extent on
    /// the map as the provided sprite.
    pub fn object_group_properties_for_sprite(
        &self,
        sprite: &Sprite,
        object_group_layer: &str,
    ) -> Option<&ObjectGroupObject> {
        if let Some(object_group) = self.object_group_named(object_group_layer) {
            let x = sprite.origin.x.floor() as i32 * self.tile_width as i32;
            let y = (self.height * self.tile_height) as i32
                - (sprite.origin.y.floor() as i32 * self.tile_height as i32);
            let width = sprite.extent.x.floor() as u32 * self.tile_width;
            let height = sprite.extent.y.floor() as u32 * self.tile_height;

            for obj in object_group.objects.iter() {
                if obj.x == x && obj.y == y && obj.width == width && obj.height == height {
                    return Some(obj);
                }
            }

            None
        } else {
            None
        }
    }

    /// Returns a vector of all animated sprite names
    pub fn generate_animations<Z>(&self, layer: &Layer, z_depth: Z) -> Vec<SpriteFlipbookAnimation>
    where
        Z: Fn(&Sprite, &tileset::Tile) -> f32,
    {
        let mut animations_by_name: HashMap<String, SpriteFlipbookAnimation> = HashMap::new();

        self.generate(
            layer,
            |_, _| 0, // sprites always have entity_id of zero
            z_depth,
            |sprite, tile| {
                if sprite.mask & ENTITY == 0 {
                    if let Some(animation_name) = tile.get_property("animation") {
                        if !animations_by_name.contains_key(animation_name) {
                            // only generate the animation once, because all sprites with this animation name will
                            // share the same animation sequence
                            let animation_sequence = self
                                .tileset
                                .get_tiles_with_property("animation", animation_name);

                            animations_by_name.insert(
                                animation_name.to_string(),
                                SpriteFlipbookAnimation::new(
                                    animation_name,
                                    *sprite,
                                    animation_sequence,
                                    &self.tileset,
                                ),
                            );
                        } else if let Some(animation) = animations_by_name.get_mut(animation_name) {
                            animation.sprites.push(*sprite);
                        }
                    }
                }
            },
        );

        let mut animations: Vec<SpriteFlipbookAnimation> = vec![];
        for v in animations_by_name.values() {
            animations.push(v.clone());
        }
        animations
    }

    /// Generates a vector of Sprite for the contents of the specified layer
    pub fn generate_sprites<Z>(&self, layer: &Layer, z_depth: Z) -> Vec<Sprite>
    where
        Z: Fn(&Sprite, &tileset::Tile) -> f32,
    {
        let mut sprites: Vec<Sprite> = vec![];

        self.generate(
            layer,
            |_, _| 0, // sprites always have entity_id of zero
            z_depth,
            |sprite, tile| {
                if sprite.mask & ENTITY == 0 && !tile.has_property("animation") {
                    sprites.push(*sprite);
                }
            },
        );

        sprites
    }

    pub fn generate_entities<Z>(
        &self,
        layer: &Layer,
        collision_space: &mut collision::Space,
        entity_id_vendor: &mut entity::IdVendor,
        z_depth: Z,
    ) -> Vec<Box<dyn entity::Entity>>
    where
        Z: Fn(&Sprite, &tileset::Tile) -> f32,
    {
        let mut entities: Vec<Box<dyn entity::Entity>> = vec![];

        self.generate(
            layer,
            |_, _| entity_id_vendor.next_id(),
            z_depth,
            |sprite, tile| {
                if let Some(name) = tile.get_property("entity_class") {
                    let entity = entities::instantiate_map_sprite(
                        name,
                        sprite,
                        tile,
                        self,
                        collision_space,
                        None,
                    )
                    .unwrap_or_else(|_| {
                        panic!("Unable to instantiate Entity with class name \"{}\"", name)
                    });
                    entities.push(entity);
                }
            },
        );

        entities
    }

    fn generate<Z, C, E>(&self, layer: &Layer, mut entity_id_vendor: E, z_depth: Z, mut consumer: C)
    where
        Z: Fn(&Sprite, &tileset::Tile) -> f32,
        C: FnMut(&Sprite, &tileset::Tile),
        E: FnMut(&Sprite, &tileset::Tile) -> u32,
    {
        // https://doc.mapeditor.org/en/stable/reference/tmx-map-format/#tile-flipping
        let flipped_horizontally_flag = 0x80000000;
        let flipped_vertically_flag = 0x40000000;
        let flipped_diagonally_flag = 0x20000000;

        for y in 0..layer.height {
            for x in 0..layer.width {
                let index: usize = (y * layer.width + x) as usize;
                let tile_id = layer.tile_data[index];
                let flipped_horizontally = tile_id & flipped_horizontally_flag != 0;
                let flipped_vertically = tile_id & flipped_vertically_flag != 0;
                let flipped_diagonally = tile_id & flipped_diagonally_flag != 0;
                let tile_id = tile_id
                    & !(flipped_diagonally_flag
                        | flipped_vertically_flag
                        | flipped_horizontally_flag);

                if self.tileset_first_gid <= tile_id
                    && tile_id - self.tileset_first_gid < self.tileset.tile_count
                {
                    let tile = &self
                        .tileset
                        .get_tile(tile_id - self.tileset_first_gid)
                        .unwrap();
                    let tex_coord_bounds = self.tileset.get_tex_coords_for_tile(tile);
                    let mut mask = 0;

                    if tile.has_property("collision_shape") {
                        mask |= COLLIDER;
                    }
                    if tile.boolean_property("water") {
                        mask |= WATER;
                    }
                    if tile.boolean_property("ratchet") {
                        mask |= RATCHET;
                    }
                    if tile.has_property("entity_class") {
                        mask |= ENTITY;
                    }
                    if tile.boolean_property("contact_damage") {
                        mask |= CONTACT_DAMAGE;
                    }
                    if tile.boolean_property("shootable") {
                        mask |= SHOOTABLE;
                    }

                    let mut sd = Sprite::unit(
                        tile.shape(),
                        point2(x as i32, (layer.height - y) as i32),
                        0.0,
                        tex_coord_bounds.origin,
                        tex_coord_bounds.extent,
                        vec4(1.0, 1.0, 1.0, 1.0),
                        mask,
                    );

                    if mask & ENTITY != 0 {
                        sd.entity_id = Some(entity_id_vendor(&sd, tile));
                    }

                    sd.origin.z = z_depth(&sd, tile);

                    if flipped_diagonally {
                        sd = sd.flipped_diagonally();
                    }

                    if flipped_horizontally {
                        sd = sd.flipped_horizontally();
                    }

                    if flipped_vertically {
                        sd = sd.flipped_vertically();
                    }

                    consumer(&sd, tile);
                }
            }
        }
    }
}
