use serde::{Deserialize, Serialize};

/// The type of data a field can hold.
///
/// Determines how the field is indexed, tokenized, and queried.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FieldType {
    /// Full-text searchable content. Tokenized during indexing.
    Text,
    /// Exact-match keyword. Stored as-is, not tokenized.
    Keyword,
    /// 64-bit signed integer.
    Integer,
    /// 64-bit floating point number.
    Float,
    /// Boolean true/false.
    Boolean,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn field_type_serde_roundtrip() {
        let cases = vec![
            FieldType::Text,
            FieldType::Keyword,
            FieldType::Integer,
            FieldType::Float,
            FieldType::Boolean,
        ];
        for ft in cases {
            let json = serde_json::to_string(&ft).unwrap();
            let deserialized: FieldType = serde_json::from_str(&json).unwrap();
            assert_eq!(ft, deserialized);
        }
    }

    #[test]
    fn field_type_debug() {
        assert_eq!(format!("{:?}", FieldType::Text), "Text");
        assert_eq!(format!("{:?}", FieldType::Keyword), "Keyword");
    }

    #[test]
    fn field_type_clone() {
        let a = FieldType::Text;
        let b = a.clone();
        assert_eq!(a, b);
    }
}
