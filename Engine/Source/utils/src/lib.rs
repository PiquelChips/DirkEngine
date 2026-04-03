use std::{fmt, str::FromStr};

use thiserror::Error;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Version(u32);

impl Version {
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

}
