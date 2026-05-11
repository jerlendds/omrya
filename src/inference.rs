use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{RyaIri, RyaStatement, RyaType, XSD_ANY_URI};
use crate::query::{InMemoryRyaDao, QueryOptions, StatementPattern};

pub const RDF_NAMESPACE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
pub const RDFS_NAMESPACE: &str = "http://www.w3.org/2000/01/rdf-schema#";
pub const OWL_NAMESPACE: &str = "http://www.w3.org/2002/07/owl#";
pub const SESAME_NAMESPACE: &str = "http://www.openrdf.org/schema/sesame#";

pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
pub const RDFS_SUBCLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
pub const RDFS_SUBPROPERTY_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
pub const OWL_EQUIVALENT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
pub const OWL_INVERSE_OF: &str = "http://www.w3.org/2002/07/owl#inverseOf";
pub const OWL_SAME_AS: &str = "http://www.w3.org/2002/07/owl#sameAs";
pub const OWL_SYMMETRIC_PROPERTY: &str = "http://www.w3.org/2002/07/owl#SymmetricProperty";
pub const OWL_TRANSITIVE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";

pub const INFERRED: &str = "inferred";
pub const TRUE: &str = "true";
pub const FALSE: &str = "false";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InferenceEngine {
    sub_class_of: BTreeMap<RyaIri, BTreeSet<RyaIri>>,
    sub_property_of: BTreeMap<RyaIri, BTreeSet<RyaIri>>,
    symmetric_properties: BTreeSet<RyaIri>,
    transitive_properties: BTreeSet<RyaIri>,
    inverse_of: BTreeMap<RyaIri, RyaIri>,
    same_as: BTreeMap<RyaIri, BTreeSet<RyaIri>>,
    initialized: bool,
    schedule: bool,
    refresh_graph_schedule_millis: u64,
    refresh_count: usize,
}

impl InferenceEngine {
    pub fn new() -> Self {
        Self {
            schedule: true,
            refresh_graph_schedule_millis: 5 * 60 * 1000,
            ..Self::default()
        }
    }

    pub fn init_from_dao(&mut self, dao: &InMemoryRyaDao) {
        self.refresh_from_dao(dao);
    }

    pub fn refresh_graph(&mut self, dao: &InMemoryRyaDao) {
        self.refresh_from_dao(dao);
    }

    pub fn refresh_from_dao(&mut self, dao: &InMemoryRyaDao) {
        self.sub_class_of.clear();
        self.sub_property_of.clear();
        self.symmetric_properties.clear();
        self.transitive_properties.clear();
        self.inverse_of.clear();
        self.same_as.clear();

        for statement in dao.query(&StatementPattern::default(), &QueryOptions::default()) {
            if statement.predicate.data() == RDFS_SUBCLASS_OF {
                if let Some(parent) = iri_object(&statement.object) {
                    self.sub_class_of
                        .entry(statement.subject.clone())
                        .or_default()
                        .insert(parent);
                }
            } else if statement.predicate.data() == RDFS_SUBPROPERTY_OF {
                if let Some(parent) = iri_object(&statement.object) {
                    self.sub_property_of
                        .entry(statement.subject.clone())
                        .or_default()
                        .insert(parent);
                }
            } else if statement.predicate.data() == OWL_EQUIVALENT_PROPERTY {
                if let Some(other) = iri_object(&statement.object) {
                    self.sub_property_of
                        .entry(statement.subject.clone())
                        .or_default()
                        .insert(other.clone());
                    self.sub_property_of
                        .entry(other)
                        .or_default()
                        .insert(statement.subject.clone());
                }
            } else if statement.predicate.data() == RDF_TYPE {
                if statement.object.data() == OWL_SYMMETRIC_PROPERTY {
                    self.symmetric_properties.insert(statement.subject.clone());
                } else if statement.object.data() == OWL_TRANSITIVE_PROPERTY {
                    self.transitive_properties.insert(statement.subject.clone());
                }
            } else if statement.predicate.data() == OWL_INVERSE_OF {
                if let Some(other) = iri_object(&statement.object) {
                    self.inverse_of
                        .insert(statement.subject.clone(), other.clone());
                    self.inverse_of.insert(other, statement.subject.clone());
                }
            } else if statement.predicate.data() == OWL_SAME_AS
                && let Some(other) = iri_object(&statement.object)
            {
                self.same_as
                    .entry(statement.subject.clone())
                    .or_default()
                    .insert(other.clone());
                self.same_as
                    .entry(other)
                    .or_default()
                    .insert(statement.subject.clone());
            }
        }
        self.initialized = true;
        self.refresh_count += 1;
    }

    pub fn find_subclasses_of(&self, class: &RyaIri) -> BTreeSet<RyaIri> {
        descendants(class, &self.sub_class_of)
    }

