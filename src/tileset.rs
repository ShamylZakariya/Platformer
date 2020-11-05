use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::ops::{Deref, DerefMut};
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
}

impl Deref for Tile {
    type Target = Tile;

    fn deref(&self) -> &Self::Target {
        &self
    }
}

impl DerefMut for Tile {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self
    }
}

#[derive(Debug)]
pub struct TileSet {
    pub image_path: String,
    pub tiles: Vec<Tile>,
}

impl TileSet {
    pub fn new_tsx(spritesheet: &str) -> Result<Self> {
        let file =
            File::open(spritesheet).with_context(|| format!("Unable to open {}", spritesheet))?;
        let file = BufReader::new(file);
        let parser = EventReader::new(file);

        let mut image_path: Option<String> = None;
        let mut tiles: Vec<Tile> = vec![];
        let mut current_tile: Option<Tile> = None;

        for e in parser {
            match e {
                Ok(XmlEvent::StartElement {
                    name, attributes, ..
                }) => match name.local_name.as_str() {
                    "image" => {
                        for attr in attributes {
                            if attr.name.local_name == "source" {
                                image_path = Some(attr.name.local_name);
                                break;
                            }
                        }
                    }
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
                        current_tile =
                            Some(Tile::new(id.context("Expect <tile> to have 'id' attr.")?));
                    }
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
                        let attr_name =
                            attr_name.context("Expected <property> to have a 'name' attribute")?;
                        let attr_value = attr_value
                            .context("Expected <property> to have a 'value' attribute")?;
                        current_tile.as_deref_mut().map(|t| {
                            t.properties.insert(attr_name, attr_value);
                            t
                        });
                    }
                    _ => {}
                },
                Ok(XmlEvent::EndElement { name }) => match name.local_name.as_str() {
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

        Ok(TileSet {
            image_path: image_path.expect("Expected <image> element in tsx file"),
            tiles,
        })
    }
}
