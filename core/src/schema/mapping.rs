use crate::document::Document;
use crate::error::SearchError;
use crate::schema::field::Field;
use crate::schema::field_type::FieldType;

/// Defines the structure of documents for a single index.
///
/// A `Mapping` holds a collection of field definitions and provides
/// validation to ensure documents conform to the schema.
///
/// # Examples
///
/// ```
/// use pelisearch_core::schema::{Mapping, Field, FieldType};
///
/// let mapping = Mapping::new(vec![
///     Field::new("title", FieldType::Text, true),
///     Field::new("category", FieldType::Keyword, false),
/// ]);
///
/// assert!(mapping.field_exists("title"));
/// assert!(!mapping.field_exists("nonexistent"));
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct Mapping {
    fields: Vec<Field>,
}

impl Mapping {
    /// Create a new `Mapping` from a list of field definitions.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::schema::{Mapping, Field, FieldType};
    ///
    /// let mapping = Mapping::new(vec![
    ///     Field::new("title", FieldType::Text, true),
    /// ]);
    /// ```
    pub fn new(fields: Vec<Field>) -> Self {
        Self { fields }
    }

    /// Validate that a document conforms to this mapping.
    ///
    /// Checks that all required fields are present and that field
    /// values match their declared types.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::collections::HashMap;
    /// use pelisearch_core::document::Document;
    /// use pelisearch_core::schema::{Mapping, Field, FieldType};
    ///
    /// let mapping = Mapping::new(vec![
    ///     Field::new("title", FieldType::Text, true),
    ///     Field::new("count", FieldType::Integer, false),
    /// ]);
    ///
    /// let mut fields = HashMap::new();
    /// fields.insert("title".to_string(), serde_json::json!("hello"));
    /// fields.insert("count".to_string(), serde_json::json!(42));
    /// let doc = Document::new("doc1", fields).unwrap();
    ///
    /// assert!(mapping.validate_document(&doc).is_ok());
    /// ```
    pub fn validate_document(&self, doc: &Document) -> Result<(), SearchError> {
        for field in &self.fields {
            let value = doc.get_field(&field.name);

            if field.required {
                match value {
                    None => {
                        return Err(SearchError::SchemaValidationError(format!(
                            "missing required field '{}'",
                            field.name
                        )));
                    }
                    Some(v) => {
                        if !type_matches(v, &field.field_type) {
                            return Err(SearchError::SchemaValidationError(format!(
                                "field '{}' expected type {:?} but got incompatible value",
                                field.name, field.field_type
                            )));
                        }
                    }
                }
            } else if let Some(v) = value {
                if !type_matches(v, &field.field_type) {
                    return Err(SearchError::SchemaValidationError(format!(
                        "field '{}' expected type {:?} but got incompatible value",
                        field.name, field.field_type
                    )));
                }
            }
        }

        Ok(())
    }

    /// Get the field definition for a given name.
    ///
    /// Returns `None` if the field does not exist in this mapping.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::schema::{Mapping, Field, FieldType};
    ///
    /// let mapping = Mapping::new(vec![
    ///     Field::new("title", FieldType::Text, true),
    /// ]);
    ///
    /// let field = mapping.get_field("title").unwrap();
    /// assert_eq!(field.name, "title");
    /// assert_eq!(mapping.get_field("nonexistent"), None);
    /// ```
    pub fn get_field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Check whether a field name exists in this mapping.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::schema::{Mapping, Field, FieldType};
    ///
    /// let mapping = Mapping::new(vec![
    ///     Field::new("title", FieldType::Text, true),
    /// ]);
    ///
    /// assert!(mapping.field_exists("title"));
    /// assert!(!mapping.field_exists("nonexistent"));
    /// ```
    pub fn field_exists(&self, name: &str) -> bool {
        self.fields.iter().any(|f| f.name == name)
    }

    /// Return a reference to all field definitions.
    pub fn fields(&self) -> &[Field] {
        &self.fields
    }
}