    pub fn find_instances_of_class(
        &self,
        dao: &InMemoryRyaDao,
        class: &RyaIri,
    ) -> Vec<RyaStatement> {
        let mut candidate_classes = self.find_subclasses_of(class);
        candidate_classes.insert(class.clone());
        let mut out = Vec::new();
        for candidate in candidate_classes {
            for statement in dao.query(
                &StatementPattern::new(
                    None,
                    Some(RyaIri::new(RDF_TYPE).expect("RDF type IRI")),
                    Some(candidate.into_type()),
                ),
                &QueryOptions::default(),
            ) {
                out.push(RyaStatement::new(
                    statement.subject,
                    RyaIri::new(RDF_TYPE).expect("RDF type IRI"),
                    class.clone().into_type(),
                ));
            }
        }
        out
    }

    pub fn find_subproperties_of(&self, property: &RyaIri) -> BTreeSet<RyaIri> {
        descendants(property, &self.sub_property_of)
    }

    pub fn is_symmetric_property(&self, property: &RyaIri) -> bool {
        self.symmetric_properties.contains(property)
    }

    pub fn is_transitive_property(&self, property: &RyaIri) -> bool {
        self.transitive_properties.contains(property)
    }

    pub fn find_inverse_of(&self, property: &RyaIri) -> Option<&RyaIri> {
        self.inverse_of.get(property)
    }

    pub fn find_same_as(&self, value: &RyaIri) -> BTreeSet<RyaIri> {
        let mut seen = BTreeSet::from([value.clone()]);
        let mut stack = vec![value.clone()];
        while let Some(next) = stack.pop() {
            if let Some(neighbors) = self.same_as.get(&next) {
                for neighbor in neighbors {
                    if seen.insert(neighbor.clone()) {
                        stack.push(neighbor.clone());
                    }
                }
            }
        }
        seen
    }

    pub fn find_transitive_property(
        &self,
        dao: &InMemoryRyaDao,
        subject: Option<&RyaIri>,
        property: &RyaIri,
        object: Option<&RyaIri>,
    ) -> Vec<RyaStatement> {
        if !self.is_transitive_property(property) {
            return Vec::new();
        }
        match (subject, object) {
            (Some(subject), None) => self.transitive_from_subject(dao, subject, property),
            (None, Some(object)) => self.transitive_to_object(dao, property, object),
            _ => Vec::new(),
        }
    }

    pub fn refresh_graph_schedule_millis(&self) -> u64 {
        self.refresh_graph_schedule_millis
    }

    pub fn set_refresh_graph_schedule_millis(&mut self, value: u64) {
        self.refresh_graph_schedule_millis = value;
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn refresh_count(&self) -> usize {
        self.refresh_count
    }

    pub fn destroy(&mut self) {
        self.initialized = false;
    }

    pub fn is_schedule(&self) -> bool {
        self.schedule
    }

    pub fn set_schedule(&mut self, schedule: bool) {
        self.schedule = schedule;
    }

    fn transitive_from_subject(
        &self,
        dao: &InMemoryRyaDao,
        subject: &RyaIri,
        property: &RyaIri,
    ) -> Vec<RyaStatement> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        let mut stack = vec![subject.clone()];
        while let Some(current) = stack.pop() {
            for statement in dao.query(
                &StatementPattern::new(Some(current), Some(property.clone()), None),
                &QueryOptions::default(),
            ) {
                let Some(next) = iri_object(&statement.object) else {
                    continue;
                };
                if seen.insert(next.clone()) {
                    out.push(RyaStatement::new(
                        subject.clone(),
                        property.clone(),
                        next.clone().into_type(),
                    ));
                    stack.push(next);
                }
            }
        }
        out
    }

