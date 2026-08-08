macro_rules! define_flags {
    (
        $(#[$meta:meta])*
        pub struct $name:ident($storage:ty) {
            $($(#[$flag_meta:meta])* const $flag:ident = $value:expr;)+
        }
    ) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
        pub struct $name($storage);

        impl $name {
            /// No flags.
            pub const NONE: Self = Self(0);
            $($(#[$flag_meta])* pub const $flag: Self = Self($value);)+

            /// Returns true when all flags in `other` are present.
            #[must_use]
            pub const fn contains(self, other: Self) -> bool {
                self.0 & other.0 == other.0
            }

            /// Returns the underlying bit representation.
            #[must_use]
            pub const fn bits(self) -> $storage {
                self.0
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
