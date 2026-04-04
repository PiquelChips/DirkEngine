/// Renderer internal window type. This object stores
/// the [utils::Window] implementation provided on
/// renderer init & window creation.
pub struct Window<PlatWindow: utils::Window> {
    window: PlatWindow,
}
