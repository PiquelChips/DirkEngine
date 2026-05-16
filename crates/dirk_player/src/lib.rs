#![doc = include_str!("../README.md")]

use std::{
    fmt::Display,
    ops::{Add, AddAssign},
};

use dirk_universe::components::Component;

/// A light identifier for [`DirkPlayer`]s.
///
/// [`PlayerId`] implements [`trait@Component`]. It is also used as the
/// [`Universe`] representation of the [`DirkPlayer`].
///
/// [`Universe`]: dirk_universe::Universe
#[derive(Component, Clone, Copy, Debug, Default, Hash, Eq, PartialEq)]
pub struct PlayerId(u32);

impl Display for PlayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Add<u32> for PlayerId {
    type Output = Self;
    fn add(self, rhs: u32) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl AddAssign<u32> for PlayerId {
    fn add_assign(&mut self, rhs: u32) {
        self.0 += rhs;
    }
}
