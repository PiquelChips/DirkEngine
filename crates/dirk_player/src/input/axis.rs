//! Analog input axes.

use super::{InputAction, InputSnapshot};

/// A named scalar axis composed from two digital actions.
#[derive(Debug, Clone)]
pub struct InputAxis {
    name: &'static str,
    negative: InputAction,
    positive: InputAction,
}

impl InputAxis {
    /// Creates a digital axis.
    #[must_use]
    pub fn digital(name: &'static str, negative: InputAction, positive: InputAction) -> Self {
        Self {
            name,
            negative,
            positive,
        }
    }

    /// Returns this axis' stable name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        self.name
    }

    pub(crate) fn value(&self, input: &InputSnapshot) -> f32 {
        let positive = f32::from(u8::from(self.positive.is_active(input)));
        let negative = f32::from(u8::from(self.negative.is_active(input)));
        positive - negative
    }
}
