use serde::{Deserialize, Serialize};

use crate::schema::field_type::FieldType;

/// Defines a single field within a schema.
///
/// Each field has a name, a type, and a required flag.
///
/// # Examples
///
/// ```
/// use pelisearch_core::schema::{Field, FieldType};
///
/// let field = Field::new("title", FieldType::Text, true);
/// assert_eq!(field.name, "title");
/// assert_eq!(field.field_type, FieldType::Text);
/// assert!(field.required);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Field {
    /// The field name.
    pub name: String,
    /// The type of data this field holds.
    pub field_type: FieldType,
    /// Whether this field is required when indexing documents.
    pub required: bool,
}

impl Field {
    /// Create a new `Field`.
    pub fn new(name: impl Into<String>, field_type: FieldType, required: bool) -> Self {
        Self {
            name: name.into(),
            field_type,
            required,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_text_field() {
        let f = Field::new("title", FieldType::Text, true);
        assert_eq!(f.name, "title");
        assert_eq!(f.field_type, FieldType::Text);
        assert!(f.required);
    }

    #[test]
    fn create_keyword_field() {
        let f = Field::new("category", FieldType::Keyword, false);
        assert_eq!(f.name, "category");
        assert_eq!(f.field_type, FieldType::Keyword);
        assert!(!f.required);
    }

    #[test]
    fn create_integer_field() {
        let f = Field::new("price", FieldType::Integer, true);
        assert_eq!(f.name, "price");
        assert_eq!(f.field_type, FieldType::Integer);
        assert!(f.required);
    }

    #[test]
    fn create_float_field() {
        let f = Field::new("rating", FieldType::Float, false);
        assert_eq!(f.name, "rating");
        assert_eq!(f.field_type, FieldType::Float);
        assert!(!f.required);
    }

    #[test]
    fn create_boolean_field() {
        let f = Field::new("active", FieldType::Boolean, true);
        assert_eq!(f.name, "active");
        assert_eq!(f.field_type, FieldType::Boolean);
        assert!(f.required);
    }

    #[test]
    fn field_debug_output() {
        let f = Field::new("title", FieldType::Text, true);
        let debug = format!("{:?}", f);
        assert!(debug.contains("title"));
        assert!(debug.contains("Text"));
    }

    #[test]
    fn field_serde_roundtrip() {
        let f = Field::new("title", FieldType::Text, true);
        let json = serde_json::to_string(&f).unwrap();
        let deserialized: Field = serde_json::from_str(&json).unwrap();
        assert_eq!(f, deserialized);
    }

    #[test]
    fn field_clone() {
        let a = Field::new("title", FieldType::Keyword, false);
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn field_required_false() {
        let f = Field::new("optional_tag", FieldType::Keyword, false);
        assert!(!f.required);
    }

    #[test]
    fn field_name_empty_allowed() {
        let f = Field::new("", FieldType::Text, false);
        assert_eq!(f.name, "");
    }
}
