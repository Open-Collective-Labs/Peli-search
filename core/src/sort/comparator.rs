use std::cmp::Ordering;

use crate::document::Document;
use crate::index::Index;
use crate::sort::sort::{SortField, SortOrder};
use crate::types::SearchHit;

/// A comparable sort value extracted from a document field.
#[derive(Debug, Clone, PartialEq)]
enum SortValue {
    Number(f64),
    String(String),
}

impl PartialOrd for SortValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (SortValue::Number(a), SortValue::Number(b)) => a.partial_cmp(b),
            (SortValue::String(a), SortValue::String(b)) => Some(a.cmp(b)),
            (SortValue::Number(_), SortValue::String(_)) => Some(Ordering::Less),
            (SortValue::String(_), SortValue::Number(_)) => Some(Ordering::Greater),
        }
    }
}

/// A sort key that distinguishes present values from missing ones.
#[derive(Debug, Clone, PartialEq)]
enum SortKey {
    Present(SortValue),
    Missing,
}

impl PartialOrd for SortKey {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match (self, other) {
            (SortKey::Present(a), SortKey::Present(b)) => a.partial_cmp(b),
            (SortKey::Present(_), SortKey::Missing) => Some(Ordering::Less),
            (SortKey::Missing, SortKey::Present(_)) => Some(Ordering::Greater),
            (SortKey::Missing, SortKey::Missing) => Some(Ordering::Equal),
        }
    }
}

/// Sort a list of search hits according to the given sort fields.
///
/// Multiple sort fields are applied in order: the first field is the primary
/// sort, the second is the secondary sort (tiebreaker), and so on. A final
/// tiebreaker on document ID ensures deterministic ordering.
///
/// Documents whose sort field is missing or not a comparable type are placed
/// after documents that have a valid value, regardless of sort direction.
///
/// # Examples
///
/// Single-field sort:
///
/// ```
/// use std::collections::HashMap;
/// use pelisearch_core::document::Document;
/// use pelisearch_core::index::Index;
/// use pelisearch_core::schema::Mapping;
/// use pelisearch_core::sort::{SortField, SortOrder};
/// use pelisearch_core::sort::comparator::sort_hits;
/// use pelisearch_core::types::SearchHit;
///
/// let mut index = Index::new("test", Mapping::new(vec![]));
///
/// let mut fields = HashMap::new();
/// fields.insert("price".to_string(), serde_json::json!(100));
/// index.add_document(Document::new("doc_100", fields).unwrap()).unwrap();
///
/// let mut fields = HashMap::new();
/// fields.insert("price".to_string(), serde_json::json!(50));
/// index.add_document(Document::new("doc_50", fields).unwrap()).unwrap();
///
/// let hits = vec![
///     SearchHit::new("test", "doc_100", 1.0),
///     SearchHit::new("test", "doc_50", 1.0),
/// ];
///
/// let sorted = sort_hits(hits, &[SortField::asc("price")], &index);
/// assert_eq!(sorted[0].document_id, "doc_50");
/// assert_eq!(sorted[1].document_id, "doc_100");
/// ```
///
/// Multi-field sort (category asc, price desc):
///
/// ```
/// use std::collections::HashMap;
/// use pelisearch_core::document::Document;
/// use pelisearch_core::index::Index;
/// use pelisearch_core::schema::Mapping;
/// use pelisearch_core::sort::{SortField, SortOrder};
/// use pelisearch_core::sort::comparator::sort_hits;
/// use pelisearch_core::types::SearchHit;
///
/// let mut index = Index::new("test", Mapping::new(vec![]));
///
/// for (id, category, price) in [
///     ("a", "electronics", 100.0),
///     ("b", "electronics", 50.0),
///     ("c", "sports", 200.0),
/// ] {
///     let mut fields = HashMap::new();
///     fields.insert("category".to_string(), serde_json::json!(category));
///     fields.insert("price".to_string(), serde_json::json!(price));
///     index.add_document(Document::new(id, fields).unwrap()).unwrap();
/// }
///
/// let hits = vec![
///     SearchHit::new("test", "a", 1.0),
///     SearchHit::new("test", "b", 1.0),
///     SearchHit::new("test", "c", 1.0),
/// ];
///
/// let sorted = sort_hits(hits, &[
///     SortField::asc("category"),
///     SortField::desc("price"),
/// ], &index);
///
/// // electronics first (asc), then sports
/// // within electronics: higher price first (desc)
/// assert_eq!(sorted[0].document_id, "a"); // electronics, 100
/// assert_eq!(sorted[1].document_id, "b"); // electronics, 50
/// assert_eq!(sorted[2].document_id, "c"); // sports, 200
/// ```
pub fn sort_hits(
    mut hits: Vec<SearchHit>,
    sort_fields: &[SortField],
    index: &Index,
) -> Vec<SearchHit> {
    if sort_fields.is_empty() {
        return hits;
    }

    hits.sort_by(|a, b| compare_hits(a, b, sort_fields, index));
    hits
}

