# renderer

The renderer owns Vulkan rendering for the engine. GPU operations are handled
through `ash`.

On macOS, the renderer discovers the `AppKit` surface extensions and enables
Vulkan portability enumeration for `MoltenVK`. It loads the Vulkan loader
provided by the user's Vulkan SDK; no Vulkan SDK or `MoltenVK` library is
linked, installed, or bundled by this crate.

Debug builds enable `VK_LAYER_KHRONOS_validation`, so its manifest must be
discoverable through `VK_ADD_LAYER_PATH` or a standard Vulkan layer path.

Register `RendererPlugin` with an `EngineBuilder` to install the renderer
subsystem and its ECS integration systems. The plugin depends on
`PlatformPlugin` and `AssetsPlugin`, reads engine metadata for Vulkan
application info, and renders once per engine tick.
