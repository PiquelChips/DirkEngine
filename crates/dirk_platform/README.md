# platform

This crate has all the platform level functionnality. It should be the
only place where any kind of platform dependent code or `#[cfg()]` attributes
should be used. This allows us to create a central platform API for eaiser
development.

The `DirkEngine`'s platform API is build on the winit crate.
