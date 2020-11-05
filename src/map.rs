use anyhow::{Context, Result};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use xml::reader::{EventReader, XmlEvent};

use crate::tileset;

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

#[derive(Debug)]
pub struct Map {
    tileset: tileset::TileSet,
    width: u32,
    height: u32,
    tile_width: u32,
    tile_height: u32,
    layers: Vec<Layer>,
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
        let mut width: Option<u32> = None;
        let mut height: Option<u32> = None;
        let mut tile_width: Option<u32> = None;
        let mut tile_height: Option<u32> = None;
        let mut layers: Vec<Layer> = vec![];
        let mut current_layer: Option<Layer> = None;
        let mut handle_current_layer_data = false;

        for e in parser {
            match e {
                Ok(XmlEvent::StartElement {
                    name, attributes, ..
                }) => match name.local_name.as_str() {
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
                            if attr.name.local_name == "source" {
                                let tileset_path = parent_dir.join(attr.value);
                                tileset =
                                    Some(tileset::TileSet::new_tsx(&tileset_path).with_context(
                                        || {
                                            format!(
                                                "Expected to load referenced <tileset> from {}",
                                                tileset_path.display()
                                            )
                                        },
                                    )?);
                            }
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
                                        "Expected to parse 'id' field of <layer> to u32'",
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
                    _ => {}
                },
                Ok(XmlEvent::Characters(characters)) if handle_current_layer_data => {
                    if let Some(layer) = &mut current_layer {
                        for line in characters.split_whitespace() {
                            for index in line.split(",") {
                                let index = index.trim();
                                if index.len() > 0 {
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
                Ok(XmlEvent::EndElement { name }) => match name.local_name.as_str() {
                    "layer" => {
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
                    _ => {}
                },
                Err(_) => {}
                _ => {}
            }
        }

        // verify all required fields were loaded
        let tileset = tileset.context("Expected to read <tileset> from tmx file.")?;
        let width = width.context("Expected to read width attribute on <map>")?;
        let height = height.context("Expected to read height attribute on <map>")?;
        let tile_width = tile_width.context("Expected to read tile_width attribute on <map>")?;
        let tile_height = tile_height.context("Expected to read tile_height attribute on <map>")?;

        Ok(Map {
            tileset,
            width,
            height,
            tile_width,
            tile_height,
            layers,
        })
    }
}