fn compare_hits(
    a: &SearchHit,
    b: &SearchHit,
    sort_fields: &[SortField],
    index: &Index,
) -> Ordering {
    for sf in sort_fields {
        let a_doc = index.get_document(&a.document_id).ok();
        let b_doc = index.get_document(&b.document_id).ok();

        let a_key = a_doc.map_or(SortKey::Missing, |d| extract_sort_key(d, &sf.field));
        let b_key = b_doc.map_or(SortKey::Missing, |d| extract_sort_key(d, &sf.field));

        // Handle missing: both missing -> tie (fall through)
        // One missing -> present always comes first
        match (&a_key, &b_key) {
            (SortKey::Missing, SortKey::Missing) => {}
            (SortKey::Present(_), SortKey::Missing) => return Ordering::Less,
            (SortKey::Missing, SortKey::Present(_)) => return Ordering::Greater,
            (SortKey::Present(va), SortKey::Present(vb)) => {
                let cmp = match sf.order {
                    SortOrder::Asc => va.partial_cmp(vb).unwrap_or(Ordering::Equal),
                    SortOrder::Desc => vb.partial_cmp(va).unwrap_or(Ordering::Equal),
                };
                if cmp != Ordering::Equal {
                    return cmp;
                }
            }
        }
    }

    a.document_id.cmp(&b.document_id)
}

