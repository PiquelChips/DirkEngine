//! This module handles everything to do with players.

use std::{collections::HashSet, f32::consts::PI};

use events::{Dispatcher, EventManager};
use platform::{KeyCode, PhysicalKey, WindowId};

use crate::{Entity, World, WorldId, components, events::PlayerEvent};

pub type PlayerId = u32;

/// Look sensitivity in radians per physical pixel.
const MOUSE_SENSITIVITY: f32 = 0.002;
/// Movement speed in world units per second.
const MOVE_SPEED: f32 = 500.0;
/// Pitch is clamped to ±(90° − ε) to prevent gimbal flip.
const PITCH_LIMIT: f32 = PI / 2.0 - 0.01;

/// A player's rectangular slice of a window, expressed in normalised
/// `[0, 1] × [0, 1]` window coordinates.
///
/// A single-player game uses the default full-screen region. For split-screen,
/// set up two regions whose offsets and sizes tile the window without overlap.
///
/// ```text
/// ┌─────────────────────┐
/// │  Player 1           │  offset=(0.0, 0.0), size=(0.5, 1.0)
/// │           │         │
/// │           │  P2     │  offset=(0.5, 0.0), size=(0.5, 1.0)
/// └─────────────────────┘
/// ```
pub struct PlayerRegion {
    /// Top-left corner in normalised window space.
    pub offset: glam::Vec2,
    /// Width and height in normalised window space.
    pub size: glam::Vec2,
}

impl Default for PlayerRegion {
    fn default() -> Self {
        Self {
            offset: glam::Vec2::ZERO,
            size: glam::Vec2::ONE,
        }
    }
}

impl PlayerRegion {
    /// Returns `true` if `norm_pos` (in `[0,1]²` window space) lies inside
    /// this region.
    pub fn contains(&self, norm_pos: glam::Vec2) -> bool {
        let max = self.offset + self.size;
        norm_pos.cmpge(self.offset).all() && norm_pos.cmplt(max).all()
    }

    /// Maps a normalised window position to a normalised position *within*
    /// this region (`[0, 1]²` in region space).
    ///
    /// Returns `Vec2::ZERO` if the region has zero area.
    pub fn to_local(&self, norm_pos: glam::Vec2) -> glam::Vec2 {
        if self.size.x == 0.0 || self.size.y == 0.0 {
            return glam::Vec2::ZERO;
        }
        (norm_pos - self.offset) / self.size
    }
}

/// Per-player input state that accumulates between ticks.
#[derive(Default)]
struct InputState {
    /// Physical keys currently held down.
    keys_held: HashSet<KeyCode>,
    /// Raw pointer position (physical pixels) the last time we saw a
    /// `PointerMoved` event. `None` until the first event or after the
    /// pointer leaves the window.
    last_pointer_px: Option<glam::DVec2>,
    /// Accumulated raw pointer movement (physical pixels) since the last
    /// call to [`Player::tick`]. Used to drive mouse look.
    look_delta_px: glam::Vec2,
}

pub struct Player {
    id: PlayerId,
    world: WorldId,
    entity: Entity,
    window: WindowId,
    region: PlayerRegion,
    input_state: InputState,
    dispatcher: Dispatcher<PlayerEvent>,
}

impl Player {
    /// Spawns a new player entity into `world`, attaches a default
    /// [`world::components::Transform`] and [`world::components::Camera`],
    /// and returns the [`Player`] handle.
    ///
    /// Fires [`PlayerEvent::Spawned`].
    pub fn spawn(
        id: PlayerId,
        world: &mut World,
        window: WindowId,
        event_manager: &EventManager,
    ) -> Self {
        use crate::components;
        let entity = world.spawn();
        world.insert(
            entity,
            components::Transform {
                location: glam::vec3(0.0, 1000.0, 1000.0),
                rotation: glam::vec3(-PI / 4.0, 0.0, 0.0),
                scale: glam::Vec3::ONE,
            },
        );
        world.insert(
            entity,
            components::Camera {
                fov: 45_f32.to_radians(),
                near_clip: 0.1,
                far_clip: 100_000.0,
                width: 100.0,
                height: 100.0,
            },
        );

        let dispatcher = event_manager.register();
        dispatcher.dispatch(PlayerEvent::Spawned(id));

        Self {
            id,
            world: world.id(),
            entity,
            window,
            region: PlayerRegion::default(),
            input_state: InputState::default(),
            dispatcher,
        }
    }

    /// Removes the player entity from the world and fires
    /// [`PlayerEvent::Despawned`].
    ///
    /// Consumes `self` so the handle cannot be used after despawning.
    pub fn despawn(self, world: &mut World) {
        world.despawn(self.entity);
        self.dispatcher.dispatch(PlayerEvent::Despawned(self.id));
    }

