use serde::{Deserialize, Serialize};

use super::delta;
use super::varint;

/// A posting list backed by delta+varint compressed storage.
///
/// Posting lists are sorted sequences of internal document IDs (u64).
/// The `CompressedPostingList` stores them as compressed bytes,
/// providing transparent encode/decode on access.
///
/// # Memory Efficiency
///
/// For a sequence of N consecutive doc IDs (0, 1, 2, ..., N-1):
/// - Raw `Vec<u64>`: N × 8 bytes
/// - Compressed: ~N bytes (deltas of 1 encode as 1 byte each)
///
/// This yields up to 8× compression for dense posting lists.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressedPostingList {
    /// Delta+varint encoded posting list bytes.
    data: Vec<u8>,
}

impl CompressedPostingList {
    /// Create an empty compressed posting list.
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    /// Create a compressed posting list from a sorted sequence of doc IDs.
    ///
    /// The input MUST be sorted in ascending order (no validation).
    pub fn from_sorted(ids: &[u64]) -> Self {
        let mut buf = vec![0u8; delta::encoded_delta_size(ids)];
        let n = delta::encode_delta(ids, &mut buf);
        buf.truncate(n);
        Self { data: buf }
    }

    /// Decode and return all document IDs.
    pub fn decode(&self) -> Vec<u64> {
        delta::decode_delta(&self.data)
    }

    /// Check if the posting list is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Return the number of document IDs stored.
    pub fn len(&self) -> usize {
        if self.data.is_empty() {
            return 0;
        }
        // Count varints in the data (each varint = one doc or delta)
        varint::decode_u64_sequence(&self.data).len()
    }

    /// Return the raw compressed byte size.
    pub fn compressed_size(&self) -> usize {
        self.data.len()
    }

    /// Estimate the uncompressed size in bytes (for comparison).
    pub fn uncompressed_size(&self) -> usize {
        self.len() * 8
    }

    /// Add a single document ID to the posting list.
    ///
    /// The ID must be greater than the last ID (or the list must be empty).
    /// This is efficient for building posting lists incrementally.
    pub fn push(&mut self, id: u64) {
        let mut ids = self.decode();
        ids.push(id);
        *self = Self::from_sorted(&ids);
    }
}

impl Default for CompressedPostingList {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Vec<u64>> for CompressedPostingList {
    fn from(ids: Vec<u64>) -> Self {
        Self::from_sorted(&ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_list() {
        let list = CompressedPostingList::new();
        assert!(list.is_empty());
        assert_eq!(list.len(), 0);
        assert_eq!(list.decode(), Vec::<u64>::new());
    }

    #[test]
    fn single_id() {
        let list = CompressedPostingList::from_sorted(&[42]);
        assert_eq!(list.len(), 1);
        assert!(!list.is_empty());
        assert_eq!(list.decode(), vec![42]);
    }

    #[test]
    fn consecutive_ids() {
        let ids: Vec<u64> = (0..1000).collect();
        let list = CompressedPostingList::from_sorted(&ids);
        assert_eq!(list.len(), 1000);
        assert_eq!(list.decode(), ids);
    }

    #[test]
    fn sparse_ids() {
        let ids = vec![0u64, 100, 200, 1000, 1_000_000];
        let list = CompressedPostingList::from_sorted(&ids);
        assert_eq!(list.decode(), ids);
    }

    #[test]
    fn compression_ratio_dense() {
        let ids: Vec<u64> = (0..10000).collect();
        let list = CompressedPostingList::from_sorted(&ids);
        let raw = list.uncompressed_size();
        let compressed = list.compressed_size();
        // 10000 consecutive IDs compress to ~10KB (delta=1 = 1 byte each)
        // vs 80KB raw
        assert!(
            compressed < raw / 4,
            "dense posting lists should compress well: raw={raw}, compressed={compressed}"
        );
    }

    #[test]
    fn push_maintains_order() {
        let mut list = CompressedPostingList::new();
        list.push(1);
        list.push(5);
        list.push(10);
        assert_eq!(list.decode(), vec![1, 5, 10]);
    }

    #[test]
    fn roundtrip_from_vec() {
        let ids = vec![1u64, 2, 3, 5, 8, 13, 21];
        let list: CompressedPostingList = ids.clone().into();
        assert_eq!(list.decode(), ids);
    }

    #[test]
    fn large_id_values() {
        let ids = vec![u64::MAX - 10, u64::MAX];
        let list = CompressedPostingList::from_sorted(&ids);
        assert_eq!(list.decode(), ids);
    }
}
