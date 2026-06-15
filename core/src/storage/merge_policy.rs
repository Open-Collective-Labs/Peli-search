use crate::storage::segment_metadata::SegmentMetadata;

/// Decides when and which segments to merge.
///
/// The merge policy prevents segment explosion by triggering compaction
/// when the number of segments or their total size exceeds configured
/// thresholds. It also selects which segments to merge together.
///
/// # Example
///
/// ```
/// use pelisearch_core::storage::merge_policy::{MergePolicy, SegmentToMerge};
///
/// let policy = MergePolicy::default();
/// let segments = vec![];
/// let to_merge = policy.select_segments(&segments);
/// assert!(matches!(to_merge, SegmentToMerge::None), "no segments → no merge");
/// ```
#[derive(Debug, Clone)]
pub struct MergePolicy {
    /// Maximum number of segments allowed before triggering a merge.
    /// Default: 10
    pub max_segment_count: usize,

    /// Maximum total size (bytes) of all segments before triggering a merge.
    /// Default: 100 MB (100 * 1024 * 1024)
    pub max_total_size_bytes: u64,

    /// Minimum number of segments to merge in one operation.
    /// Merging too few is wasteful. Default: 3
    pub min_merge_count: usize,

    /// Maximum number of segments to merge in one operation.
    /// Merging too many at once is expensive. Default: 20
    pub max_merge_count: usize,
}

/// The result of selecting segments to merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentToMerge {
    /// No merging is needed at this time.
    None,

    /// Merge these segment IDs together into one.
    Merge(Vec<u64>),
}

impl Default for MergePolicy {
    fn default() -> Self {
        Self {
            max_segment_count: 10,
            max_total_size_bytes: 100 * 1024 * 1024, // 100 MB
            min_merge_count: 3,
            max_merge_count: 20,
        }
    }
}

impl MergePolicy {
    /// Create a new policy with custom thresholds.
    ///
    /// # Panics
    ///
    /// Panics if `max_segment_count < 2`, `min_merge_count < 2`,
    /// or `min_merge_count > max_merge_count`.
    pub fn new(
        max_segment_count: usize,
        max_total_size_bytes: u64,
        min_merge_count: usize,
        max_merge_count: usize,
    ) -> Self {
        assert!(
            max_segment_count >= 2,
            "max_segment_count must be >= 2"
        );
        assert!(
            min_merge_count >= 2,
            "min_merge_count must be >= 2"
        );
        assert!(
            min_merge_count <= max_merge_count,
            "min_merge_count must be <= max_merge_count"
        );
        Self {
            max_segment_count,
            max_total_size_bytes,
            min_merge_count,
            max_merge_count,
        }
    }

    /// Select which segments to merge, given the current active segments.
    ///
    /// Returns:
    /// - `SegmentToMerge::None` if no merge is needed
    /// - `SegmentToMerge::Merge(ids)` with the segment IDs to merge
    ///
    /// The policy prefers merging the *oldest* segments first when a merge
    /// is triggered, because older segments are more likely to contain
    /// cold data and merging them frees up resources.
    pub fn select_segments(&self, segments: &[SegmentMetadata]) -> SegmentToMerge {
        let active: Vec<&SegmentMetadata> = segments.iter().filter(|s| s.is_active()).collect();

        // No merge needed if we're under the count AND size thresholds
        if active.len() < self.max_segment_count && self.total_size(&active) < self.max_total_size_bytes {
            return SegmentToMerge::None;
        }

        // Not enough segments to merge
        if active.len() < self.min_merge_count {
            return SegmentToMerge::None;
        }

        // Select segments to merge — oldest first (lower ID = older)
        let mut sorted: Vec<&SegmentMetadata> = active.into_iter().collect();
        sorted.sort_by_key(|s| s.id);

        let merge_count = std::cmp::min(sorted.len(), self.max_merge_count);

        // If count is above max_segment_count, merge just enough to get
        // under the threshold. Otherwise merge the oldest min_merge_count.
        let count = if sorted.len() > self.max_segment_count {
            // Merge everything beyond max_segment_count + 1, or at least min_merge_count
            std::cmp::max(
                sorted.len() - self.max_segment_count + 1,
                self.min_merge_count,
            )
        } else {
            // Over size threshold — merge the oldest segments
            self.min_merge_count
        };

        let count = std::cmp::min(count, merge_count);
        let ids: Vec<u64> = sorted.iter().take(count).map(|s| s.id).collect();
        SegmentToMerge::Merge(ids)
    }