    fn transitive_to_object(
        &self,
        dao: &InMemoryRyaDao,
        property: &RyaIri,
        object: &RyaIri,
    ) -> Vec<RyaStatement> {
        let mut out = Vec::new();
        let mut seen = BTreeSet::new();
        let mut stack = vec![object.clone()];
        while let Some(current_object) = stack.pop() {
            for statement in dao.query(
                &StatementPattern::new(
                    None,
                    Some(property.clone()),
                    Some(current_object.clone().into_type()),
                ),
                &QueryOptions::default(),
            ) {
                if seen.insert(statement.subject.clone()) {
                    out.push(RyaStatement::new(
                        statement.subject.clone(),
                        property.clone(),
                        object.clone().into_type(),
                    ));
                    stack.push(statement.subject);
                }
            }
        }
        out
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AlgebraStatementPattern {
    pub subject: Option<RyaIri>,
    pub predicate: Option<RyaIri>,
    pub object: Option<RyaType>,
    pub context: Option<RyaIri>,
    pub marker: StatementPatternMarker,
}

impl AlgebraStatementPattern {
    pub fn new(
        subject: Option<RyaIri>,
        predicate: Option<RyaIri>,
        object: Option<RyaType>,
    ) -> Self {
        Self {
            subject,
            predicate,
            object,
            context: None,
            marker: StatementPatternMarker::Plain,
        }
    }

    pub fn with_marker(mut self, marker: StatementPatternMarker) -> Self {
        self.marker = marker;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatementPatternMarker {
    Plain,
    Fixed,
    DoNotExpand,
    TransitiveProperty,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TupleExpr {
    Statement(AlgebraStatementPattern),
    FixedStatement {
        pattern: AlgebraStatementPattern,
        statements: Vec<RyaStatement>,
    },
    Join {
        left: Box<TupleExpr>,
        right: Box<TupleExpr>,
        inferred: bool,
    },
    Union {
        left: Box<TupleExpr>,
        right: Box<TupleExpr>,
        inferred: bool,
    },
    Filter {
        arg: Box<TupleExpr>,
        condition: String,
    },
    Projection(Box<TupleExpr>),
    Slice {
        arg: Box<TupleExpr>,
        limit: usize,
    },
}

impl TupleExpr {
    pub fn statement(pattern: AlgebraStatementPattern) -> Self {
        Self::Statement(pattern)
    }

    pub fn join(left: TupleExpr, right: TupleExpr) -> Self {
        Self::Join {
            left: Box::new(left),
            right: Box::new(right),
            inferred: false,
        }
    }

    pub fn infer_join(left: TupleExpr, right: TupleExpr) -> Self {
        Self::Join {
            left: Box::new(left),
            right: Box::new(right),
            inferred: true,
        }
    }

    pub fn infer_union(left: TupleExpr, right: TupleExpr) -> Self {
        Self::Union {
            left: Box::new(left),
            right: Box::new(right),
            inferred: true,
        }
    }

    pub fn is_statement(&self) -> bool {
        matches!(self, Self::Statement(_))
    }
}

pub fn reorder_join(expr: TupleExpr) -> TupleExpr {
    match expr {
        TupleExpr::Join {
            left,
            right,
            inferred,
        } => {
            let left = reorder_join(*left);
            let right = reorder_join(*right);
            if left.is_statement()
                && let TupleExpr::Join {
                    left: nested_left,
                    right: nested_right,
                    inferred: nested_inferred,
                } = right
            {
                if nested_left.is_statement() {
                    return TupleExpr::Join {
                        left: Box::new(TupleExpr::join(left, *nested_left)),
                        right: nested_right,
                        inferred: nested_inferred,
                    };
                }
                if nested_right.is_statement() {
                    return TupleExpr::Join {
                        left: Box::new(TupleExpr::join(left, *nested_right)),
                        right: nested_left,
                        inferred: nested_inferred,
                    };
                }
                return TupleExpr::Join {
                    left: Box::new(left),
                    right: Box::new(TupleExpr::Join {
                        left: nested_left,
                        right: nested_right,
                        inferred: nested_inferred,
                    }),
                    inferred,
                };
            }
            TupleExpr::Join {
                left: Box::new(left),
                right: Box::new(right),
                inferred,
            }
        }
        other => other,
    }
}

pub fn separate_filter_joins(expr: TupleExpr) -> TupleExpr {
    match expr {
        TupleExpr::Filter { arg, condition } => {
            let arg = separate_filter_joins(*arg);
            match arg {
                TupleExpr::Join {
                    left,
                    right,
                    inferred,
                } if left.is_statement() && right.is_statement() => TupleExpr::Join {
                    left: Box::new(TupleExpr::Filter {
                        arg: left,
                        condition: condition.clone(),
                    }),
                    right: Box::new(TupleExpr::Filter {
                        arg: right,
                        condition,
                    }),
                    inferred,
                },
                other => TupleExpr::Filter {
                    arg: Box::new(other),
                    condition,
                },
            }
        }
        other => other,
    }
}

pub fn should_expand_predicate(predicate: &RyaIri) -> bool {
    let value = predicate.data();
    !(value.starts_with(RDF_NAMESPACE)
        || value.starts_with(RDFS_NAMESPACE)
        || value.starts_with(SESAME_NAMESPACE))
}

fn descendants(root: &RyaIri, edges: &BTreeMap<RyaIri, BTreeSet<RyaIri>>) -> BTreeSet<RyaIri> {
    let mut out = BTreeSet::new();
    let mut stack = vec![root.clone()];
    while let Some(parent) = stack.pop() {
        for (child, parents) in edges {
            if parents.contains(&parent) && out.insert(child.clone()) {
                stack.push(child.clone());
            }
        }
    }
    out
}

fn iri_object(object: &RyaType) -> Option<RyaIri> {
    if object.data_type() == Some(XSD_ANY_URI) {
        RyaIri::new(object.data()).ok()
    } else {
        None
    }
}

#[cfg(test)]
#[path = "tests/inference_tests.rs"]
mod tests;
