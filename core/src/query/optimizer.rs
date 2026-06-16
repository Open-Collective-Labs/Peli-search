use crate::query::planner::ExecutionPlan;

/// Optimizes an execution plan by applying a series of rewrite rules.
pub struct QueryOptimizer;

impl QueryOptimizer {
    /// Optimize an execution plan in-place.
    pub fn optimize(plan: ExecutionPlan) -> ExecutionPlan {
        let plan = Self::remove_redundancy(plan);
        let plan = Self::reorder_intersection(plan);
        plan
    }

    /// Remove redundant plan nodes:
    /// - Intersection with 0 children → empty results
    /// - Intersection with 1 child → just the child
    /// - Union with 1 child → just the child
    /// - Union with 0 children → empty results
    /// - Double Score wrapping → single Score
    fn remove_redundancy(plan: ExecutionPlan) -> ExecutionPlan {
        match plan {
            ExecutionPlan::Intersection { children } => {
                let children: Vec<ExecutionPlan> =
                    children.into_iter().map(Self::remove_redundancy).collect();
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
            ExecutionPlan::Union { children } => {
                let children: Vec<ExecutionPlan> =
                    children.into_iter().map(Self::remove_redundancy).collect();
                if children.is_empty() {
                    return ExecutionPlan::Intersection {
                        children: Vec::new(),
                    };
                }
                if children.len() == 1 {
                    return children.into_iter().next().unwrap();
                }
                ExecutionPlan::Union { children }
            }
            ExecutionPlan::Score {
                child,
                field,
                value,
            } => {
                let child = Self::remove_redundancy(*child);
                // Unwrap double Score
                if let ExecutionPlan::Score {
                    child: inner_child, ..
                } = &child
                {
                    return ExecutionPlan::Score {
                        child: inner_child.clone(),
                        field,
                        value,
                    };
                }
                ExecutionPlan::Score {
                    child: Box::new(child),
                    field,
                    value,
                }
            }
            ExecutionPlan::Filter { child, query } => {
                let child = Self::remove_redundancy(*child);
                ExecutionPlan::Filter {
                    child: Box::new(child),
                    query,
                }
            }
            ExecutionPlan::Difference { positive, negative } => {
                let positive = Self::remove_redundancy(*positive);
                let negative = Self::remove_redundancy(*negative);
                ExecutionPlan::Difference {
                    positive: Box::new(positive),
                    negative: Box::new(negative),
                }
            }
            ExecutionPlan::Limit { child, from, size } => {
                let child = Self::remove_redundancy(*child);
                ExecutionPlan::Limit {
                    child: Box::new(child),
                    from,
                    size,
                }
            }
            other => other,
        }
    }

    /// Reorder intersection children so the cheapest (lowest cost) is first.
    fn reorder_intersection(plan: ExecutionPlan) -> ExecutionPlan {
        match plan {
            ExecutionPlan::Intersection { mut children } => {
                for child in &mut children {
                    *child = Self::reorder_intersection(child.take());
                }
                children.sort_by_key(|c| c.estimated_cost());
                ExecutionPlan::Intersection { children }
            }
            ExecutionPlan::Union { children } => {
                let children: Vec<ExecutionPlan> = children
                    .into_iter()
                    .map(Self::reorder_intersection)
                    .collect();
                ExecutionPlan::Union { children }
            }
            ExecutionPlan::Score { child, field, value } => {
                let child = Self::reorder_intersection(*child);
                ExecutionPlan::Score {
                    child: Box::new(child),
                    field,
                    value,
                }
            }
            ExecutionPlan::Filter { child, query } => {
                let child = Self::reorder_intersection(*child);
                ExecutionPlan::Filter {
                    child: Box::new(child),
                    query,
                }
            }
            ExecutionPlan::Difference { positive, negative } => {
                let positive = Self::reorder_intersection(*positive);
                let negative = Self::reorder_intersection(*negative);
                ExecutionPlan::Difference {
                    positive: Box::new(positive),
                    negative: Box::new(negative),
                }
            }
            ExecutionPlan::Limit { child, from, size } => {
                let child = Self::reorder_intersection(*child);
                ExecutionPlan::Limit {
                    child: Box::new(child),
                    from,
                    size,
                }
            }
            other => other,
        }
    }
}

trait TakeExt: Sized {
    fn take(&mut self) -> Self;
}

impl<T: Default> TakeExt for T {
    fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::planner::ExecutionPlan;

    fn term_scan(term: &str, cost: u64) -> ExecutionPlan {
        ExecutionPlan::TermScan {
            term: term.to_string(),
            cost,
        }
    }

    #[test]
    fn single_child_intersection_unwrapped() {
        let plan = ExecutionPlan::Intersection {
            children: vec![term_scan("a", 10)],
        };
        let optimized = QueryOptimizer::optimize(plan);
        assert_eq!(optimized, term_scan("a", 10));
    }

    #[test]
    fn single_child_union_unwrapped() {
        let plan = ExecutionPlan::Union {
            children: vec![term_scan("a", 10)],
        };
        let optimized = QueryOptimizer::optimize(plan);
        assert_eq!(optimized, term_scan("a", 10));
    }

    #[test]
    fn empty_intersection_stays_empty() {
        let plan = ExecutionPlan::Intersection {
            children: vec![],
        };
        let optimized = QueryOptimizer::optimize(plan);
        assert!(matches!(optimized, ExecutionPlan::Intersection { .. }));
    }

    #[test]
    fn cheapest_child_first() {
        let plan = ExecutionPlan::Intersection {
            children: vec![term_scan("expensive", 1000), term_scan("cheap", 5)],
        };
        let optimized = QueryOptimizer::optimize(plan);
        if let ExecutionPlan::Intersection { children } = &optimized {
            assert_eq!(children[0].estimated_cost(), 5);
            assert_eq!(children[1].estimated_cost(), 1000);
        } else {
            panic!("expected Intersection");
        }
    }

    #[test]
    fn double_score_unwrapped() {
        let plan = ExecutionPlan::Score {
            child: Box::new(ExecutionPlan::Score {
                child: Box::new(term_scan("a", 10)),
                field: "title".to_string(),
                value: "a".to_string(),
            }),
            field: "title".to_string(),
            value: "a".to_string(),
        };
        let optimized = QueryOptimizer::optimize(plan);
        if let ExecutionPlan::Score { child, .. } = &optimized {
            assert_eq!(child.estimated_cost(), 10);
        } else {
            panic!("expected Score");
        }
    }

    #[test]
    fn nested_intersection_reordered() {
        let plan = ExecutionPlan::Intersection {
            children: vec![
                term_scan("c", 100),
                ExecutionPlan::Intersection {
                    children: vec![term_scan("b", 50), term_scan("d", 200)],
                },
            ],
        };
        let optimized = QueryOptimizer::optimize(plan);
        if let ExecutionPlan::Intersection { children } = &optimized {
            assert_eq!(children[0].estimated_cost(), 50);
            assert_eq!(children[1].estimated_cost(), 100);
        }
    }
}
