# renderer

The renderer owns Vulkan rendering for the engine. GPU operations are handled
through `ash`.

On macOS, the renderer discovers the `AppKit` surface extensions and enables
Vulkan portability enumeration for `MoltenVK`. It first loads the normal Vulkan
loader and then falls back to a directly available `libMoltenVK.dylib`; no
`MoltenVK` library is linked or installed by this crate.

Debug builds enable `VK_LAYER_KHRONOS_validation`, so its manifest must be
discoverable through `VK_ADD_LAYER_PATH` or a standard Vulkan layer path.

Register `RendererPlugin` with an `EngineBuilder` to install the renderer
subsystem and its ECS integration systems. The plugin depends on
`PlatformPlugin` and `AssetsPlugin`, reads engine metadata for Vulkan
application info, and renders once per engine tick.
