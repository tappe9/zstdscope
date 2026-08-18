/// Resource limits applied by [`crate::inspect_with_limits`].
///
/// Limits are checked before parsing the next affected structure. Reaching a
/// limit is allowed; attempting to parse one more frame or block returns
/// [`crate::ZstdError::ResourceLimitExceeded`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InspectionLimits {
    /// Maximum number of top-level Standard or Skippable Frames.
    pub max_frames: usize,
    /// Maximum number of blocks within any one Standard Frame.
    pub max_blocks_per_frame: usize,
    /// Maximum total number of blocks across all Standard Frames.
    pub max_total_blocks: usize,
}

impl InspectionLimits {
    pub(crate) const UNLIMITED: Self = Self {
        max_frames: usize::MAX,
        max_blocks_per_frame: usize::MAX,
        max_total_blocks: usize::MAX,
    };
}
