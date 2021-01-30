use crate::{geom::Bounds, sprite};
use anyhow::{Context, Result};
use cgmath::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use xml::reader::{EventReader, XmlEvent};

#[derive(Clone, Debug)]
pub struct Tile {
    pub id: u32,
    properties: HashMap<String, String>,
}

impl Tile {
    fn new(id: u32) -> Self {
        Tile {
            id,
            properties: HashMap::new(),
        }
    }

    pub fn shape(&self) -> sprite::CollisionShape {
        use sprite::CollisionShape;

        let collision_shape = self.properties.get("collision_shape");
        if let Some(collision_shape) = collision_shape {
            match collision_shape.as_str() {
                "square" => CollisionShape::Square,
                "triangle_ne" => CollisionShape::NorthEast,
                "triangle_se" => CollisionShape::SouthEast,
                "triangle_sw" => CollisionShape::SouthWest,
                "triangle_nw" => CollisionShape::NorthWest,
                _ => CollisionShape::None,
            }
        } else {
            CollisionShape::None
        }
    }

    pub fn has_property(&self, name: &str) -> bool {
        self.properties.get(name).is_some()
    }

    pub fn boolean_property(&self, name: &str) -> bool {
        self.get_property(name) == Some("true")
    }

    pub fn float_property(&self, name: &str) -> f32 {
        self.get_property(name)
            .unwrap_or_else(|| panic!("Expected property\"{}\"", name))
            .parse::<f32>()
            .unwrap_or_else(|_| panic!("Expected property \"{}\" to parse to f32", name))
    }

    pub fn int_property(&self, name: &str) -> i32 {
        self.get_property(name)
            .unwrap_or_else(|| panic!("Expected property\"{}\"", name))
            .parse::<i32>()
            .unwrap_or_else(|_| panic!("Expected property \"{}\" to parse to i32", name))
    }

    pub fn get_property(&self, name: &str) -> Option<&str> {
        self.properties.get(name).map(|p| p.as_str())
    }
}

#[derive(Debug)]
pub struct TileSet {
    pub image_path: String,
    pub image_width: u32,
    pub image_height: u32,
    pub tile_count: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub spacing: u32,
    pub columns: u32,
    tiles: HashMap<u32, Tile>,
}

