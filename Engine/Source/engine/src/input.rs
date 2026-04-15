use std::collections::HashMap;

use platform::WindowId;
use player::PlayerId;
use tracing::trace;

use crate::Engine;

/// Tracks the most recently reported pixel dimensions for each window.
///
/// Used to convert raw physical-pixel pointer coordinates to normalised
/// `[0, 1] × [0, 1]` window space before region tests.
#[derive(Default)]
pub struct WindowSizes(HashMap<WindowId, glam::UVec2>);

impl WindowSizes {
    fn update(&mut self, id: WindowId, width: u32, height: u32) {
        self.0.insert(id, glam::uvec2(width, height));
    }

    /// Returns `None` if the window size is unknown (no resize event received
    /// yet) or if either dimension is zero.
    fn normalize(&self, id: &WindowId, px: glam::DVec2) -> Option<glam::Vec2> {
        let s = self.0.get(id)?;
        if s.x == 0 || s.y == 0 {
            return None;
        }
        Some(glam::vec2(
            (px.x / s.x as f64) as f32,
            (px.y / s.y as f64) as f32,
        ))
    }
}

impl Engine {
    /// Process all pending input events and forward each one to the appropriate
    /// player.
    ///
    /// Window resize events are absorbed first so that pointer-position
    /// normalisation always uses up-to-date dimensions.
    pub fn process_input_events(&mut self) {
        for event in self.window_consumer.consume_all() {
            if let platform::WindowEvent::Resized { id, width, height } = event {
                trace!("Window {id:?} resized to {width}×{height}");
                self.window_sizes.update(id, width, height);
            }
        }

        // Collect upfront so the borrow on `self.input_consumer` ends before
        // we need to borrow `self.window_sizes` inside `resolve_event`.
        let events: Vec<platform::InputEvent> = self.input_consumer.consume_all().collect();

        for event in events {
            // `resolve_event` takes an immutable borrow of `players` (reads
            // regions only) and returns the target index + adjusted event.
            // We then use the index for the one mutable borrow below,
            // satisfying the borrow checker.
            let Some((player, adjusted)) = self.resolve_event(&event) else {
                continue;
            };
            let Some(player) = self.players.get_mut(&player) else {
                continue;
            };
            player.handle_input_event(adjusted);
        }
    }

    /// Determine which player should receive `event` and, for pointer events
    /// that carry a position, remap that position to region-local `[0, 1]²`
    /// coordinates.
    ///
    /// Returns `None` if no player owns this event (e.g. the pointer is over
    /// a region not assigned to any player, or no window size is known yet).
    ///
    /// ## Position policy
    /// - **`PointerMoved`** — position is passed through unchanged in raw
    ///   physical pixels. [`Player::handle_input_event`] uses consecutive
    ///   pixel positions to compute a look delta, so raw pixels are the right
    ///   unit here (sensitivity is in rad/px).
    /// - **`MouseButtonPressed` / `MouseButtonReleased`** — position is
    ///   remapped to region-local `[0, 1]²` so that game logic can work with
    ///   a stable coordinate system regardless of which region the player
    ///   occupies.
    /// - **All other events** — routed to the first player bound to that
    ///   window (keyboard, scroll, etc. are not position-sensitive).
    fn resolve_event(
        &self,
        event: &platform::InputEvent,
    ) -> Option<(PlayerId, platform::InputEvent)> {
        use platform::InputEvent as IE;

        match event {
            IE::PointerMoved { id, position } => {
                let norm = self.window_sizes.normalize(id, *position)?;
                let player = self.find_by_region(*id, norm)?;
                Some((player, event.clone()))
            }
            IE::MouseButtonPressed {
                id,
                button,
                position,
            } => {
                let norm = self.window_sizes.normalize(id, *position)?;
                let player = self.find_by_region(*id, norm)?;
                let local = self.players[&player].region().to_local(norm);
                Some((
                    player,
                    IE::MouseButtonPressed {
                        id: *id,
                        button: button.clone(),
                        position: glam::dvec2(local.x as f64, local.y as f64),
                    },
                ))
            }
            IE::MouseButtonReleased {
                id,
                button,
                position,
            } => {
                let norm = self.window_sizes.normalize(id, *position)?;
                let player = self.find_by_region(*id, norm)?;
                let local = self.players[&player].region().to_local(norm);
                Some((
                    player,
                    IE::MouseButtonReleased {
                        id: *id,
                        button: button.clone(),
                        position: glam::dvec2(local.x as f64, local.y as f64),
                    },
                ))
            }
            other => self
                .players
                .iter()
                .find_map(|(id, p)| {
                    if p.window() == *other.id() {
                        None
                    } else {
                        Some(id.to_owned())
                    }
                })
                .map(|player| (player, other.clone())),
        }
    }

    /// Find the index of the player whose window matches `window_id` and whose
    /// region contains `norm_pos` (normalised `[0, 1]²` window space).
    fn find_by_region(&self, window_id: WindowId, norm_pos: glam::Vec2) -> Option<PlayerId> {
        self.players.iter().find_map(|(id, p)| {
            if p.window() == window_id && p.region().contains(norm_pos) {
                None
            } else {
                Some(id.to_owned())
            }
        })
    }
}
