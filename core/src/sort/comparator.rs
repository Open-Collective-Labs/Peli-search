use crate::document::Document;
use crate::index::Index;
use crate::sort::sort::{SortField, SortOrder};
use crate::types::SearchHit;

/// Sort a list of search hits according to the given sort fields.
///
/// The sorting is stable: hits with equal sort values retain their original
/// relative order (tied through document ID as a final tiebreaker).
///
/// Documents whose sort field is missing or not a comparable type are placed
/// after documents that have a valid value, regardless of sort direction.
///
/// # Examples
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
) -> std::cmp::Ordering {
    for sf in sort_fields {
        let a_doc = index.get_document(&a.document_id).ok();
        let b_doc = index.get_document(&b.document_id).ok();

        let a_val = a_doc.and_then(|d| extract_sort_value(d, &sf.field));
        let b_val = b_doc.and_then(|d| extract_sort_value(d, &sf.field));

        match (&a_val, &b_val) {
            (Some(va), Some(vb)) => {
                let cmp = match sf.order {
                    SortOrder::Asc => va.partial_cmp(vb).unwrap_or(std::cmp::Ordering::Equal),
                    SortOrder::Desc => vb.partial_cmp(va).unwrap_or(std::cmp::Ordering::Equal),
                };
                if cmp != std::cmp::Ordering::Equal {
                    return cmp;
                }
            }
            (Some(_), None) => {
                // a has value, b is missing — a comes first
                if matches!(sf.order, SortOrder::Asc | SortOrder::Desc) {
                    return std::cmp::Ordering::Less;
                }
            }
            (None, Some(_)) => {
                // a is missing, b has value — b comes first
                return std::cmp::Ordering::Greater;
            }
            (None, None) => {
                // Both missing — fall through to next sort field or tiebreaker
            }
        }
    }

    // Tiebreaker: document ID for stable ordering
    a.document_id.cmp(&b.document_id)
}

/// Extract a numeric sort value from a document field.
/// Returns `None` if the field is missing or not a number.
fn extract_sort_value(doc: &Document, field: &str) -> Option<f64> {
    match doc.get_field(field) {
        Some(serde_json::Value::Number(n)) => n.as_f64(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::document::Document;
    use crate::index::Index;
    use crate::schema::Mapping;
    use crate::sort::sort::{SortField, SortOrder};
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
}
