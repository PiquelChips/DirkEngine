/// Holds the visual description of an entity.
#[derive(Debug, Clone)]
// TODO: actually renderable
pub struct Renderable {
    mesh: String,
    texture: String,
}

/// A zero-size tag component — marks the player entity.
#[derive(Debug, Clone)]
pub struct IsPlayer;

/// A zero-size tag component — marks entities scheduled for removal.
#[derive(Debug, Clone)]
pub struct IsDead;
