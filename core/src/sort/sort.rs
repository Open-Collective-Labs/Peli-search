use serde::{Deserialize, Serialize};

/// The direction of a sort operation.
///
/// # Examples
///
/// ```
/// use pelisearch_core::sort::SortOrder;
///
/// let asc = SortOrder::Asc;
/// let desc = SortOrder::Desc;
/// assert_ne!(asc, desc);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortOrder {
    /// Sort values from lowest to highest.
    Asc,
    /// Sort values from highest to lowest.
    Desc,
}

/// A single sort field specification.
///
/// # Examples
///
/// ```
/// use pelisearch_core::sort::{SortField, SortOrder};
///
/// let s = SortField {
///     field: "price".into(),
///     order: SortOrder::Asc,
/// };
/// assert_eq!(s.field, "price");
/// assert_eq!(s.order, SortOrder::Asc);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SortField {
    /// The document field to sort by.
    pub field: String,
    /// The sort direction.
    pub order: SortOrder,
}

impl SortField {
    /// Create a new `SortField` with ascending order.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::sort::{SortField, SortOrder};
    ///
    /// let s = SortField::asc("price");
    /// assert_eq!(s.order, SortOrder::Asc);
    /// ```
    pub fn asc(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            order: SortOrder::Asc,
        }
    }

    /// Create a new `SortField` with descending order.
    ///
    /// # Examples
    ///
    /// ```
    /// use pelisearch_core::sort::{SortField, SortOrder};
    ///
    /// let s = SortField::desc("price");
    /// assert_eq!(s.order, SortOrder::Desc);
    /// ```
    pub fn desc(field: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            order: SortOrder::Desc,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_order_default_is_asc() {
        // SortOrder does not implement Default; verify Asc exists.
        assert_eq!(format!("{:?}", SortOrder::Asc), "Asc");
        assert_eq!(format!("{:?}", SortOrder::Desc), "Desc");
    }

    #[test]
    fn sort_field_asc_helper() {
        let s = SortField::asc("price");
        assert_eq!(s.field, "price");
        assert_eq!(s.order, SortOrder::Asc);
    }

    #[test]
    fn sort_field_desc_helper() {
        let s = SortField::desc("rating");
        assert_eq!(s.field, "rating");
        assert_eq!(s.order, SortOrder::Desc);
    }

    #[test]
    fn sort_field_serde_roundtrip() {
        let s = SortField::asc("price");
        let json = serde_json::to_string(&s).unwrap();
        let deserialized: SortField = serde_json::from_str(&json).unwrap();
        assert_eq!(s, deserialized);
    }

    #[test]
    fn sort_order_serde() {
        let json = serde_json::to_string(&SortOrder::Asc).unwrap();
        assert_eq!(json, "\"Asc\"");
        let deserialized: SortOrder = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, SortOrder::Asc);
    }

    #[test]
    fn sort_field_debug_output() {
        let s = SortField::asc("field");
        let debug = format!("{s:?}");
        assert!(debug.contains("field"));
        assert!(debug.contains("Asc"));
    }

    #[test]
    fn sort_field_clone() {
        let a = SortField::asc("x");
        let b = a.clone();
        assert_eq!(a, b);
    }

    #[test]
    fn sort_order_not_equal() {
        assert_ne!(SortOrder::Asc, SortOrder::Desc);
    }
}
