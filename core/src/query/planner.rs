use crate::index::Index;
use crate::query::query::Query;
use crate::tokenizer::tokenize;

/// A node in the query execution plan tree.
///
/// The planner converts a `Query` tree into an `ExecutionPlan` tree
/// that can be optimized and then executed.
#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionPlan {
    /// Return all document IDs from the index.
    ScanAll,

    /// Scan a single term's posting list.
    /// `cost` is an estimate of the number of matching documents.
    TermScan {
        term: String,
        cost: u64,
    },

    /// Full-text match scan: score all docs containing any query term using BM25.
    MatchScan {
        query_text: String,
        field: String,
    },

    /// Intersection (AND) of child plans.
    /// Children should be ordered from cheapest to most expensive.
    Intersection {
        children: Vec<ExecutionPlan>,
    },

    /// Union (OR) of child plans.
    Union {
        children: Vec<ExecutionPlan>,
    },

    /// Documents in `positive` minus documents in `negative`.
    Difference {
        positive: Box<ExecutionPlan>,
        negative: Box<ExecutionPlan>,
    },

    /// Score the results of the child plan using BM25.
    Score {
        child: Box<ExecutionPlan>,
        field: String,
        value: String,
    },

    /// Post-filter the child results.
    Filter {
        child: Box<ExecutionPlan>,
        query: Query,
    },

    /// Apply pagination (offset + limit).
    Limit {
        child: Box<ExecutionPlan>,
        from: usize,
        size: usize,
    },
}

impl ExecutionPlan {
    /// Estimate the number of results this plan will return.
    pub fn estimated_cost(&self) -> u64 {
        match self {
            ExecutionPlan::ScanAll => u64::MAX / 2,
            ExecutionPlan::TermScan { cost, .. } => *cost,
            ExecutionPlan::MatchScan { .. } => u64::MAX / 4,
            ExecutionPlan::Intersection { children } => {
                if children.is_empty() {
                    return 0;
                }
                // The cost of an intersection is bounded by the cheapest child
                children.iter().map(|c| c.estimated_cost()).min().unwrap_or(0)
            }
            ExecutionPlan::Union { children } => {
                children.iter().map(|c| c.estimated_cost()).sum()
            }
            ExecutionPlan::Difference { positive, .. } => positive.estimated_cost(),
            ExecutionPlan::Score { child, .. } => child.estimated_cost(),
            ExecutionPlan::Filter { child, .. } => child.estimated_cost(),
            ExecutionPlan::Limit { child, size, .. } => {
                std::cmp::min(child.estimated_cost(), *size as u64)
            }
        }
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        ExecutionPlan::ScanAll
    }
}

/// Converts a `Query` tree into an optimized `ExecutionPlan`.
pub struct QueryPlanner;

impl QueryPlanner {
    /// Build an execution plan from a query, using the index for cost estimates.
    pub fn plan(index: &Index, query: &Query) -> ExecutionPlan {
        Self::plan_query(index, query)
    }

