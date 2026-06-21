//! This crate contains the CLI entrypoint & management for the enhanced clap
//! engine CLI.

use anyhow::Context;

use crate::DefaultPlugins;

/// Runs the enhanced [`clap`] CLI.
///
/// # Errors
///
/// Returns any error that occurs while running the engine.
pub fn run() -> anyhow::Result<()> {
    let mut builder = dirk_engine::Engine::builder();
    builder.with_plugin(DefaultPlugins)?;
    let engine = builder.build().context("build new engine")?;

    engine.run().context("run new engine")?;
    Ok(())
}
