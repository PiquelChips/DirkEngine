macro_rules! define_flags {
    (
        $(#[$meta:meta])*
        $visibility:vis struct $name:ident($bits:ty) {
            $($(#[$constant_meta:meta])* const $constant:ident = $value:expr;)+
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        $visibility struct $name($bits);

        impl $name {
            $($(#[$constant_meta])*
            $visibility const $constant: Self = Self($value);)+

            /// Returns a value with no flags set.
            #[must_use]
            $visibility const fn empty() -> Self {
                Self(0)
            }

            /// Returns the raw bits represented by this value.
            #[must_use]
            $visibility const fn bits(self) -> $bits {
                self.0
            }

            /// Returns whether no flags are set.
            #[must_use]
            $visibility const fn is_empty(self) -> bool {
                self.0 == 0
            }

            /// Returns whether every flag in `other` is present.
            #[must_use]
            $visibility const fn contains(self, other: Self) -> bool {
                self.0 & other.0 == other.0
            }

            /// Returns whether any flag in `other` is present.
            #[must_use]
            $visibility const fn intersects(self, other: Self) -> bool {
                self.0 & other.0 != 0
            }
        }

        impl std::ops::BitOr for $name {
            type Output = Self;

            fn bitor(self, rhs: Self) -> Self::Output {
                Self(self.0 | rhs.0)
            }
        }

        impl std::ops::BitOrAssign for $name {
            fn bitor_assign(&mut self, rhs: Self) {
                self.0 |= rhs.0;
            }
        }

        impl std::ops::BitAnd for $name {
            type Output = Self;

            fn bitand(self, rhs: Self) -> Self::Output {
                Self(self.0 & rhs.0)
            }
        }
    };
}

pub(crate) use define_flags;