    fn plan_query(index: &Index, query: &Query) -> ExecutionPlan {
        match query {
            Query::Match(mq) => {
                let tokens = tokenize(&mq.value);
                if tokens.is_empty() {
                    return ExecutionPlan::ScanAll;
                }
                if tokens.len() == 1 {
                    let cost = index
                        .inverted_index_clone()
                        .get_postings(&tokens[0])
                        .map(|p| p.len() as u64)
                        .unwrap_or(0);
                    ExecutionPlan::TermScan {
                        term: tokens[0].clone(),
                        cost,
                    }
                } else {
                    ExecutionPlan::MatchScan {
                        query_text: mq.value.clone(),
                        field: mq.field.clone(),
                    }
                }
            }
            Query::Term(_tq) => {
                ExecutionPlan::Filter {
                    child: Box::new(ExecutionPlan::ScanAll),
                    query: query.clone(),
                }
            }
            Query::Range(_) => ExecutionPlan::Filter {
                child: Box::new(ExecutionPlan::ScanAll),
                query: query.clone(),
            },
            Query::Bool(bq) => {
                let mut plan: Option<ExecutionPlan> = None;

                // must clauses → Intersection
                if !bq.must.is_empty() {
                    let mut children: Vec<ExecutionPlan> = bq
                        .must
                        .iter()
                        .map(|q| Self::plan_query(index, q))
                        .collect();
                    // Sort by estimated cost (cheapest first)
                    children.sort_by_key(|c| c.estimated_cost());
                    plan = Some(exec_intersection(children));
                }

                // filter clauses → Intersection with existing plan
                if !bq.filter.is_empty() {
                    let filter_children: Vec<ExecutionPlan> = bq
                        .filter
                        .iter()
                        .map(|q| ExecutionPlan::Filter {
                            child: Box::new(ExecutionPlan::ScanAll),
                            query: q.clone(),
                        })
                        .collect();
                    let filter_plan = exec_intersection(filter_children);
                    plan = match plan {
                        Some(p) => Some(ExecutionPlan::Intersection {
                            children: vec![p, filter_plan],
                        }),
                        None => Some(filter_plan),
                    };
                }

                // should clauses → Union (only if no must/filter)
                if !bq.should.is_empty() {
                    let should_children: Vec<ExecutionPlan> = bq
                        .should
                        .iter()
                        .map(|q| Self::plan_query(index, q))
                        .collect();
                    if plan.is_none() {
                        plan = Some(exec_union(should_children));
                    } else if bq.minimum_should_match > 0 {
                        // With minimum_should_match, treat should clauses as
                        // additional intersection constraints
                        let should_union = exec_union(should_children);
                        if let Some(p) = plan.take() {
                            plan = Some(ExecutionPlan::Intersection {
                                children: vec![p, should_union],
                            });
                        }
                    }
                }

                // must_not clauses → Difference
                if !bq.must_not.is_empty() {
                    let negative = exec_union(
                        bq.must_not
                            .iter()
                            .map(|q| Self::plan_query(index, q))
                            .collect(),
                    );
                    plan = match plan {
                        Some(p) => Some(ExecutionPlan::Difference {
                            positive: Box::new(p),
                            negative: Box::new(negative),
                        }),
                        None => Some(ExecutionPlan::Difference {
                            positive: Box::new(ExecutionPlan::ScanAll),
                            negative: Box::new(negative),
                        }),
                    };
                }

                plan.unwrap_or(ExecutionPlan::ScanAll)
            }
            Query::Phrase(pq) => {
                let tokens = tokenize(&pq.value);
                if tokens.is_empty() {
                    return ExecutionPlan::ScanAll;
                }
                // Phrase = intersection of term scans + positional filter
                let term_children: Vec<ExecutionPlan> = tokens
                    .iter()
                    .map(|t| {
                        let cost = index
                            .inverted_index_clone()
                            .get_postings(t)
                            .map(|p| p.len() as u64)
                            .unwrap_or(0);
                        ExecutionPlan::TermScan {
                            term: t.clone(),
                            cost,
                        }
                    })
                    .collect();
                ExecutionPlan::Intersection {
                    children: {
                        let mut sorted = term_children;
                        sorted.sort_by_key(|c| c.estimated_cost());
                        sorted
                    },
                }
            }
            Query::Fuzzy(fq) => {
                let tokens = tokenize(&fq.value);
                if tokens.is_empty() {
                    return ExecutionPlan::ScanAll;
                }
                ExecutionPlan::MatchScan {
                    query_text: fq.value.clone(),
                    field: fq.field.clone(),
                }
            }
            Query::Prefix(pq) => {
                ExecutionPlan::MatchScan {
                    query_text: pq.value.clone(),
                    field: pq.field.clone(),
                }
            }
            Query::ConstantScore(cs) => Self::plan_query(index, &cs.query),
            Query::DisMax(dm) => {
                let children: Vec<ExecutionPlan> = dm
                    .queries
                    .iter()
                    .map(|q| Self::plan_query(index, q))
                    .collect();
                exec_union(children)
            }
            Query::MatchAll => ExecutionPlan::ScanAll,
            Query::MatchNone => ExecutionPlan::Intersection {
                children: Vec::new(),
            },
            Query::MultiMatch(mm) => {
                // Execute multi-match as match scan, the executor handles
                // per-field boosting during scoring.
                ExecutionPlan::MatchScan {
                    query_text: mm.value.clone(),
                    field: String::new(),
                }
            }
        }
    }
}

fn exec_intersection(children: Vec<ExecutionPlan>) -> ExecutionPlan {
    if children.is_empty() {
        return ExecutionPlan::Intersection {
            children: Vec::new(),
        };
    }
    if children.len() == 1 {
        return children.into_iter().next().unwrap();
    }
    ExecutionPlan::Intersection { children }
}

