# platform

This crate contains the platform layer. It should be the only place where
platform-dependent code or platform `#[cfg]` attributes are needed, which keeps
the rest of the engine behind a central platform API.

The `DirkEngine` platform API is built on `winit`.

Register `PlatformPlugin` with an `EngineBuilder` to install the platform
subsystem. The plugin creates the main window and publishes a `PlatformWindows`
resource for subsystems such as the renderer and demo setup.
