use anyhow::{Context, Result};
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
}

#[derive(Debug)]
pub struct TileSet {
    pub image_path: String,
    pub tiles: Vec<Tile>,
}

impl TileSet {
    pub fn new_tsx(tsx_file: &Path) -> Result<Self> {
        let file = File::open(tsx_file)
            .with_context(|| format!("Unable to open {}", tsx_file.display()))?;
        let file = BufReader::new(file);
        let parser = EventReader::new(file);

        let mut image_path: Option<String> = None;
        let mut tiles: Vec<Tile> = vec![];
        let mut current_tile: Option<Tile> = None;

        for e in parser {
            match e {
                Ok(XmlEvent::StartElement {
                    name, attributes, ..
                }) => {
                    match name.local_name.as_str() {
                        //
                        // Handle <image> block
                        //
                        "image" => {
                            for attr in attributes {
                                if attr.name.local_name == "source" {
                                    image_path = Some(attr.name.local_name);
                                    break;
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

        Ok(TileSet {
            image_path: image_path.expect("Expected <image> element in tsx file"),
            tiles,
        })
    }
}