    pub fn id(&self) -> PlayerId {
        self.id
    }
    pub fn world(&self) -> WorldId {
        self.world
    }
    pub fn entity(&self) -> Entity {
        self.entity
    }
    pub fn window(&self) -> WindowId {
        self.window
    }
    pub fn region(&self) -> &PlayerRegion {
        &self.region
    }
    pub fn set_region(&mut self, region: PlayerRegion) {
        self.region = region;
    }

    /// Feed a raw platform event into this player's input accumulator.
    ///
    /// Called by [`crate::input::InputManager`] — do not call directly from
    /// outside the crate.
    pub fn handle_input_event(&mut self, event: platform::InputEvent) {
        use platform::InputEvent;

        match event {
            // Only register the initial press, not OS key-repeat events.
            InputEvent::KeyPressed {
                physical_key: PhysicalKey::Code(code),
                repeat: false,
                ..
            } => {
                self.input_state.keys_held.insert(code);
            }
            InputEvent::KeyReleased {
                physical_key: PhysicalKey::Code(code),
                ..
            } => {
                self.input_state.keys_held.remove(&code);
            }
            // Accumulate raw pixel deltas for mouse look. We intentionally
            // keep these in physical pixels so MOUSE_SENSITIVITY has a
            // natural unit (radians per pixel).
            InputEvent::PointerMoved { position, .. } => {
                if let Some(last) = self.input_state.last_pointer_px {
                    let d = position - last;
                    self.input_state.look_delta_px.x += d.x as f32;
                    self.input_state.look_delta_px.y += d.y as f32;
                }
                self.input_state.last_pointer_px = Some(position);
            }
            // Reset tracking so the next entry doesn't produce a spurious
            // large delta.
            InputEvent::PointerLeft { .. } | InputEvent::PointerEntered { .. } => {
                self.input_state.last_pointer_px = None;
            }
            _ => {}
        }
    }

    /// Applies accumulated input to the player's [`world::components::Transform`].
    ///
    /// Call this **once per frame**, after [`crate::input::InputManager::tick`]
    /// has routed all pending events.
    ///
    /// Fires [`PlayerEvent::Updated`] if the transform actually changed this
    /// frame (either look rotation or movement). Idle ticks produce no event.
    pub fn tick(&mut self, delta_time: f32, world: &mut World) {
        let Some(transform) = world.get_mut::<components::Transform>(self.entity) else {
            return;
        };

        if self.input_state.look_delta_px != glam::Vec2::ZERO {
            // Yaw (left/right): rotate around world Y.
            transform.rotation.y -= self.input_state.look_delta_px.x * MOUSE_SENSITIVITY;
            // Pitch (up/down): rotate around local X, clamped.
            transform.rotation.x = (transform.rotation.x
                - self.input_state.look_delta_px.y * MOUSE_SENSITIVITY)
                .clamp(-PITCH_LIMIT, PITCH_LIMIT);

            self.input_state.look_delta_px = glam::Vec2::ZERO;
        }

        // Movement is intentionally flat (independent of pitch) so that
        // WASD always moves the player horizontally. Up/down are separate
        // bindings. We project `forward` onto the XZ-plane for this.
        let forward = transform.forward();
        let horizontal_forward = glam::vec3(forward.x, 0.0, forward.z).normalize_or_zero();
        // In a right-handed system with Y-up: forward × Y = right.
        let right = horizontal_forward.cross(glam::Vec3::Y).normalize_or_zero();

        let mut move_dir = glam::Vec3::ZERO;
        let keys = &self.input_state.keys_held;

        if keys.contains(&KeyCode::KeyZ) || keys.contains(&KeyCode::ArrowUp) {
            move_dir += horizontal_forward;
        }
        if keys.contains(&KeyCode::KeyS) || keys.contains(&KeyCode::ArrowDown) {
            move_dir -= horizontal_forward;
        }
        if keys.contains(&KeyCode::KeyD) || keys.contains(&KeyCode::ArrowRight) {
            move_dir += right;
        }
        if keys.contains(&KeyCode::KeyA) || keys.contains(&KeyCode::ArrowLeft) {
            move_dir -= right;
        }

        if keys.contains(&KeyCode::Space) || keys.contains(&KeyCode::KeyE) {
            move_dir += glam::Vec3::Y;
        }
        if keys.contains(&KeyCode::ShiftLeft) || keys.contains(&KeyCode::KeyQ) {
            move_dir -= glam::Vec3::Y;
        }

        if move_dir.length_squared() > 0.0 {
            transform.location += move_dir.normalize() * MOVE_SPEED * delta_time;
        }
    }
}