/// Extract a sort key from a document field.
///
/// Returns `SortKey::Present` for numeric and string values,
/// and `SortKey::Missing` for missing, null, or other types.
fn extract_sort_key(doc: &Document, field: &str) -> SortKey {
    match doc.get_field(field) {
        Some(serde_json::Value::Number(n)) => n
            .as_f64()
            .map_or(SortKey::Missing, |v| SortKey::Present(SortValue::Number(v))),
        Some(serde_json::Value::String(s)) => SortKey::Present(SortValue::String(s.clone())),
        _ => SortKey::Missing,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::document::Document;
    use crate::index::Index;
    use crate::schema::Mapping;
    use crate::sort::sort::SortField;
    use crate::types::SearchHit;

    use super::sort_hits;

    fn doc(id: &str, price: f64) -> Document {
        let mut fields = HashMap::new();
        fields.insert("price".to_string(), serde_json::json!(price));
        Document::new(id, fields).unwrap()
    }

    fn doc_missing(id: &str) -> Document {
        Document::new(id, HashMap::new()).unwrap()
    }

    fn doc_string_price(id: &str, price: &str) -> Document {
        let mut fields = HashMap::new();
        fields.insert("price".to_string(), serde_json::json!(price));
        Document::new(id, fields).unwrap()
    }

    fn setup_index(docs: Vec<Document>) -> Index {
        let mut index = Index::new("test", Mapping::new(vec![]));
        for d in docs {
            index.add_document(d).unwrap();
        }
        index
    }

    fn hit(doc_id: &str) -> SearchHit {
        SearchHit::new("test", doc_id, 1.0)
    }

    #[test]
    fn ascending_sort() {
        let index = setup_index(vec![doc("a", 100.0), doc("b", 50.0), doc("c", 75.0)]);
        let hits = vec![hit("a"), hit("b"), hit("c")];
        let sorted = sort_hits(hits, &[SortField::asc("price")], &index);
        assert_eq!(sorted[0].document_id, "b");
        assert_eq!(sorted[1].document_id, "c");
        assert_eq!(sorted[2].document_id, "a");
    }

    #[test]
    fn descending_sort() {
        let index = setup_index(vec![doc("a", 100.0), doc("b", 50.0), doc("c", 75.0)]);
        let hits = vec![hit("a"), hit("b"), hit("c")];
        let sorted = sort_hits(hits, &[SortField::desc("price")], &index);
        assert_eq!(sorted[0].document_id, "a");
        assert_eq!(sorted[1].document_id, "c");
        assert_eq!(sorted[2].document_id, "b");
    }

    #[test]
    fn stable_sorting_same_values() {
        let index = setup_index(vec![doc("a", 50.0), doc("b", 50.0), doc("c", 50.0)]);
        let hits = vec![hit("c"), hit("a"), hit("b")];
        let sorted = sort_hits(hits, &[SortField::asc("price")], &index);
        // Same values: tiebroken by document ID
        assert_eq!(sorted[0].document_id, "a");
        assert_eq!(sorted[1].document_id, "b");
        assert_eq!(sorted[2].document_id, "c");
    }

    #[test]
    fn missing_field_goes_last_asc() {
        let index = setup_index(vec![doc("a", 100.0), doc_missing("b")]);
        let hits = vec![hit("a"), hit("b")];
        let sorted = sort_hits(hits, &[SortField::asc("price")], &index);
        assert_eq!(sorted[0].document_id, "a");
        assert_eq!(sorted[1].document_id, "b");
    }

    #[test]
    fn missing_field_goes_last_desc() {
        let index = setup_index(vec![doc("a", 100.0), doc_missing("b")]);
        let hits = vec![hit("a"), hit("b")];
        let sorted = sort_hits(hits, &[SortField::desc("price")], &index);
        assert_eq!(sorted[0].document_id, "a");
        assert_eq!(sorted[1].document_id, "b");
    }

    #[test]
    fn all_missing_fields_retain_order() {
        let index = setup_index(vec![doc_missing("a"), doc_missing("b")]);
        let hits = vec![hit("b"), hit("a")];
        let sorted = sort_hits(hits, &[SortField::asc("price")], &index);
        // Tiebroken by doc ID
        assert_eq!(sorted[0].document_id, "a");
        assert_eq!(sorted[1].document_id, "b");
    }

    #[test]
    fn non_numeric_field_goes_last() {
        let index = setup_index(vec![
            doc("a", 100.0),
            doc_string_price("b", "not_a_number"),
        ]);
        let hits = vec![hit("a"), hit("b")];
        let sorted = sort_hits(hits, &[SortField::asc("price")], &index);
        assert_eq!(sorted[0].document_id, "a");
        assert_eq!(sorted[1].document_id, "b");
    }

    #[test]
    fn empty_sort_fields_returns_original_order() {
        let index = setup_index(vec![doc("b", 1.0), doc("a", 2.0)]);
        let hits = vec![hit("b"), hit("a")];
        let sorted = sort_hits(hits, &[], &index);
        assert_eq!(sorted[0].document_id, "b");
        assert_eq!(sorted[1].document_id, "a");
    }

    #[test]
    fn single_hit_unchanged() {
        let index = setup_index(vec![doc("a", 100.0)]);
        let hits = vec![hit("a")];
        let sorted = sort_hits(hits, &[SortField::asc("price")], &index);
        assert_eq!(sorted.len(), 1);
        assert_eq!(sorted[0].document_id, "a");
    }

    #[test]
    fn negative_values_sort_correctly() {
        let index = setup_index(vec![doc("a", -10.0), doc("b", 0.0), doc("c", -5.0)]);
        let hits = vec![hit("a"), hit("b"), hit("c")];
        let sorted_asc = sort_hits(hits.clone(), &[SortField::asc("price")], &index);
        assert_eq!(sorted_asc[0].document_id, "a");
        assert_eq!(sorted_asc[1].document_id, "c");
        assert_eq!(sorted_asc[2].document_id, "b");

        let sorted_desc = sort_hits(hits, &[SortField::desc("price")], &index);
        assert_eq!(sorted_desc[0].document_id, "b");
        assert_eq!(sorted_desc[1].document_id, "c");
        assert_eq!(sorted_desc[2].document_id, "a");
    }

    #[test]
    fn zero_values_handled() {
        let index = setup_index(vec![doc("a", 0.0), doc("b", 0.0)]);
        let hits = vec![hit("b"), hit("a")];
        let sorted = sort_hits(hits, &[SortField::asc("price")], &index);
        // Tiebroken by doc ID
        assert_eq!(sorted[0].document_id, "a");
        assert_eq!(sorted[1].document_id, "b");
    }

    #[test]
    fn multi_field_sort_category_asc_price_desc() {
        let mut index = Index::new("test", Mapping::new(vec![]));

        fn make_doc(id: &str, category: &str, price: f64) -> Document {
            let mut fields = HashMap::new();
            fields.insert("category".to_string(), serde_json::json!(category));
            fields.insert("price".to_string(), serde_json::json!(price));
            Document::new(id, fields).unwrap()
        }

        for d in [
            make_doc("a", "electronics", 100.0),
            make_doc("b", "electronics", 50.0),
            make_doc("c", "sports", 200.0),
            make_doc("d", "sports", 50.0),
        ] {
            index.add_document(d).unwrap();
        }

        let hits = vec![hit("a"), hit("b"), hit("c"), hit("d")];
        let sorted = sort_hits(
            hits,
            &[SortField::asc("category"), SortField::desc("price")],
            &index,
        );

        // electronics first (asc), then sports
        // within electronics: higher price first (desc)
        assert_eq!(sorted[0].document_id, "a"); // electronics, 100
        assert_eq!(sorted[1].document_id, "b"); // electronics, 50
        // within sports: higher price first (desc)
        assert_eq!(sorted[2].document_id, "c"); // sports, 200
        assert_eq!(sorted[3].document_id, "d"); // sports, 50
    }

    #[test]
    fn multi_field_sort_missing_secondary() {
        let mut index = Index::new("test", Mapping::new(vec![]));

        fn make_doc(id: &str, category: &str, price: f64) -> Document {
            let mut fields = HashMap::new();
            fields.insert("category".to_string(), serde_json::json!(category));
            fields.insert("price".to_string(), serde_json::json!(price));
            Document::new(id, fields).unwrap()
        }

        fn make_doc_no_price(id: &str, category: &str) -> Document {
            let mut fields = HashMap::new();
            fields.insert("category".to_string(), serde_json::json!(category));
            Document::new(id, fields).unwrap()
        }

        for d in [
            make_doc("a", "electronics", 100.0),
            make_doc_no_price("b", "electronics"),
            make_doc("c", "electronics", 50.0),
        ] {
            index.add_document(d).unwrap();
        }

        let hits = vec![hit("a"), hit("b"), hit("c")];
        let sorted = sort_hits(
            hits,
            &[SortField::asc("category"), SortField::desc("price")],
            &index,
        );

        // All same category. Prices: a=100, b=missing, c=50
        // Desc: 100 > 50, then missing goes last
        assert_eq!(sorted[0].document_id, "a"); // 100
        assert_eq!(sorted[1].document_id, "c"); // 50
        assert_eq!(sorted[2].document_id, "b"); // missing
    }
}
