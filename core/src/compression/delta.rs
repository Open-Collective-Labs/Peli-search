/// Delta encoding for sorted sequences of unsigned integers.
///
/// Instead of storing absolute values, we store the first value followed
/// by the differences (deltas) between consecutive values. Since the
/// original sequences are sorted ascending, the deltas are small positive
/// integers that compress well with varint encoding.
///
/// # Example
///
/// ```text
/// Original:  [1, 3, 4, 10, 20]
/// Deltas:    [1, 2, 1, 6, 10]
/// ```
///
/// The first element is stored as-is, then each subsequent element is
/// the difference from the previous.

use super::varint;

/// Encode a sorted sequence of u64 values using delta + varint encoding.
///
/// Returns the encoded bytes. The sequence MUST be sorted in ascending
/// order (no validation is performed for performance).
///
/// # Panics
///
/// Panics if the buffer is too small for the encoded output.
pub fn encode_delta(values: &[u64], buf: &mut [u8]) -> usize {
    if values.is_empty() {
        return 0;
    }

    // Encode the first value as-is
    let mut offset = varint::encode_u64(values[0], buf);

    // Encode deltas for remaining values
    for i in 1..values.len() {
        let delta = values[i] - values[i - 1];
        offset += varint::encode_u64(delta, &mut buf[offset..]);
    }

    offset
}

/// Decode a delta-encoded sequence back into absolute u64 values.
///
/// Returns the decoded values.
pub fn decode_delta(buf: &[u8]) -> Vec<u64> {
    if buf.is_empty() {
        return Vec::new();
    }

    let varints = varint::decode_u64_sequence(buf);
    if varints.is_empty() {
        return Vec::new();
    }

    let mut values = Vec::with_capacity(varints.len());
    values.push(varints[0]);

    for i in 1..varints.len() {
        let delta = varints[i];
        // Check overflow — delta should not exceed what remaining space
        // there is before u64::MAX
        let next = values[i - 1].checked_add(delta).unwrap_or(u64::MAX);
        values.push(next);
    }

    values
}

/// Compute the encoded size of a sorted sequence without writing it.
pub fn encoded_delta_size(values: &[u64]) -> usize {
    if values.is_empty() {
        return 0;
    }

    let mut size = varint::encoded_size(values[0]);
    for i in 1..values.len() {
        let delta = values[i] - values[i - 1];
        size += varint::encoded_size(delta);
    }

    size
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_sorted_sequence() {
        let values = vec![1u64, 3, 4, 10, 20];
        let mut buf = vec![0u8; 64];
        let n = encode_delta(&values, &mut buf);
        let decoded = decode_delta(&buf[..n]);
        assert_eq!(decoded, values);
    }

    #[test]
    fn single_element() {
        let values = vec![42u64];
        let mut buf = vec![0u8; 64];
        let n = encode_delta(&values, &mut buf);
        let decoded = decode_delta(&buf[..n]);
        assert_eq!(decoded, vec![42]);
    }

    #[test]
    fn empty_sequence() {
        let mut buf = vec![0u8; 64];
        let n = encode_delta(&[], &mut buf);
        assert_eq!(n, 0);
        let decoded = decode_delta(&buf[..n]);
        assert!(decoded.is_empty());
    }

    #[test]
    fn consecutive_values() {
        let values: Vec<u64> = (0..100).collect();
        let mut buf = vec![0u8; 256];
        let n = encode_delta(&values, &mut buf);
        let decoded = decode_delta(&buf[..n]);
        assert_eq!(decoded, values);
    }

    #[test]
    fn sparse_values() {
        let values = vec![0u64, 1_000_000, 2_000_000, 10_000_000];
        let mut buf = vec![0u8; 64];
        let n = encode_delta(&values, &mut buf);
        let decoded = decode_delta(&buf[..n]);
        assert_eq!(decoded, values);
    }

    #[test]
    fn compression_ratio() {
        // Consecutive values should compress very well (deltas of 1)
        let values: Vec<u64> = (0..1000).collect();
        let raw_size = values.len() * 8; // 8000 bytes as raw u64
        let encoded_size = encoded_delta_size(&values);

        // Deltas of 1 encode as 1 byte each, plus the first value
        // First value 0 = 1 byte, then 999 deltas of 1 = 1 byte each = 1000 bytes
        assert!(
            encoded_size < raw_size / 4,
            "delta+varint should compress sparse sequences: raw={raw_size}, encoded={encoded_size}"
        );
    }

    #[test]
    fn single_gap() {
        let values = vec![5u64, 100];
        let mut buf = vec![0u8; 64];
        let n = encode_delta(&values, &mut buf);
        let decoded = decode_delta(&buf[..n]);
        assert_eq!(decoded, vec![5, 100]);
    }

    #[test]
    fn large_gap() {
        let values = vec![0u64, u64::MAX];
        let mut buf = vec![0u8; 64];
        let n = encode_delta(&values, &mut buf);
        let decoded = decode_delta(&buf[..n]);
        assert_eq!(decoded, vec![0, u64::MAX]);
    }
}
