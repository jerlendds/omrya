use std::collections::BTreeMap;

use crate::domain::{RyaIri, RyaType};
use crate::inference::{AlgebraStatementPattern, RDF_TYPE, StatementPatternMarker, TupleExpr};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CardinalityOf {
    Subject,
    Predicate,
    Object,
    SubjectPredicate,
    SubjectObject,
    PredicateObject,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatisticsConfig {
    pub push_empty_rdf_type_down: bool,
    pub use_composite_cardinalities: bool,
}

impl Default for StatisticsConfig {
    fn default() -> Self {
        Self {
            push_empty_rdf_type_down: true,
            use_composite_cardinalities: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RdfEvalStats {
    values: BTreeMap<(CardinalityOf, Vec<String>, Option<String>), f64>,
    default_missing: f64,
}

impl Default for RdfEvalStats {
    fn default() -> Self {
        Self {
            values: BTreeMap::new(),
            default_missing: -1.0,
        }
    }
}

impl RdfEvalStats {
    pub fn set(
        &mut self,
        cardinality_of: CardinalityOf,
        values: impl IntoIterator<Item = impl Into<String>>,
        context: Option<impl Into<String>>,
        cardinality: f64,
    ) {
        self.values.insert(
            (
                cardinality_of,
                values.into_iter().map(Into::into).collect(),
                context.map(Into::into),
            ),
            cardinality,
        );
    }

    pub fn get(
        &self,
        cardinality_of: CardinalityOf,
        values: impl IntoIterator<Item = impl Into<String>>,
        context: Option<&RyaIri>,
    ) -> f64 {
        let key = (
            cardinality_of,
            values.into_iter().map(Into::into).collect::<Vec<_>>(),
            context.map(|ctx| ctx.data().to_string()),
        );
        self.values
            .get(&key)
            .copied()
            .unwrap_or(self.default_missing)
    }
}

#[derive(Clone, Debug, Default)]
pub struct SelectivityEvalStats {
    join_selectivity: BTreeMap<(String, String), f64>,
    table_size: f64,
}

impl SelectivityEvalStats {
    pub fn with_table_size(mut self, table_size: f64) -> Self {
        self.table_size = table_size;
        self
    }

    pub fn set_join_selectivity(&mut self, left: &TupleExpr, right: &TupleExpr, selectivity: f64) {
        self.join_selectivity
            .insert((expr_key(left), expr_key(right)), selectivity);
    }

    pub fn get_join_selectivity(&self, left: &TupleExpr, right: &TupleExpr) -> f64 {
        self.join_selectivity
            .get(&(expr_key(left), expr_key(right)))
            .copied()
            .unwrap_or(0.0)
    }

    pub fn table_size(&self) -> f64 {
        self.table_size
    }
}

#[derive(Clone, Debug, Default)]
pub struct RdfStoreEvaluationStatistics {
    config: StatisticsConfig,
    stats: RdfEvalStats,
}

impl RdfStoreEvaluationStatistics {
    pub fn new(config: StatisticsConfig, stats: RdfEvalStats) -> Self {
        Self { config, stats }
    }

    pub fn get_cardinality(&self, expr: &TupleExpr) -> f64 {
        match expr {
            TupleExpr::Filter { arg, .. } => self.get_cardinality(arg) / 10.0,
            _ => self.cardinality(expr),
        }
    }

    fn cardinality(&self, expr: &TupleExpr) -> f64 {
        match expr {
            TupleExpr::Statement(pattern) => self.statement_cardinality(pattern),
            TupleExpr::FixedStatement { statements, .. } => statements.len() as f64,
            TupleExpr::Join { left, right, .. } => {
                self.cardinality(left).max(self.cardinality(right))
            }
            TupleExpr::Union { left, right, .. } => {
                self.cardinality(left) + self.cardinality(right)
            }
            TupleExpr::Filter { arg, .. } => self.get_cardinality(arg),
            TupleExpr::Projection(arg) => self.cardinality(arg) - 1.0,
            TupleExpr::Slice { limit, .. } => *limit as f64,
        }
    }

    fn statement_cardinality(&self, pattern: &AlgebraStatementPattern) -> f64 {
        if pattern.marker == StatementPatternMarker::Fixed {
            return 0.0;
        }
        if self.config.push_empty_rdf_type_down
            && pattern
                .predicate
                .as_ref()
                .is_some_and(|pred| pred.data() == RDF_TYPE)
            && pattern.subject.is_none()
            && pattern.object.is_none()
        {
            return f64::MAX;
        }

        let context = pattern.context.as_ref();
        let mut cardinality = f64::MAX - 1.0;
        if let Some(subject) = &pattern.subject {
            let mut values = vec![subject.data().to_string()];
            let mut card = CardinalityOf::Subject;
            if self.config.use_composite_cardinalities {
                if let Some(predicate) = &pattern.predicate {
                    values.push(predicate.data().to_string());
                    card = CardinalityOf::SubjectPredicate;
                } else if let Some(object) = &pattern.object {
                    values.push(object.data().to_string());
                    card = CardinalityOf::SubjectObject;
                }
            }
            cardinality = self.eval_or_one(card, values, context);
        } else if let Some(predicate) = &pattern.predicate {
            let mut values = vec![predicate.data().to_string()];
            let mut card = CardinalityOf::Predicate;
            if self.config.use_composite_cardinalities
                && let Some(object) = &pattern.object
            {
                values.push(object.data().to_string());
                card = CardinalityOf::PredicateObject;
            }
            cardinality = self.eval_or_one(card, values, context);
        } else if let Some(object) = &pattern.object {
            cardinality = self.eval_or_one(
                CardinalityOf::Object,
                vec![object.data().to_string()],
                context,
            );
        }
        cardinality
    }

    fn eval_or_one(
        &self,
        cardinality_of: CardinalityOf,
        values: Vec<String>,
        context: Option<&RyaIri>,
    ) -> f64 {
        let eval_cardinality = self.stats.get(cardinality_of, values, context);
        if eval_cardinality >= 0.0 {
            eval_cardinality
        } else {
            1.0
        }
    }
}

#[derive(Clone, Debug)]
pub struct RdfStoreSelectivityEvaluationStatistics {
    base: RdfStoreEvaluationStatistics,
    selectivity: SelectivityEvalStats,
}

impl RdfStoreSelectivityEvaluationStatistics {
    pub fn new(
        config: StatisticsConfig,
        stats: RdfEvalStats,
        selectivity: SelectivityEvalStats,
    ) -> Self {
        Self {
            base: RdfStoreEvaluationStatistics::new(config, stats),
            selectivity,
        }
    }

    pub fn get_cardinality(&self, expr: &TupleExpr) -> f64 {
        match expr {
            TupleExpr::Join { left, right, .. } => {
                if matches_fixed_do_not_expand(left, right) {
                    return self.get_cardinality(right);
                }
                let left_cost = self.get_cardinality(left);
                let right_cost = self.get_cardinality(right);
                let selectivity = self.selectivity.get_join_selectivity(left, right);
                right_cost + left_cost + left_cost * right_cost * selectivity
            }
            TupleExpr::Statement(_) => {
                let cardinality = self.base.get_cardinality(expr);
                if cardinality == f64::MAX || cardinality == f64::MAX - 1.0 {
                    self.selectivity.table_size()
                } else {
                    cardinality
                }
            }
            TupleExpr::Filter { arg, .. } => self.get_cardinality(arg) / 10.0,
            _ => self.base.get_cardinality(expr),
        }
    }
}

pub fn optimize_join_order(
    expr: TupleExpr,
    stats: &RdfStoreSelectivityEvaluationStatistics,
) -> TupleExpr {
    let mut terms = Vec::new();
    collect_join_terms(expr, &mut terms);
    if terms.len() <= 1 {
        return terms.pop().unwrap();
    }
    terms.sort_by(|left, right| {
        stats
            .get_cardinality(left)
            .partial_cmp(&stats.get_cardinality(right))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut iter = terms.into_iter();
    let first = iter.next().unwrap();
    iter.fold(first, TupleExpr::join)
}

fn collect_join_terms(expr: TupleExpr, terms: &mut Vec<TupleExpr>) {
    match expr {
        TupleExpr::Join { left, right, .. } => {
            collect_join_terms(*left, terms);
            collect_join_terms(*right, terms);
        }
        other => terms.push(other),
    }
}

fn matches_fixed_do_not_expand(left: &TupleExpr, right: &TupleExpr) -> bool {
    matches!(left, TupleExpr::FixedStatement { .. })
        && matches!(
            right,
            TupleExpr::Statement(AlgebraStatementPattern {
                marker: StatementPatternMarker::DoNotExpand,
                ..
            })
        )
}

fn expr_key(expr: &TupleExpr) -> String {
    match expr {
        TupleExpr::Statement(pattern) => pattern_key(pattern),
        TupleExpr::FixedStatement { pattern, .. } => format!("fixed:{}", pattern_key(pattern)),
        TupleExpr::Join { left, right, .. } => {
            format!("join({},{})", expr_key(left), expr_key(right))
        }
        TupleExpr::Union { left, right, .. } => {
            format!("union({},{})", expr_key(left), expr_key(right))
        }
        TupleExpr::Filter { arg, condition } => format!("filter({},{})", condition, expr_key(arg)),
        TupleExpr::Projection(arg) => format!("project({})", expr_key(arg)),
        TupleExpr::Slice { arg, limit } => format!("slice({}, {})", limit, expr_key(arg)),
    }
}

fn pattern_key(pattern: &AlgebraStatementPattern) -> String {
    format!(
        "{:?}|{:?}|{:?}",
        pattern.subject.as_ref().map(RyaIri::data),
        pattern.predicate.as_ref().map(RyaIri::data),
        pattern.object.as_ref().map(RyaType::data),
    )
}

#[cfg(test)]
#[path = "tests/evaluation_tests.rs"]
mod tests;