fn exec_union(children: Vec<ExecutionPlan>) -> ExecutionPlan {
    if children.is_empty() {
        return ExecutionPlan::Union {
            children: Vec::new(),
        };
    }
    if children.len() == 1 {
        return children.into_iter().next().unwrap();
    }
    ExecutionPlan::Union { children }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::document::Document;
    use crate::index::Index;
    use crate::query::{
        BoolQuery, MatchQuery, PhraseQuery, Query, RangeQuery, SearchRequest, TermQuery,
    };
    use crate::schema::Mapping;

    use super::*;

    fn setup_index() -> Index {
        let mut index = Index::new("test", Mapping::new(vec![]));
        let doc = Document::new(
            "doc1",
            HashMap::from([
                ("title".to_string(), serde_json::json!("rust programming")),
                ("category".to_string(), serde_json::json!("database")),
            ]),
        )
        .unwrap();
        index.add_document(doc).unwrap();
        index
    }

    #[test]
    fn single_term_scan() {
        let index = setup_index();
        let query = Query::Term(TermQuery::new("category", "database"));
        let plan = QueryPlanner::plan(&index, &query);
        assert!(matches!(plan, ExecutionPlan::Filter { .. }));
    }

    #[test]
    fn match_query_plan() {
        let index = setup_index();
        let query = Query::Match(MatchQuery::new("title", "rust"));
        let plan = QueryPlanner::plan(&index, &query);
        assert!(matches!(plan, ExecutionPlan::TermScan { .. }));
    }

    #[test]
    fn multi_term_match_plan() {
        let index = setup_index();
        let query = Query::Match(MatchQuery::new("title", "rust programming"));
        let plan = QueryPlanner::plan(&index, &query);
        assert!(matches!(plan, ExecutionPlan::MatchScan { .. }));
    }

    #[test]
    fn bool_must_intersection() {
        let index = setup_index();
        let query = Query::Bool(
            BoolQuery::new()
                .must(Query::Match(MatchQuery::new("title", "rust")))
                .must(Query::Term(TermQuery::new("category", "database"))),
        );
        let plan = QueryPlanner::plan(&index, &query);
        assert!(matches!(plan, ExecutionPlan::Intersection { .. }));
    }

    #[test]
    fn bool_should_union() {
        let index = setup_index();
        let query = Query::Bool(
            BoolQuery::new()
                .should(Query::Match(MatchQuery::new("title", "rust")))
                .should(Query::Match(MatchQuery::new("title", "programming"))),
        );
        let plan = QueryPlanner::plan(&index, &query);
        assert!(matches!(plan, ExecutionPlan::Union { .. }));
    }

    #[test]
    fn range_query_filter() {
        let index = setup_index();
        let query = Query::Range(RangeQuery::new("price").with_lte(100.0));
        let plan = QueryPlanner::plan(&index, &query);
        assert!(matches!(plan, ExecutionPlan::Filter { .. }));
    }

    #[test]
    fn phrase_query_intersection() {
        let index = setup_index();
        let query = Query::Phrase(PhraseQuery::new("title", "rust programming"));
        let plan = QueryPlanner::plan(&index, &query);
        assert!(matches!(plan, ExecutionPlan::Intersection { .. }));
    }

    #[test]
    fn match_all_scan() {
        let index = setup_index();
        let plan = QueryPlanner::plan(&index, &Query::MatchAll);
        assert_eq!(plan, ExecutionPlan::ScanAll);
    }

    #[test]
    fn match_none_empty_intersection() {
        let index = setup_index();
        let plan = QueryPlanner::plan(&index, &Query::MatchNone);
        assert!(matches!(plan, ExecutionPlan::Intersection { .. }));
    }

    #[test]
    fn cost_estimate_cheapest_first() {
        let index = setup_index();
        let query = Query::Bool(
            BoolQuery::new()
                .must(Query::Match(MatchQuery::new("title", "rust")))
                .must(Query::Match(MatchQuery::new("title", "nonexistent"))),
        );
        let plan = QueryPlanner::plan(&index, &query);
        if let ExecutionPlan::Intersection { children } = &plan {
            // The nonexistent term has cost 0 and should be first
            if children.len() >= 2 {
                let costs: Vec<u64> = children.iter().map(|c| c.estimated_cost()).collect();
                assert!(costs.windows(2).all(|w| w[0] <= w[1]));
            }
        }
    }

    #[test]
    fn bool_all_clauses() {
        let index = setup_index();
        let query = Query::Bool(
            BoolQuery::new()
                .must(Query::Match(MatchQuery::new("title", "rust")))
                .filter(Query::Term(TermQuery::new("category", "database")))
                .should(Query::Match(MatchQuery::new("title", "programming")))
                .must_not(Query::Term(TermQuery::new("status", "deleted"))),
        );
        let plan = QueryPlanner::plan(&index, &query);
        // Should produce an Intersection or Difference at top level
        let plan_str = format!("{plan:?}");
        assert!(plan_str.contains("Intersection") || plan_str.contains("Difference"));
    }
}
