//! This module has all the proxy types. These are the render representations
//! of the components needed for rendering

use ash::vk;
use platform::WindowId;
use universe::{Entity, WorldId};
use world::player::PlayerId;

use crate::{MAX_FRAMES_IN_FLIGHT, resources::buffer::UniformBuffer};