fn type_matches(value: &serde_json::Value, field_type: &FieldType) -> bool {
    match field_type {
        FieldType::Text | FieldType::Keyword => value.is_string(),
        FieldType::Integer => value.is_i64(),
        FieldType::Float => value.is_number(),
        FieldType::Boolean => value.is_boolean(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_doc(
        id: &str,
        fields: Vec<(&str, serde_json::Value)>,
    ) -> Document {
        let mut map = HashMap::new();
        for (k, v) in fields {
            map.insert(k.to_string(), v);
        }
        Document::new(id, map).unwrap()
    }

    fn product_mapping() -> Mapping {
        Mapping::new(vec![
            Field::new("title", FieldType::Text, true),
            Field::new("category", FieldType::Keyword, false),
            Field::new("price", FieldType::Float, false),
            Field::new("stock", FieldType::Integer, true),
            Field::new("active", FieldType::Boolean, false),
        ])
    }

    #[test]
    fn valid_document_succeeds() {
        let mapping = product_mapping();
        let doc = make_doc(
            "prod_1",
            vec![
                ("title", serde_json::json!("Wireless Mouse")),
                ("category", serde_json::json!("electronics")),
                ("price", serde_json::json!(29.99)),
                ("stock", serde_json::json!(150)),
                ("active", serde_json::json!(true)),
            ],
        );
        assert!(mapping.validate_document(&doc).is_ok());
    }

    #[test]
    fn missing_required_field_fails() {
        let mapping = product_mapping();
        let doc = make_doc(
            "prod_1",
            vec![
                ("title", serde_json::json!("Wireless Mouse")),
                ("price", serde_json::json!(29.99)),
                // stock is missing but required
            ],
        );
        let err = mapping.validate_document(&doc).unwrap_err();
        assert!(matches!(err, SearchError::SchemaValidationError(_)));
        assert!(format!("{err}").contains("stock"));
    }

    #[test]
    fn wrong_field_type_fails() {
        let mapping = product_mapping();
        let doc = make_doc(
            "prod_1",
            vec![
                ("title", serde_json::json!("Wireless Mouse")),
                ("stock", serde_json::json!("not_an_integer")), // should be integer
            ],
        );
        let err = mapping.validate_document(&doc).unwrap_err();
        assert!(matches!(err, SearchError::SchemaValidationError(_)));
        assert!(format!("{err}").contains("stock"));
    }

    #[test]
    fn optional_field_with_wrong_type_fails() {
        let mapping = product_mapping();
        let doc = make_doc(
            "prod_1",
            vec![
                ("title", serde_json::json!("Widget")),
                ("stock", serde_json::json!(10)),
                // price should be a float, not a string
                ("price", serde_json::json!("expensive")),
            ],
        );
        let err = mapping.validate_document(&doc).unwrap_err();
        assert!(matches!(err, SearchError::SchemaValidationError(_)));
    }

    #[test]
    fn missing_optional_field_ok() {
        let mapping = product_mapping();
        let doc = make_doc(
            "prod_1",
            vec![
                ("title", serde_json::json!("Widget")),
                ("stock", serde_json::json!(10)),
                // category, price, active are optional and missing — should be ok
            ],
        );
        assert!(mapping.validate_document(&doc).is_ok());
    }

    #[test]
    fn get_field_returns_field() {
        let mapping = product_mapping();
        let title = mapping.get_field("title").unwrap();
        assert_eq!(title.name, "title");
        assert_eq!(title.field_type, FieldType::Text);
        assert!(title.required);
    }

    #[test]
    fn get_field_missing_returns_none() {
        let mapping = product_mapping();
        assert!(mapping.get_field("nonexistent").is_none());
    }

    #[test]
    fn field_exists_returns_true() {
        let mapping = product_mapping();
        assert!(mapping.field_exists("price"));
    }

    #[test]
    fn field_exists_returns_false() {
        let mapping = product_mapping();
        assert!(!mapping.field_exists("missing"));
    }

    #[test]
    fn empty_mapping_allows_any_document() {
        let mapping = Mapping::new(vec![]);
        let doc = make_doc("doc1", vec![]);
        assert!(mapping.validate_document(&doc).is_ok());

        let doc2 = make_doc("doc2", vec![("anything", serde_json::json!("value"))]);
        assert!(mapping.validate_document(&doc2).is_ok());
    }

    #[test]
    fn fields_returns_all_definitions() {
        let mapping = product_mapping();
        assert_eq!(mapping.fields().len(), 5);
    }

    #[test]
    fn boolean_field_validation() {
        let mapping = Mapping::new(vec![
            Field::new("active", FieldType::Boolean, true),
        ]);

        let doc = make_doc("doc1", vec![("active", serde_json::json!(true))]);
        assert!(mapping.validate_document(&doc).is_ok());

        let doc = make_doc("doc2", vec![("active", serde_json::json!("yes"))]);
        assert!(mapping.validate_document(&doc).is_err());
    }

    #[test]
    fn integer_field_with_float_fails() {
        let mapping = Mapping::new(vec![
            Field::new("count", FieldType::Integer, true),
        ]);

        let doc = make_doc("doc1", vec![("count", serde_json::json!(3.14))]);
        assert!(mapping.validate_document(&doc).is_err());

        let doc = make_doc("doc2", vec![("count", serde_json::json!(42))]);
        assert!(mapping.validate_document(&doc).is_ok());
    }

    #[test]
    fn float_field_accepts_integer() {
        let mapping = Mapping::new(vec![
            Field::new("price", FieldType::Float, true),
        ]);

        let doc = make_doc("doc1", vec![("price", serde_json::json!(42))]);
        // Integers are valid f64 values
        assert!(mapping.validate_document(&doc).is_ok());
    }

    #[test]
    fn mapping_debug_output() {
        let mapping = Mapping::new(vec![
            Field::new("x", FieldType::Integer, true),
        ]);
        let debug = format!("{mapping:?}");
        assert!(debug.contains("x"));
    }
}
