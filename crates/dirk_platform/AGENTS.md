This is `DirkEngine's` platform crate.
It uses `winit` behind the scenes for the platform layer. However, I do not
want this dependency to lead out & end up anywhere else in the engine as I
am considering writing my own platform system.
You should thus either reexport winit types from `dirk_platform` or write
your own wrapper/shim types.
