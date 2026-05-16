//! The [`PlayerRegion`] is the region of the screen that the local players
//! are rendered to.

/// A player's rectangular slice of a window, expressed in normalised
/// `[0, 1] × [0, 1]` window coordinates.
///
/// * `offset` — top-left corner of the region in window-normalised space.
/// * `size`   — width and height of the region in window-normalised space.
///
/// Both axes run from `0.0` (left / top) to `1.0` (right / bottom).
///
/// # Invariants
///
/// The caller is responsible for ensuring that `offset + size` does not exceed
/// `(1.0, 1.0)`.  The struct itself does not enforce this.
///
/// # Examples
///
/// ```rust
/// # use dirk_player::region::PlayerRegion;
/// // Left half of the screen (Player 1 in a horizontal split-screen).
/// let p1 = PlayerRegion {
///     offset: glam::vec2(0.0, 0.0),
///     size:   glam::vec2(0.5, 1.0),
/// };
/// assert!(p1.contains(glam::vec2(0.25, 0.5)));
/// assert!(!p1.contains(glam::vec2(0.75, 0.5)));
/// ```
#[derive(Debug, Clone)]
pub struct PlayerRegion {
    /// Top-left corner in normalised window space.
    pub offset: glam::Vec2,
    /// Width and height in normalised window space.
    pub size: glam::Vec2,
}

impl Default for PlayerRegion {
    /// Returns the full-screen region: `offset = (0,0)`, `size = (1,1)`.
    fn default() -> Self {
        Self {
            offset: glam::Vec2::ZERO,
            size: glam::Vec2::ONE,
        }
    }
}

impl PlayerRegion {
    /// Returns `true` if `norm_pos` (in `[0,1]²` window space) lies **inside**
    /// this region.
    ///
    /// The check is half-open: the left and top edges are inclusive, the right
    /// and bottom edges are exclusive.  This ensures that a position sitting
    /// exactly on a shared boundary between two adjacent regions belongs to
    /// exactly one of them.
    ///
    /// # Arguments
    ///
    /// * `norm_pos` — a point in normalised window space `[0, 1]²`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use dirk_player::region::PlayerRegion;
    /// let region = PlayerRegion {
    ///     offset: glam::vec2(0.25, 0.25),
    ///     size:   glam::vec2(0.5,  0.5),
    /// };
    /// assert!(region.contains(glam::vec2(0.5, 0.5)));   // centre — inside
    /// assert!(region.contains(glam::vec2(0.25, 0.25))); // top-left corner — inclusive
    /// assert!(!region.contains(glam::vec2(0.75, 0.75))); // bottom-right corner — exclusive
    /// assert!(!region.contains(glam::vec2(0.1, 0.5)));  // left of region
    /// ```
    #[must_use]
    pub fn contains(&self, norm_pos: glam::Vec2) -> bool {
        let max = self.offset + self.size;
        norm_pos.cmpge(self.offset).all() && norm_pos.cmplt(max).all()
    }

    /// Maps a normalised **window** position to a normalised position *within*
    /// this region (`[0, 1]²` in region-local space).
    ///
    /// This is useful for passing pointer / cursor coordinates to per-player
    /// UI or camera logic without it needing to know about the global layout.
    ///
    /// Returns `Vec2::ZERO` if the region has zero area (i.e. either dimension
    /// of `size` is `0.0`) to avoid a division by zero.
    ///
    /// # Arguments
    ///
    /// * `norm_pos` — a point in normalised window space `[0, 1]²`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use dirk_player::region::PlayerRegion;
    /// // Right half of the screen.
    /// let region = PlayerRegion {
    ///     offset: glam::vec2(0.5, 0.0),
    ///     size:   glam::vec2(0.5, 1.0),
    /// };
    /// // The horizontal centre of the right half maps to 0.5 in local space.
    /// let local = region.to_local(glam::vec2(0.75, 0.5));
    /// assert!((local.x - 0.5).abs() < f32::EPSILON);
    /// assert!((local.y - 0.5).abs() < f32::EPSILON);
    /// ```
    #[must_use]
    pub fn to_local(&self, norm_pos: glam::Vec2) -> glam::Vec2 {
        if self.size.x == 0.0 || self.size.y == 0.0 {
            return glam::Vec2::ZERO;
        }
        (norm_pos - self.offset) / self.size
    }
}
