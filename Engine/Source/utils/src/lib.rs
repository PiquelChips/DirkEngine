use std::{fmt, str::FromStr};

use thiserror::Error;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Version(u32);

impl Default for Version {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Version {
    pub const ZERO: Self = Self(0);
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        assert!(
            major < (1 << 10),
            "major version must fit in 10 bits (0–1023)"
        );
        assert!(
            minor < (1 << 10),
            "minor version must fit in 10 bits (0–1023)"
        );
        assert!(
            patch < (1 << 12),
            "patch version must fit in 12 bits (0–4095)"
        );
        Self(((major) << 22) | ((minor) << 12) | (patch))
    }
    pub fn major(&self) -> u32 {
        self.0 >> 22
    }
    pub fn minor(&self) -> u32 {
        (self.0 >> 12) & 0x3ff
    }
    pub fn patch(&self) -> u32 {
        self.0 & 0xfff
    }
    /// Increments major, resets minor and patch to 0.
    pub fn bump_major(self) -> Self {
        Self::new(self.major() + 1, 0, 0)
    }
    /// Increments minor, resets patch to 0.
    pub fn bump_minor(self) -> Self {
        Self::new(self.major(), self.minor() + 1, 0)
    }
    /// Increments patch.
    pub fn bump_patch(self) -> Self {
        Self::new(self.major(), self.minor(), self.patch() + 1)
    }
    /// Returns true if `self` is semver-compatible with `required`
    /// (same major, self.minor >= required.minor).
    pub fn is_compatible_with(self, required: Self) -> bool {
        self.major() == required.major() && self >= required
    }
    /// If the major is 0, then this is a prerelease version.
    pub fn is_prerelease(self) -> bool {
        self.major() == 0
    }
}

impl From<u32> for Version {
    fn from(v: u32) -> Self {
        Self(v)
    }
}

impl From<Version> for u32 {
    fn from(v: Version) -> Self {
        v.0
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Comparing the raw packed `u32` is equivalent to comparing
/// (major, minor, patch) lexicographically because the fields are
/// stored in significance order.
impl Ord for Version {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major(), self.minor(), self.patch())
    }
}

impl fmt::Debug for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Version({self})")
    }
}

#[derive(Debug, PartialEq, Error)]
pub struct ParseVersionError(String);

impl fmt::Display for ParseVersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid version string: '{}'", self.0)
    }
}

impl FromStr for Version {
    type Err = ParseVersionError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ParseVersionError(s.to_string());
        let mut parts = s.trim_start_matches('v').splitn(3, '.');
        let major = parts
            .next()
            .ok_or_else(err)?
            .parse::<u32>()
            .map_err(|_| err())?;
        let minor = parts
            .next()
            .ok_or_else(err)?
            .parse::<u32>()
            .map_err(|_| err())?;
        let patch = parts
            .next()
            .ok_or_else(err)?
            .parse::<u32>()
            .map_err(|_| err())?;
        Ok(Self::new(major, minor, patch))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Construction & accessors --

    #[test]
    fn test_fields_roundtrip() {
        let v = Version::new(1, 2, 3);
        assert_eq!(v.major(), 1);
        assert_eq!(v.minor(), 2);
        assert_eq!(v.patch(), 3);
    }

    #[test]
    fn test_zero_constant() {
        assert_eq!(Version::ZERO.major(), 0);
        assert_eq!(Version::ZERO.minor(), 0);
        assert_eq!(Version::ZERO.patch(), 0);
    }

    /// major/minor: 10 bits → max 1023 ; patch: 12 bits → max 4095
    #[test]
    fn test_max_field_values() {
        let v = Version::new(1023, 1023, 4095);
        assert_eq!(v.major(), 1023);
        assert_eq!(v.minor(), 1023);
        assert_eq!(v.patch(), 4095);
    }

    #[test]
    #[should_panic]
    fn test_major_overflow_panics() {
        Version::new(1024, 0, 0);
    }

    #[test]
    #[should_panic]
    fn test_patch_overflow_panics() {
        Version::new(0, 0, 4096);
    }

    // -- Bumping --

    #[test]
    fn test_bump_major_resets_minor_and_patch() {
        let v = Version::new(1, 5, 3).bump_major();
        assert_eq!((v.major(), v.minor(), v.patch()), (2, 0, 0));
    }

    #[test]
    fn test_bump_minor_resets_patch() {
        let v = Version::new(1, 5, 3).bump_minor();
        assert_eq!((v.major(), v.minor(), v.patch()), (1, 6, 0));
    }

    #[test]
    fn test_bump_patch() {
        let v = Version::new(1, 5, 3).bump_patch();
        assert_eq!((v.major(), v.minor(), v.patch()), (1, 5, 4));
    }

    // -- Ordering --

    #[test]
    fn test_ordering() {
        assert!(Version::new(2, 0, 0) > Version::new(1, 9, 9));
        assert!(Version::new(1, 1, 0) > Version::new(1, 0, 99));
        assert!(Version::new(1, 0, 1) > Version::new(1, 0, 0));
        assert_eq!(Version::new(1, 2, 3), Version::new(1, 2, 3));
    }

    // -- Compatibility --

    #[test]
    fn test_compatible_same_major_higher_minor() {
        assert!(Version::new(1, 3, 0).is_compatible_with(Version::new(1, 2, 0)));
    }

    #[test]
    fn test_incompatible_different_major() {
        assert!(!Version::new(2, 0, 0).is_compatible_with(Version::new(1, 0, 0)));
    }

    #[test]
    fn test_incompatible_older_version() {
        assert!(!Version::new(1, 1, 0).is_compatible_with(Version::new(1, 2, 0)));
    }

    #[test]
    fn test_prerelease() {
        assert!(Version::new(0, 9, 0).is_prerelease());
        assert!(!Version::new(1, 0, 0).is_prerelease());
    }

    // -- Formatting & parsing --

    #[test]
    fn test_display() {
        assert_eq!(Version::new(1, 2, 3).to_string(), "1.2.3");
    }

    #[test]
    fn test_parse_plain() {
        let v: Version = "1.2.3".parse().unwrap();
        assert_eq!((v.major(), v.minor(), v.patch()), (1, 2, 3));
    }

    #[test]
    fn test_parse_with_v_prefix() {
        let v: Version = "v2.0.0".parse().unwrap();
        assert_eq!(v.major(), 2);
    }

    #[test]
    fn test_parse_invalid() {
        assert!("1.2".parse::<Version>().is_err());
        assert!("abc".parse::<Version>().is_err());
        assert!("1.x.3".parse::<Version>().is_err());
    }

    #[test]
    fn test_parse_display_roundtrip() {
        let original = Version::new(3, 14, 42);
        let roundtripped: Version = original.to_string().parse().unwrap();
        assert_eq!(original, roundtripped);
    }

    // -- Raw conversion --

    #[test]
    fn test_from_into_u32() {
        let v = Version::new(1, 2, 3);
        let raw: u32 = v.into();
        let back = Version::from(raw);
        assert_eq!(v, back);
    }
}
