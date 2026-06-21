//! This crate contains the CLI entrypoint & management for the enhanced clap
//! engine CLI.

use anyhow::Context;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "dirkengine")]
#[command(about = "DirkEngine CLI")]
struct Cli {
    /// Enable debug logging.
    #[arg(short = 'v', long = "verbose", global = true)]
    verbose: bool,

    /// Disable the demo world.
    #[arg(long = "no-demo", global = true)]
    no_demo: bool,
}

/// Runs the enhanced [`clap`] CLI.
///
/// # Errors
///
/// Returns any error that occurs while running the engine.
pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut builder = dirk_engine::Engine::builder();

    if cli.verbose {
        builder.with_log_level(piquel_log::LogLevel::Trace);
    }

    #[cfg(feature = "editor")]
    builder.with_plugin(dirk_editor::EditorPlugin)?;
    builder.with_plugin(dirk_assets::AssetsPlugin)?;
    builder.with_plugin(dirk_platform::PlatformPlugin)?;
    builder.with_plugin(dirk_player::PlayerPlugin)?;
    builder.with_plugin(dirk_world::WorldPlugin)?;
    builder.with_plugin(dirk_renderer::RendererPlugin)?;

    if !cli.no_demo {
        builder.with_plugin(crate::demo::DemoPlugin)?;
    }

    let engine = builder.build().context("build new engine")?;

    engine.run().context("run new engine")?;
    Ok(())
}
