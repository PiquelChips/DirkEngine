# renderer

The renderer owns Vulkan rendering for the engine. GPU operations are handled
through `ash`.

Register `RendererPlugin` with an `EngineBuilder` to install the renderer
subsystem and its ECS integration systems. The plugin depends on
`PlatformPlugin` and `AssetsPlugin`, reads engine metadata for Vulkan
application info, and renders once per engine tick.
