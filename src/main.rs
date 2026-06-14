//! Entrypoint for exercising the new plugin/subsystem engine runtime.

use anyhow::Context;
use tracing::error;

fn run() -> anyhow::Result<()> {
    let mut builder = dirk_engine::Engine::builder();

    builder.with_plugin(dirk_engine::signal::OperatingSystemSignalPlugin)?;
    builder.with_plugin(dirkengine::assets::AssetsPlugin)?;
    builder.with_plugin(dirkengine::platform::PlatformPlugin)?;
    builder.with_plugin(dirkengine::player::PlayerPlugin)?;
    builder.with_plugin(dirkengine::world::WorldPlugin)?;
    builder.with_plugin(dirkengine::renderer::RendererPlugin)?;
    builder.with_plugin(dirkengine::demo::DemoPlugin)?;

    let engine = builder.build().context("build new engine")?;

    engine.run().context("run new engine")?;
    Ok(())
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(err) => {
            error!("{err:#}");
            panic!("Error: {err:#}");
        }
    }
}
