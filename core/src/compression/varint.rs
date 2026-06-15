/// Variable-length integer encoding (unsigned LEB128).
///
/// Each byte uses 7 bits for data and 1 bit (MSB) as a continuation flag.
/// Values 0-127 fit in 1 byte, 128-16383 in 2 bytes, etc.
///
/// # Encoding
///
/// ```text
/// Value  →  Encoded bytes
/// 0      →  0x00
/// 1      →  0x01
/// 127    →  0x7F
/// 128    →  0x80 0x01
/// 16383  →  0xFF 0x7F
/// ```

/// Encode a u64 value into a variable-length byte sequence.
///
/// Returns the number of bytes written to `buf`. The buffer must be at
/// least 10 bytes (max varint size for u64).
pub fn encode_u64(value: u64, buf: &mut [u8]) -> usize {
    let mut v = value;
    let mut i = 0;
    loop {
        if v < 0x80 {
            buf[i] = v as u8;
            i += 1;
            break;
        }
        buf[i] = (v as u8 & 0x7F) | 0x80;
        v >>= 7;
        i += 1;
    }
    i
}

/// Decode a single u64 value from a varint-encoded byte sequence.
///
/// Returns the decoded value and the number of bytes consumed.
/// Returns `None` if the buffer is empty or malformed.
pub fn decode_u64(buf: &[u8]) -> Option<(u64, usize)> {
    if buf.is_empty() {
        return None;
    }
    let mut value: u64 = 0;
    let mut shift: u64 = 0;
    let mut consumed: usize = 0;

    for &byte in buf {
        consumed += 1;
        value |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Some((value, consumed));
        }
        shift += 7;
        if shift > 63 {
            return None;
        }
    }

    None
}

/// Encode a sequence of u64 values into a varint byte buffer.
///
/// Returns the total encoded size in bytes.
pub fn encode_u64_sequence(values: &[u64], buf: &mut [u8]) -> Option<usize> {
    let mut offset = 0;
    for &v in values {
        if offset >= buf.len() {
            return None;
        }
        offset += encode_u64(v, &mut buf[offset..]);
    }
    Some(offset)
}

/// Decode a sequence of varint-encoded u64 values from a byte buffer.
///
/// Returns the decoded values. Stops at buffer end or malformed data.
pub fn decode_u64_sequence(buf: &[u8]) -> Vec<u64> {
    let mut values = Vec::new();
    let mut offset = 0;
    while offset < buf.len() {
        match decode_u64(&buf[offset..]) {
            Some((value, consumed)) => {
                values.push(value);
                offset += consumed;
            }
            None => break,
        }
    }
    values
}

/// Compute the encoded size of a u64 value without writing it.
pub fn encoded_size(value: u64) -> usize {
    let mut size = 1;
    let mut v = value;
    while v >= 0x80 {
        v >>= 7;
        size += 1;
    }
    size
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_small_values() {
        let cases: Vec<(u64, &[u8])> = vec![(0u64, &[0x00]), (1, &[0x01]), (127, &[0x7F])];
        for (value, expected) in cases {
            let mut buf = [0u8; 10];
            let n = encode_u64(value, &mut buf);
            assert_eq!(&buf[..n], expected, "value={value}");
        }
    }

    #[test]
    fn encode_large_values() {
        let cases: Vec<(u64, &[u8])> = vec![
            (128u64, &[0x80, 0x01]),
            (16383, &[0xFF, 0x7F]),
            (16384, &[0x80, 0x80, 0x01]),
        ];
        for (value, expected) in cases {
            let mut buf = [0u8; 10];
            let n = encode_u64(value, &mut buf);
            assert_eq!(&buf[..n], expected, "value={value}");
        }
    }

    #[test]
    fn encode_max_u64() {
        let mut buf = [0u8; 10];
        let n = encode_u64(u64::MAX, &mut buf);
        // u64::MAX takes 10 bytes in LEB128
        assert_eq!(n, 10);
        let (decoded, consumed) = decode_u64(&buf[..n]).unwrap();
        assert_eq!(decoded, u64::MAX);
        assert_eq!(consumed, 10);
    }

    #[test]
    fn roundtrip_individual_values() {
        let test_values = [
            0u64,
            1,
            127,
            128,
            255,
            256,
            16383,
            16384,
            65535,
            1_000_000,
            10_000_000,
            100_000_000,
            1_000_000_000,
            u64::MAX,
        ];
        for &v in &test_values {
            let mut buf = [0u8; 10];
            let n = encode_u64(v, &mut buf);
            let (decoded, consumed) = decode_u64(&buf[..n]).unwrap();
            assert_eq!(decoded, v, "roundtrip failed for value={v}");
            assert_eq!(consumed, n);
        }
    }

    #[test]
    fn decode_empty_buffer() {
        assert_eq!(decode_u64(&[]), None);
    }

    #[test]
    fn sequence_roundtrip() {
        let values = vec![0u64, 1, 127, 128, 16383, 1_000_000, u64::MAX];
        let mut buf = vec![0u8; 128];
        let n = encode_u64_sequence(&values, &mut buf).unwrap();
        let decoded = decode_u64_sequence(&buf[..n]);
        assert_eq!(decoded, values);
    }

    #[test]
    fn encoded_size_matches_output() {
        let values = [0u64, 1, 127, 128, 255, 16383, 1_000_000, u64::MAX];
        for &v in &values {
            let mut buf = [0u8; 10];
            let n = encode_u64(v, &mut buf);
            assert_eq!(encoded_size(v), n, "encoded_size mismatch for {v}");
        }
    }

    #[test]
    fn empty_sequence() {
        let mut buf = [0u8; 10];
        let n = encode_u64_sequence(&[], &mut buf).unwrap();
        assert_eq!(n, 0);
        let decoded = decode_u64_sequence(&buf[..n]);
        assert!(decoded.is_empty());
    }
}
