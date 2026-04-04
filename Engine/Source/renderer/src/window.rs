/// Renderer internal window type. This object stores
/// the [utils::Window] implementation provided on
/// renderer init & window creation.
pub struct Window {
    window: Box<dyn utils::Window>,
}

impl Window {
    pub fn new(window: Box<dyn utils::Window>) -> Self {
        Self { window }
    }
}
