use std::fmt;

/// Errors that can occur during search engine operations.
#[derive(Debug, Clone, PartialEq)]
pub enum SearchError {
    /// The document ID is empty or contains invalid characters.
    InvalidDocumentId(String),
    /// A document with the given ID already exists.
    DocumentAlreadyExists(String),
    /// No document was found with the given ID.
    DocumentNotFound(String),
    /// An internal error occurred.
    Internal(String),
    /// A document failed schema validation.
    SchemaValidationError(String),
    /// An index with the given name already exists.
    IndexAlreadyExists(String),
    /// An index with the given name was not found.
    IndexNotFound(String),
}

impl fmt::Display for SearchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDocumentId(id) => write!(f, "invalid document ID: {id}"),
            Self::DocumentAlreadyExists(id) => write!(f, "document '{id}' already exists"),
            Self::DocumentNotFound(id) => write!(f, "document '{id}' not found"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
            Self::SchemaValidationError(msg) => write!(f, "schema validation error: {msg}"),
            Self::IndexAlreadyExists(name) => write!(f, "index '{name}' already exists"),
            Self::IndexNotFound(name) => write!(f, "index '{name}' not found"),
        }
    }
}

impl std::error::Error for SearchError {}
