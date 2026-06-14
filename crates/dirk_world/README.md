# world

This crate provides common world components and ECS systems used by the engine.

Register `WorldPlugin` with an `EngineBuilder` to install the world ECS
systems. The plugin depends on `AssetsPlugin` and uses the published
`AssetRegistry` resource to wire model upload systems into the engine universe.
