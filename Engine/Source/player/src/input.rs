use crate::Player;

/// This struct handles receiving window input events and dispatching them
/// to each player.
pub struct InputManager {
    consumer: events::Consumer<platform::InputEvent>,
}

impl InputManager {
    pub fn init(event_manager: &mut events::EventManager) -> Self {
        Self {
            consumer: event_manager.subscribe(),
        }
    }
    pub fn tick(&self, delta_time: f32, players: &[Player]) {}
}
