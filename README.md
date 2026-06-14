# `DirkEngine`

A Vulkan Game Engine in Rust

The engine is assembled with plugins and runtime subsystems:

```rust,no_run
# fn main() -> anyhow::Result<()> {
let mut builder = dirk_engine::Engine::builder();
builder.with_plugin(dirk_assets::AssetsPlugin)?;
builder.with_plugin(dirk_platform::PlatformPlugin)?;
builder.with_plugin(dirk_player::PlayerPlugin)?;
builder.with_plugin(dirk_world::WorldPlugin)?;
builder.with_plugin(dirk_renderer::RendererPlugin)?;

let engine = builder.build()?;
engine.run()?;
# Ok(()) }
```
