use crate::document::Document;

/// Determines whether a document matches a filter predicate.
pub trait FilterEvaluator {
    /// Returns `true` if the document satisfies this filter.
    fn evaluate(&self, doc: &Document) -> bool;
}
