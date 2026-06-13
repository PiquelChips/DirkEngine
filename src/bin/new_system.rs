//! Entrypoint for exercising the new plugin/subsystem engine runtime.

use dirkengine::demo::Demo;

use anyhow::Context;
use tracing::error;

fn run() -> anyhow::Result<()> {
    let mut builder = dirk_engine::Engine::builder();

    builder.with_plugin(dirk_assets::AssetsPlugin)?;
    builder.with_plugin(dirk_platform::PlatformPlugin)?;
    builder.with_plugin(dirk_player::PlayerPlugin)?;
    builder.with_plugin(dirk_world::WorldPlugin)?;
    builder.with_plugin(dirk_renderer::RendererPlugin)?;
    builder.add_subsystem(|ctx| {
        Ok(Demo::new(
            ctx.resource::<dirk_player::PlayerRegistry>()?,
            ctx.resource::<dirk_platform::PlatformWindows>()?,
        ))
    });

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
