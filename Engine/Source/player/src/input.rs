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
    pub fn tick(&self, _delta_time: f32, players: &[Player]) {
        for event in self.consumer.consume_all() {
            let Some(player) = Self::get_player_for_event(players, &event) else {
                continue;
            };

            // TODO: adjust the position in certain of the events to
            // match the actual position within the player's region
            // (in normalised coords)
            player.handle_input_event(event);
        }
    }

    fn get_player_for_event<'a>(
        players: &'a [Player],
        event: &platform::InputEvent,
    ) -> Option<&'a Player> {
        // TODO: calculate the position based on the regions
        for player in players {
            if player.window() == event.id() {
                return Some(player);
            }
        }
        None
    }
}