    /// Whether compaction should be triggered at all.
    pub fn should_compact(&self, segments: &[SegmentMetadata]) -> bool {
        !matches!(self.select_segments(segments), SegmentToMerge::None)
    }

    /// Compute the target size (in bytes) for a merged segment.
    ///
    /// This is the sum of the sizes of the segments that would be merged.
    /// Useful for estimating whether a merge will produce a segment that
    /// exceeds the size threshold.
    pub fn merge_target_size(&self, segments: &[SegmentMetadata], ids: &[u64]) -> u64 {
        segments
            .iter()
            .filter(|s| ids.contains(&s.id))
            .map(|s| s.size_bytes)
            .sum()
    }

    /// Total on-disk size of the given segments.
    fn total_size(&self, segments: &[&SegmentMetadata]) -> u64 {
        segments.iter().map(|s| s.size_bytes).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::segment_metadata::SegmentMetadata;

    fn make_segment(id: u64, size_bytes: u64) -> SegmentMetadata {
        SegmentMetadata::new(id, 100, size_bytes)
    }

    #[test]
    fn default_policy_does_not_merge_empty() {
        let policy = MergePolicy::default();
        assert_eq!(policy.select_segments(&[]), SegmentToMerge::None);
        assert!(!policy.should_compact(&[]));
    }

    #[test]
    fn default_policy_does_not_merge_few_segments() {
        let policy = MergePolicy::default();
        let segments = vec![
            make_segment(1, 1024),
            make_segment(2, 1024),
            make_segment(3, 1024),
        ];
        // 3 segments is under max_segment_count (10) and size is small
        assert_eq!(policy.select_segments(&segments), SegmentToMerge::None);
    }

    #[test]
    fn policy_triggers_on_count_threshold() {
        let policy = MergePolicy::new(3, 1_000_000, 2, 5);
        // 4 segments = above max_segment_count (3)
        let segments = vec![
            make_segment(1, 1024),
            make_segment(2, 1024),
            make_segment(3, 1024),
            make_segment(4, 1024),
        ];
        let decision = policy.select_segments(&segments);
        assert!(
            matches!(&decision, SegmentToMerge::Merge(ids) if ids.len() >= 2),
            "should merge segments when count exceeds threshold"
        );
    }

    #[test]
    fn policy_triggers_on_size_threshold() {
        let policy = MergePolicy::new(10, 5000, 2, 5);
        // Each segment is 2000 bytes — 3 segments = 6000 > 5000
        let segments = vec![
            make_segment(1, 2000),
            make_segment(2, 2000),
            make_segment(3, 2000),
        ];
        let decision = policy.select_segments(&segments);
        assert!(
            matches!(&decision, SegmentToMerge::Merge(ids) if !ids.is_empty()),
            "should merge when total size exceeds threshold"
        );
    }

    #[test]
    fn policy_merges_oldest_first() {
        let policy = MergePolicy::new(3, 1_000_000, 2, 5);
        let segments = vec![
            make_segment(10, 1024),
            make_segment(5, 1024),
            make_segment(1, 1024),
            make_segment(20, 1024),
        ];
        let decision = policy.select_segments(&segments);
        match decision {
            SegmentToMerge::Merge(ids) => {
                // Should merge oldest (smallest ID) first
                assert!(ids.contains(&1), "oldest segment (1) should be merged");
                assert!(ids.contains(&5), "old segment (5) should be merged");
            }
            _ => panic!("expected merge decision"),
        }
    }

    #[test]
    fn policy_respects_max_merge_count() {
        let policy = MergePolicy::new(2, 1_000_000, 2, 3);
        // 10 segments — should merge at most 3 (max_merge_count)
        let segments: Vec<_> = (1..=10)
            .map(|i| make_segment(i, 1024))
            .collect();
        let decision = policy.select_segments(&segments);
        match decision {
            SegmentToMerge::Merge(ids) => {
                assert!(ids.len() <= 3, "should not exceed max_merge_count");
            }
            _ => panic!("expected merge decision"),
        }
    }

    #[test]
    fn policy_respects_min_merge_count() {
        let policy = MergePolicy::new(3, 1_000_000, 4, 5);
        let segments = vec![
            make_segment(1, 1024),
            make_segment(2, 1024),
            // Only 2 segments — below min_merge_count (4)
        ];
        assert_eq!(policy.select_segments(&segments), SegmentToMerge::None);
    }

    #[test]
    fn should_compact_returns_true_when_needed() {
        let policy = MergePolicy::new(2, 1_000_000, 2, 5);
        let segments = vec![
            make_segment(1, 1024),
            make_segment(2, 1024),
            make_segment(3, 1024),
        ];
        assert!(policy.should_compact(&segments));
    }

    #[test]
    fn should_compact_returns_false_when_not_needed() {
        let policy = MergePolicy::new(5, 1_000_000, 3, 5);
        let segments = vec![
            make_segment(1, 1024),
            make_segment(2, 1024),
            make_segment(3, 1024),
        ];
        // 3 segments, under max_segment_count (5), under size, and >= min_merge_count
        // but under max_segment_count triggers differently:
        assert!(!policy.should_compact(&segments));
    }

    #[test]
    fn merge_policy_with_non_default_config() {
        let policy = MergePolicy::new(5, 10_000_000, 3, 10);
        let segments = vec![
            make_segment(1, 1000),
            make_segment(2, 1000),
            make_segment(3, 1000),
        ];
        assert!(!policy.should_compact(&segments));

        let segments = vec![
            make_segment(1, 1000),
            make_segment(2, 1000),
            make_segment(3, 1000),
            make_segment(4, 1000),
            make_segment(5, 1000),
            make_segment(6, 1000),
        ];
        assert!(policy.should_compact(&segments));
    }

    #[test]
    fn inactive_segments_excluded() {
        let policy = MergePolicy::new(3, 1_000_000, 2, 5);
        let mut merging = make_segment(1, 1024);
        merging.state = crate::storage::SegmentState::Merging;
        let mut deleted = make_segment(2, 1024);
        deleted.state = crate::storage::SegmentState::Deleted;

        // Only 1 active segment (id=3), 2 inactive — should not merge
        let segments = vec![merging, deleted, make_segment(3, 1024)];
        assert_eq!(policy.select_segments(&segments), SegmentToMerge::None);
    }

    #[test]
    #[should_panic(expected = "max_segment_count must be >= 2")]
    fn invalid_max_count_panics() {
        MergePolicy::new(1, 1000, 2, 5);
    }

    #[test]
    #[should_panic(expected = "min_merge_count must be >= 2")]
    fn invalid_min_merge_panics() {
        MergePolicy::new(5, 1000, 1, 5);
    }

    #[test]
    #[should_panic(expected = "min_merge_count must be <= max_merge_count")]
    fn invalid_range_panics() {
        MergePolicy::new(5, 1000, 5, 3);
    }

    #[test]
    fn merge_target_size_sums_correctly() {
        let policy = MergePolicy::default();
        let segments = vec![
            make_segment(1, 1000),
            make_segment(2, 2000),
            make_segment(3, 3000),
            make_segment(4, 4000),
        ];
        let size = policy.merge_target_size(&segments, &[1, 3]);
        assert_eq!(size, 4000, "should sum sizes of segments 1 and 3");
    }

    #[test]
    fn merge_target_size_empty_ids() {
        let policy = MergePolicy::default();
        let segments = vec![make_segment(1, 1000)];
        let size = policy.merge_target_size(&segments, &[]);
        assert_eq!(size, 0);
    }

    #[test]
    fn merge_target_size_all_segments() {
        let policy = MergePolicy::default();
        let segments = vec![
            make_segment(1, 500),
            make_segment(2, 500),
            make_segment(3, 500),
        ];
        let size = policy.merge_target_size(&segments, &[1, 2, 3]);
        assert_eq!(size, 1500);
    }
}