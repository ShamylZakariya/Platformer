# Architecture

> Note: This is not a game engine, but rather a "Level One of Gargoyle's Quest" engine.

The application lives in `AppState`, which owns the following top-level items:

- `MessageDispatcher` : A simple message queue used to pass `Event` objects around
- `GpuState` : Creates and manages the lifecycle of the `wgpu` device, queue, etc
- `GameState` : Represents the game itself, loading the level, creating and updating and drawing `Entity` instances, etc
- `GameUi` : Represents the in-game UI drawer
- `GameController` : A top level game controller handling high level game events
- `LcdFilter` : A post processing pass which renders the LCD effect to the color attachment used by `GameState` and `GameUi`
- `Audio` : Plays soundtracks and sound effects

There is no fancy ECS or anthing here, rather, `AppState` passes a `AppContext` struct to other objects which has mutable references to various resources.

## Entities

The `Entity` trait defines the basic interface for entities to be created at runtime. Entities are enemies, checkpoints, spawn points, ui elements, and so on. Entities have a basic create, update, draw, handle-message lifecycle. All entities are defined in `crate::entities` module.

Entities are either instantiated from map sprites at level loading time, via `crate::entities::instantiate_map_sprite` which is a factory method that looks at sprite metadata to create an `Entity`. Other entities (such as fireballs, death animations, etc) may also be instantiated at runtime via `crate::entities::instantiate_entity_by_class_name`

## Sprites

In `Platformer` a "sprite" is a model object representing a sprite loaded from a map. A `Sprite` represents collision shape, dimensions, collision masks, and so on. A `Sprite` is not renderable in and of itself; instead we create a `crate::sprite::rendering::Mesh` with associated `crate::sprite::rendering::Material` and so on. Generally one or more sprites are instantated from the level, as "templates", and then `Mesh`, `Material` and uniforms are updated at runtime and drawn.

## Collision

`Platformer` does not have a collision dispatch system; rather it has a simple collision "space" - `crate::collision::Collider` are added, updated over time, and queries can be run to determine contacts.

## Level Loading

The level format is a simple imlementation of `tmx`, and the tilesets are of `tsx`, built in the `Tiled` editor. Maps are loaded by `crate::map::Map` and tilesets from `crate::tileset::TileSet`.