impl TileSet {
    pub fn new_tsx<P: AsRef<Path>>(tsx_file: P) -> Result<Self> {
        let path_copy = tsx_file.as_ref().to_path_buf();
        let file =
            File::open(tsx_file).with_context(|| format!("Unable to open {:?}", path_copy))?;
        let file = BufReader::new(file);
        let parser = EventReader::new(file);

        let mut image_path: Option<String> = None;
        let mut image_width: Option<u32> = None;
        let mut image_height: Option<u32> = None;
        let mut tile_width: Option<u32> = None;
        let mut tile_height: Option<u32> = None;
        let mut tile_count: Option<u32> = None;
        let mut tiles: Vec<Tile> = vec![];
        let mut current_tile: Option<Tile> = None;
        let mut spacing: Option<u32> = None;
        let mut columns: Option<u32> = None;

        for e in parser {
            match e {
                Ok(XmlEvent::StartElement {
                    name, attributes, ..
                }) => {
                    match name.local_name.as_str() {
                        //
                        // Handle <tileset>
                        //
                        "tileset" => {
                            for attr in attributes {
                                match attr.name.local_name.as_str() {
                                    "spacing" => {
                                        spacing = Some(
                                            attr.value
                                                .parse()
                                                .context("Expected to parse 'spacing' to u32")?,
                                        )
                                    }

                                    "columns" => {
                                        columns = Some(
                                            attr.value
                                                .parse()
                                                .context("Expected to parse 'columns' to u32")?,
                                        )
                                    }

                                    "tilewidth" => {
                                        tile_width = Some(
                                            attr.value
                                                .parse()
                                                .context("Expected to parse 'tilewidth' to u32")?,
                                        )
                                    }
                                    "tileheight" => {
                                        tile_height = Some(
                                            attr.value
                                                .parse()
                                                .context("Expected to parse 'tileheight' to u32")?,
                                        )
                                    }
                                    "tilecount" => {
                                        tile_count = Some(
                                            attr.value
                                                .parse()
                                                .context("Expected to parse 'tilecount' to u32")?,
                                        )
                                    }
                                    _ => {}
                                }
                            }
                        }

                        //
                        // Handle <image> block
                        //
                        "image" => {
                            for attr in attributes {
                                match attr.name.local_name.as_str() {
                                    "source" => image_path = Some(attr.value),
                                    "width" => {
                                        image_width = Some(attr.value.parse().context(
                                            "Expected to parse <image> width attr to u32",
                                        )?)
                                    }
                                    "height" => {
                                        image_height = Some(attr.value.parse().context(
                                            "Expected to parse <image> height attr to u32",
                                        )?)
                                    }
                                    _ => {}
                                }
                            }
                        }

                        //
                        // Handle <tile> block - sets current_tile to be mutated by <property> block
                        //
                        "tile" => {
                            let mut id: Option<u32> = None;
                            for attr in attributes {
                                if attr.name.local_name == "id" {
                                    id = Some(
                                        attr.value
                                            .parse::<u32>()
                                            .context("Expect 'id' attr to parse to u32")?,
                                    );
                                }
                            }
                            let id = id.context("Expect <tile> to have 'id' attr.")?;
                            current_tile = Some(Tile::new(id));
                        }

                        //
                        // Handle <property> block - mutates the current_tile
                        //
                        "property" => {
                            let mut attr_name: Option<String> = None;
                            let mut attr_value: Option<String> = None;
                            for attr in attributes {
                                match attr.name.local_name.as_str() {
                                    "name" => attr_name = Some(attr.value),
                                    "value" => attr_value = Some(attr.value),
                                    _ => {}
                                }
                            }
                            let attr_name = attr_name
                                .context("Expected <property> to have a 'name' attribute")?;
                            let attr_value = attr_value
                                .context("Expected <property> to have a 'value' attribute")?;

                            if let Some(tile) = &mut current_tile {
                                tile.properties.insert(attr_name, attr_value);
                            } else {
                                anyhow::bail!("Expected current_tile to be Some when handling <property> block");
                            }
                        }
                        _ => {}
                    }
                }
                Ok(XmlEvent::EndElement { name }) => {
                    if name.local_name.as_str() == "tile" {
                        //
                        //  Closes the <tile> block by assigning the current_tile
                        //
                        let tile = current_tile
                            .take()
                            .context("Expected to have a valid Tile when reaching </tile>")?;
                        tiles.push(tile);
                    }
                }
                Err(_) => {}
                _ => {}
            }
        }

        let image_path = image_path.context("Expected <image> element in tsx file")?;
        let image_width =
            image_width.context("Expected <image> element to have width attribute")?;
        let image_height =
            image_height.context("Expected <image> element to have height attribute")?;
        let tile_count =
            tile_count.context("Expected <image> element to have tilecount attribute")?;
        let tile_width =
            tile_width.context("Expected <image> element to have tilewidth attribute")?;
        let tile_height =
            tile_height.context("Expected <image> element to have tileheight attribute")?;
        let spacing = spacing.unwrap_or(0);
        let columns = columns.context("Expected to read a 'columns' attribute on <tileset>")?;

        // add tiles with custom properties to the hash, adding empty ones for defaults.
        let mut tiles_map = HashMap::new();
        for tile in tiles {
            tiles_map.insert(tile.id, tile);
        }
        for idx in 0..tile_count {
            tiles_map.entry(idx).or_insert_with(|| Tile::new(idx));
        }

        Ok(TileSet {
            image_path,
            image_width,
            image_height,
            tile_count,
            tile_width,
            tile_height,
            tiles: tiles_map,
            spacing,
            columns,
        })
    }

    /// Returns the size of a sprite tile
    pub fn get_sprite_size(&self) -> Vector2<u32> {
        vec2(self.tile_width, self.tile_height)
    }

    pub fn get_tile(&self, id: u32) -> Option<&Tile> {
        self.tiles.get(&id)
    }

    pub fn get_tiles(&self) -> Vec<&Tile> {
        self.tiles.values().collect()
    }

    pub fn get_tiles_with_property(&self, property_key: &str, property_value: &str) -> Vec<&Tile> {
        self.tiles
            .values()
            .filter(|tile| tile.get_property(property_key) == Some(property_value))
            .collect::<Vec<_>>()
    }

    /// Returns the column and row of the given tile, where (0,0) is the first or top-left tile in the tileset.
    pub fn get_tile_position(&self, tile: &Tile) -> Point2<u32> {
        let col = tile.id % self.columns;
        let row = tile.id / self.columns;
        point2(col, row)
    }

    /// Returns the tile at a given row/col position (where (0,0) is the first or top-left tile in the tileset)
    /// or None if the position is outside the tileset.
    pub fn get_tile_at_position(&self, position: Point2<u32>) -> Option<&Tile> {
        if position.x >= self.columns {
            None
        } else {
            let idx = position.y * self.columns + position.x;
            self.tiles.get(&idx)
        }
    }

    pub fn get_tex_coords_for_tile(&self, tile: &Tile) -> Bounds {
        // compute pixel values, and then normalize
        let col = tile.id % self.columns;
        let row = tile.id / self.columns;
        let px_x = col * self.tile_width + col * self.spacing;
        let px_y = self.image_height - ((row + 1) * self.tile_height + row * self.spacing);

        Bounds::new(
            point2(
                px_x as f32 / self.image_width as f32,
                px_y as f32 / self.image_height as f32,
            ),
            vec2(
                self.tile_width as f32 / self.image_width as f32,
                self.tile_height as f32 / self.image_height as f32,
            ),
        )
    }
}
