use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use xml::reader::{EventReader, XmlEvent};

use crate::sprite;

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

    pub fn shape(&self) -> sprite::SpriteShape {
        let collision_shape = self.properties.get("collision_shape");
        if let Some(collision_shape) = collision_shape {
            match collision_shape.as_str() {
                "square" => sprite::SpriteShape::Square,
                "triangle_ne" => sprite::SpriteShape::NorthEast,
                "triangle_se" => sprite::SpriteShape::SouthEast,
                "triangle_sw" => sprite::SpriteShape::SouthWest,
                "triangle_nw" => sprite::SpriteShape::NorthWest,
                _ => sprite::SpriteShape::Square,
            }
        } else {
            sprite::SpriteShape::Square
        }
    }

    pub fn has_property(&self, name: &str) -> bool {
        self.properties.get(name).is_some()
    }
}

#[derive(Debug)]
pub struct TileSet {
    pub image_path: String,
    pub image_width: u32,
    pub image_height: u32,
    pub tile_width: u32,
    pub tile_height: u32,
    pub tiles: Vec<Tile>,
    pub spacing: u32,
    pub columns: u32,
}

impl TileSet {
    pub fn new_tsx(tsx_file: &Path) -> Result<Self> {
        let file = File::open(tsx_file)
            .with_context(|| format!("Unable to open {}", tsx_file.display()))?;
        let file = BufReader::new(file);
        let parser = EventReader::new(file);

        let mut image_path: Option<String> = None;
        let mut image_width: Option<u32> = None;
        let mut image_height: Option<u32> = None;
        let mut tile_width: Option<u32> = None;
        let mut tile_height: Option<u32> = None;
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
                Ok(XmlEvent::EndElement { name }) => match name.local_name.as_str() {
                    //
                    //  Closes the <tile> block by assigning the current_tile
                    //
                    "tile" => {
                        let tile = current_tile
                            .take()
                            .context("Expected to have a valid Tile when reaching </tile>")?;
                        tiles.push(tile);
                    }
                    _ => {}
                },
                Err(_) => {}
                _ => {}
            }
        }

        let image_path = image_path.context("Expected <image> element in tsx file")?;
        let image_width =
            image_width.context("Expected <image> element to have width attribute")?;
        let image_height =
            image_height.context("Expected <image> element to have height attribute")?;
        let tile_width =
            tile_width.context("Expected <image> element to have tilewidth attribute")?;
        let tile_height =
            tile_height.context("Expected <image> element to have tileheight attribute")?;
        let spacing = spacing.context("Expected to read a 'spacing' attribute on <tileset>")?;
        let columns = columns.context("Expected to read a 'columns' attribute on <tileset>")?;

        Ok(TileSet {
            image_path,
            image_width,
            image_height,
            tile_width,
            tile_height,
            tiles,
            spacing,
            columns,
        })
    }

    pub fn tex_coords_for_tile(&self, tile: &Tile) -> (cgmath::Point2<f32>, cgmath::Vector2<f32>) {
        // compute pixel values, and then normalize
        let col = tile.id % self.columns;
        let row = tile.id / self.columns;
        let px_x = col * self.tile_width + col * self.spacing;
        let px_y = self.image_height - ((row + 1) * self.tile_height + row * self.spacing);

        (
            cgmath::Point2::new(
                px_x as f32 / self.image_width as f32,
                px_y as f32 / self.image_height as f32,
            ),
            cgmath::Vector2::new(
                self.tile_width as f32 / self.image_width as f32,
                self.tile_height as f32 / self.image_height as f32,
            ),
        )
    }
}
