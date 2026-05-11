use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
pub const RDFS_SUBCLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
pub const RDFS_SUBPROPERTY_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
pub const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
pub const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
pub const OWL_THING: &str = "http://www.w3.org/2002/07/owl#Thing";
pub const OWL_NOTHING: &str = "http://www.w3.org/2002/07/owl#Nothing";
pub const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
pub const OWL_OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
pub const OWL_DATATYPE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";
pub const OWL_TRANSITIVE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";
pub const OWL_SYMMETRIC_PROPERTY: &str = "http://www.w3.org/2002/07/owl#SymmetricProperty";
pub const OWL_FUNCTIONAL_PROPERTY: &str = "http://www.w3.org/2002/07/owl#FunctionalProperty";
pub const OWL_INVERSE_FUNCTIONAL_PROPERTY: &str =
    "http://www.w3.org/2002/07/owl#InverseFunctionalProperty";
pub const OWL_EQUIVALENT_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
pub const OWL_EQUIVALENT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
pub const OWL_INVERSE_OF: &str = "http://www.w3.org/2002/07/owl#inverseOf";
pub const OWL_DISJOINT_WITH: &str = "http://www.w3.org/2002/07/owl#disjointWith";
pub const OWL_COMPLEMENT_OF: &str = "http://www.w3.org/2002/07/owl#complementOf";
pub const OWL_ON_PROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
pub const OWL_SOME_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
pub const OWL_ALL_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#allValuesFrom";
pub const OWL_HAS_VALUE: &str = "http://www.w3.org/2002/07/owl#hasValue";
pub const OWL_MAX_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxCardinality";
pub const OWL_ASYMMETRIC_PROPERTY: &str = "http://www.w3.org/2002/07/owl#AsymmetricProperty";
pub const OWL_IRREFLEXIVE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#IrreflexiveProperty";
pub const OWL_PROPERTY_DISJOINT_WITH: &str = "http://www.w3.org/2002/07/owl#propertyDisjointWith";
pub const OWL_ON_CLASS: &str = "http://www.w3.org/2002/07/owl#onClass";
pub const OWL_MAX_QUALIFIED_CARDINALITY: &str =
    "http://www.w3.org/2002/07/owl#maxQualifiedCardinality";
pub const XSD_INT: &str = "http://www.w3.org/2001/XMLSchema#int";
pub const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

pub const RYA_REASONING_MODULE: &str = "omrya::reasoning";
pub const RYA_REASONING_ARTIFACT: &str = "omrya-reasoning";
pub const RYA_REASONING_ENGINE: &str = "omrya::reasoning::ReasoningEngine";
pub const REASONING_TEST_FEATURE: &str = "reasoning-tests";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RustDependency {
    pub crate_name: &'static str,
    pub version_req: &'static str,
    pub feature: Option<&'static str>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum RdfValue {
    Resource(String),
    Literal {
        lexical: String,
        datatype: Option<String>,
        language: Option<String>,
    },
}

impl RdfValue {
    pub fn resource(value: impl Into<String>) -> Self {
        Self::Resource(value.into())
    }

    pub fn literal(lexical: impl Into<String>, datatype: impl Into<String>) -> Self {
        Self::Literal {
            lexical: lexical.into(),
            datatype: Some(datatype.into()),
            language: None,
        }
    }

    pub fn language_literal(lexical: impl Into<String>, language: impl Into<String>) -> Self {
        Self::Literal {
            lexical: lexical.into(),
            datatype: None,
            language: Some(language.into()),
        }
    }

    pub fn string_value(&self) -> &str {
        match self {
            Self::Resource(value) => value,
            Self::Literal { lexical, .. } => lexical,
        }
    }

    pub fn as_resource(&self) -> Option<&str> {
        match self {
            Self::Resource(value) => Some(value),
            Self::Literal { .. } => None,
        }
    }

    pub fn is_literal(&self) -> bool {
        matches!(self, Self::Literal { .. })
    }

    fn encoded(&self) -> String {
        match self {
            Self::Resource(value) => value.clone(),
            Self::Literal {
                lexical,
                datatype,
                language,
            } => {
                let escaped = lexical.replace('\\', "\\\\").replace('"', "\\\"");
                if let Some(language) = language {
                    format!("\"{escaped}\"@{language}")
                } else if let Some(datatype) = datatype {
                    format!("\"{escaped}\"^^<{datatype}>")
                } else {
                    format!("\"{escaped}\"")
                }
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Triple {
    pub subject: String,
    pub predicate: String,
    pub object: RdfValue,
    pub context: Option<String>,
}

impl Triple {
    pub fn new(subject: impl Into<String>, predicate: impl Into<String>, object: RdfValue) -> Self {
        Self {
            subject: subject.into(),
            predicate: predicate.into(),
            object,
            context: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum OwlRule {
    ScmCls,
    ScmSco,
    ScmEqc1,
    ScmEqc2,
    ScmOp,
    ScmDp,
    ScmSpo,
    ScmEqp1,
    ScmEqp2,
    ScmDom1,
    ScmDom2,
    ScmRng1,
    ScmRng2,
    ScmHv,
    ScmSvf1,
    ScmSvf2,
    ScmAvf1,
    ScmAvf2,
    ClsNothing2,
    PrpIrp,
    PrpDom,
    PrpRng,
    CaxSco,
    PrpInv,
    PrpSpo1,
    PrpSymp,
    ClsSvf2,
    ClsHv2,
    ClsHv1,
    PrpAsyp,
    PrpPdw,
    CaxDw,
    ClsCom,
    ClsMaxc1,
    ClsMaxqc2,
    PrpTrp,
    ClsSvf1,
    ClsAvf,
    None,
}

impl OwlRule {
    pub fn description(self) -> &'static str {
        match self {
            Self::ClsNothing2 => "No resource can have type owl:Nothing",
            Self::PrpIrp => {
                "owl:IrreflexiveProperty -- Resource can't be related to itself via irreflexive property"
            }
            Self::PrpDom => "rdfs:domain -- Predicate's domain implies subject's type",
            Self::PrpRng => "rdfs:range -- Predicate's range implies object's type",
            Self::CaxSco => "owl:subClassOf -- Infer supertypes",
            Self::PrpInv => {
                "owl:inverseOf -- Relation via one property implies reverse relation via the inverse property"
            }
            Self::PrpSpo1 => {
                "rdfs:subPropertyOf -- Relation via subproperty implies relation via superproperty"
            }
            Self::PrpSymp => {
                "owl:SymmetricProperty -- Relation via this property is always bidirectional"
            }
            Self::ClsSvf2 => {
                "owl:someValuesFrom(owl:Thing) -- Infer membership in the set of resources related via this property to anything"
            }
            Self::ClsHv2 => {
                "owl:hasValue -- Infer membership in the set of all resources having a specific property+value"
            }
            Self::ClsHv1 => {
                "owl:hasValue -- Infer a specific property+value from the subject's membership in the set of resources with that property+value"
            }
            Self::PrpAsyp => "owl:AsymmetricProperty -- Asymmetric property can't be bidirectional",
            Self::PrpPdw => {
                "owl:propertyDisjointWith -- Two disjoint properties can't relate the same subject and object"
            }
            Self::CaxDw => "owl:disjointWith -- Resource can't belong to two disjoint classes",
            Self::ClsCom => {
                "owl:complementOf -- Resource can't belong to both a class and its complement"
            }
            Self::ClsMaxc1 => {
                "owl:maxCardinality(0) -- Max cardinality 0 for this property implies subject can't have any relation via the property"
            }
            Self::ClsMaxqc2 => {
                "owl:maxQualifiedCardinality(0/owl:Thing) -- Max cardinality 0 (with respect to owl:Thing) implies subject can't have any relation via the property"
            }
            Self::PrpTrp => "owl:TransitiveProperty -- Infer transitive relation",
            Self::ClsSvf1 => {
                "owl:someValuesFrom -- Infer membership in the set of resources related via this property to an instance of the appropriate type"
            }
            Self::ClsAvf => {
                "owl:allValuesFrom -- Infer the object's type from the subject's membership in the set of resources whose values for this property all belong to one type"
            }
            Self::None => "No rule given",
            _ => "schema rule",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Derivation {
    pub iteration: u32,
    pub rule: OwlRule,
    pub node: Option<String>,
    sources: BTreeSet<Fact>,
    source_nodes: BTreeSet<String>,
}

impl Default for Derivation {
    fn default() -> Self {
        Self {
            iteration: 0,
            rule: OwlRule::None,
            node: None,
            sources: BTreeSet::new(),
            source_nodes: BTreeSet::new(),
        }
    }
}

impl Derivation {
    pub fn new(iteration: u32, rule: OwlRule, node: impl Into<String>) -> Self {
        let node = node.into();
        Self {
            iteration,
            rule,
            node: Some(node.clone()),
            sources: BTreeSet::new(),
            source_nodes: BTreeSet::from([node]),
        }
    }

    pub fn sources(&self) -> &BTreeSet<Fact> {
        &self.sources
    }

    pub fn source_nodes(&self) -> &BTreeSet<String> {
        &self.source_nodes
    }

    pub fn add_source(&mut self, source: Fact) {
        if let Some(derivation) = &source.derivation {
            self.source_nodes
                .extend(derivation.source_nodes.iter().cloned());
        }
        self.sources.insert(source);
    }

    pub fn has_source(&self, other: &Fact) -> bool {
        self.sources
            .iter()
            .any(|source| source == other || source.has_source(other))
    }

    pub fn has_source_node(&self, node: &str) -> bool {
        self.source_nodes.contains(node)
    }

    pub fn span(&self) -> usize {
        self.source_nodes.len()
    }

    pub fn explain(&self, multiline: bool) -> String {
        let mut out = format!("[{}]", self.rule.description());
        for source in &self.sources {
            if multiline {
                out.push_str("\n   +---");
                out.push_str(&source.explain(multiline));
            } else {
                out.push_str(" (");
                out.push_str(&source.explain(false));
                out.push(')');
            }
        }
        out
    }
}

impl Ord for Derivation {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rule
            .cmp(&other.rule)
            .then_with(|| self.sources.cmp(&other.sources))
    }
}

impl PartialOrd for Derivation {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
pub struct Fact {
    pub triple: Option<Triple>,
    pub derivation: Option<Derivation>,
    useful: bool,
}

impl Default for Fact {
    fn default() -> Self {
        Self {
            triple: None,
            derivation: None,
            useful: true,
        }
    }
}

impl Fact {
    pub fn new(subject: impl Into<String>, predicate: impl Into<String>, object: RdfValue) -> Self {
        Self {
            triple: Some(Triple::new(subject, predicate, object)),
            derivation: None,
            useful: true,
        }
    }

    pub fn inferred(
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object: RdfValue,
        iteration: u32,
        rule: OwlRule,
        node: impl Into<String>,
    ) -> Self {
        Self {
            triple: Some(Triple::new(subject, predicate, object)),
            derivation: Some(Derivation::new(iteration, rule, node)),
            useful: true,
        }
    }

    pub fn empty_with_derivation(derivation: Derivation) -> Self {
        Self {
            triple: None,
            derivation: Some(derivation),
            useful: true,
        }
    }

    pub fn subject(&self) -> &str {
        &self.triple.as_ref().expect("fact has triple").subject
    }

    pub fn predicate(&self) -> &str {
        &self.triple.as_ref().expect("fact has triple").predicate
    }

    pub fn object(&self) -> &RdfValue {
        &self.triple.as_ref().expect("fact has triple").object
    }

    pub fn is_empty(&self) -> bool {
        self.triple.is_none()
    }

    pub fn is_inference(&self) -> bool {
        self.derivation.is_some()
    }

    pub fn useful(&self) -> bool {
        self.useful
    }

    pub fn set_useful(&mut self, useful: bool) {
        self.useful = useful;
    }

    pub fn iteration(&self) -> u32 {
        self.derivation.as_ref().map_or(0, |d| d.iteration)
    }

    pub fn add_source(&mut self, source: Fact) {
        self.derivation
            .get_or_insert_with(Derivation::default)
            .add_source(source);
    }

    pub fn unset_derivation(&mut self) -> Derivation {
        self.derivation.take().unwrap_or_default()
    }

    pub fn has_source(&self, other: &Fact) -> bool {
        self.derivation
            .as_ref()
            .is_some_and(|derivation| derivation.has_source(other))
    }

    pub fn has_rule(&self, rule: OwlRule) -> bool {
        self.derivation
            .as_ref()
            .is_some_and(|derivation| derivation.rule == rule)
    }

    pub fn is_cycle(&self) -> bool {
        self.derivation
            .as_ref()
            .is_some_and(|derivation| derivation.has_source(self))
    }

    pub fn span(&self) -> usize {
        if let Some(derivation) = &self.derivation {
            let mut span = derivation.span() + 1;
            if let Some(triple) = &self.triple {
                if derivation.has_source_node(&triple.subject) {
                    span -= 1;
                }
                if let Some(object) = triple.object.as_resource()
                    && derivation.has_source_node(object)
                {
                    span -= 1;
                }
            }
            span
        } else {
            1
        }
    }

    pub fn serialize_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        if let Some(triple) = &self.triple {
            let encoded = format!(
                "{}\0{}\0{}\0{}",
                triple.context.clone().unwrap_or_default(),
                triple.subject,
                triple.predicate,
                triple.object.encoded()
            );
            out.extend_from_slice(&(encoded.len() as u32).to_be_bytes());
            out.extend_from_slice(encoded.as_bytes());
        } else {
            out.extend_from_slice(&0u32.to_be_bytes());
        }
        out.push(u8::from(self.useful));
        out.push(u8::from(self.derivation.is_some()));
        out
    }

    pub fn explain(&self, multiline: bool) -> String {
        let sep = if multiline { "\n" } else { " " };
        let mut out = String::new();
        if let Some(triple) = &self.triple {
            out.push_str(&format!(
                "<{}>{sep}<{}>{sep}<{}>{sep}",
                triple.subject,
                triple.predicate,
                triple.object.string_value()
            ));
        } else {
            out.push_str("(empty)");
            out.push_str(sep);
        }
        if let Some(derivation) = &self.derivation {
            out.push_str(&derivation.explain(multiline));
        } else {
            out.push_str("[input]");
        }
        out
    }
}

impl PartialEq for Fact {
    fn eq(&self, other: &Self) -> bool {
        match (&self.triple, &other.triple) {
            (Some(left), Some(right)) => left == right,
            (None, None) => self.derivation == other.derivation,
            _ => false,
        }
    }
}

impl Eq for Fact {}

impl Ord for Fact {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.triple, &other.triple) {
            (Some(left), Some(right)) => left.cmp(right),
            (None, None) => self
                .derivation
                .as_ref()
                .unwrap_or(&Derivation::default())
                .cmp(other.derivation.as_ref().unwrap_or(&Derivation::default())),
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
        }
    }
}

impl PartialOrd for Fact {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Default)]
pub struct OwlClass {
    pub uri: String,
    super_classes: BTreeSet<String>,
    disjoint_classes: BTreeSet<String>,
    complementary_classes: BTreeSet<String>,
    svf_restrictions: BTreeSet<String>,
    avf_restrictions: BTreeSet<String>,
    qc_restrictions: BTreeSet<String>,
    properties: BTreeSet<String>,
    svf_classes: BTreeSet<String>,
    avf_classes: BTreeSet<String>,
    qc_classes: BTreeSet<String>,
    values: BTreeSet<RdfValue>,
    max_cardinality: Option<i32>,
    max_qualified_cardinality: Option<i32>,
}

impl OwlClass {
    fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            max_cardinality: None,
            max_qualified_cardinality: None,
            ..Self::default()
        }
    }

    pub fn super_classes(&self) -> BTreeSet<String> {
        let mut out = self.super_classes.clone();
        out.insert(self.uri.clone());
        out.insert(OWL_THING.to_string());
        out
    }

    pub fn equivalent_classes(&self, classes: &BTreeMap<String, OwlClass>) -> BTreeSet<String> {
        let mut out = BTreeSet::from([self.uri.clone()]);
        for candidate in &self.super_classes {
            if classes
                .get(candidate)
                .is_some_and(|other| other.super_classes.contains(&self.uri))
            {
                out.insert(candidate.clone());
            }
        }
        out
    }

    pub fn disjoint_classes(&self) -> BTreeSet<String> {
        self.disjoint_classes.clone()
    }

    pub fn complementary_classes(&self) -> BTreeSet<String> {
        self.complementary_classes.clone()
    }

    pub fn on_property(&self) -> BTreeSet<String> {
        self.properties.clone()
    }

    pub fn some_values_from(&self) -> BTreeSet<String> {
        self.svf_classes.clone()
    }

    pub fn all_values_from(&self) -> BTreeSet<String> {
        self.avf_classes.clone()
    }

    pub fn on_class(&self) -> BTreeSet<String> {
        self.qc_classes.clone()
    }

    pub fn has_value(&self) -> BTreeSet<RdfValue> {
        self.values.clone()
    }

    pub fn max_cardinality(&self) -> Option<i32> {
        self.max_cardinality
    }

    pub fn max_qualified_cardinality(&self) -> Option<i32> {
        self.max_qualified_cardinality
    }
}

#[derive(Clone, Debug, Default)]
pub struct OwlProperty {
    pub uri: String,
    transitive: bool,
    symmetric: bool,
    asymmetric: bool,
    functional: bool,
    inverse_functional: bool,
    irreflexive: bool,
    super_properties: BTreeSet<String>,
    disjoint_properties: BTreeSet<String>,
    inverse_properties: BTreeSet<String>,
    domain: BTreeSet<String>,
    range: BTreeSet<String>,
    restrictions: BTreeSet<String>,
}

impl OwlProperty {
    fn new(uri: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            ..Self::default()
        }
    }

    pub fn is_transitive(&self) -> bool {
        self.transitive
    }

    pub fn is_symmetric(&self) -> bool {
        self.symmetric
    }

    pub fn is_asymmetric(&self) -> bool {
        self.asymmetric
    }

    pub fn is_functional(&self) -> bool {
        self.functional
    }

    pub fn is_inverse_functional(&self) -> bool {
        self.inverse_functional
    }

    pub fn is_irreflexive(&self) -> bool {
        self.irreflexive
    }

    pub fn super_properties(&self) -> BTreeSet<String> {
        let mut out = self.super_properties.clone();
        out.insert(self.uri.clone());
        out
    }

    pub fn equivalent_properties(
        &self,
        properties: &BTreeMap<String, OwlProperty>,
    ) -> BTreeSet<String> {
        let mut out = BTreeSet::from([self.uri.clone()]);
        for candidate in &self.super_properties {
            if properties
                .get(candidate)
                .is_some_and(|other| other.super_properties.contains(&self.uri))
            {
                out.insert(candidate.clone());
            }
        }
        out
    }

    pub fn disjoint_properties(&self) -> BTreeSet<String> {
        self.disjoint_properties.clone()
    }

    pub fn inverse_properties(&self) -> BTreeSet<String> {
        self.inverse_properties.clone()
    }

    pub fn domain(&self) -> BTreeSet<String> {
        self.domain.clone()
    }

    pub fn range(&self) -> BTreeSet<String> {
        self.range.clone()
    }

    pub fn restrictions(&self) -> BTreeSet<String> {
        self.restrictions.clone()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Schema {
    properties: BTreeMap<String, OwlProperty>,
    classes: BTreeMap<String, OwlClass>,
}

impl Schema {
    pub fn is_schema_triple(triple: &Triple) -> bool {
        schema_predicates().contains(triple.predicate.as_str())
            || (triple.predicate == RDF_TYPE
                && triple
                    .object
                    .as_resource()
                    .is_some_and(|object| schema_types().contains(object)))
    }

    pub fn get_class(&mut self, class: impl Into<String>) -> &mut OwlClass {
        let class = class.into();
        self.classes
            .entry(class.clone())
            .or_insert_with(|| OwlClass::new(class))
    }

    pub fn class(&self, class: &str) -> Option<&OwlClass> {
        self.classes.get(class)
    }

    pub fn get_property(&mut self, property: impl Into<String>) -> &mut OwlProperty {
        let property = property.into();
        self.properties
            .entry(property.clone())
            .or_insert_with(|| OwlProperty::new(property))
    }

    pub fn property(&self, property: &str) -> Option<&OwlProperty> {
        self.properties.get(property)
    }

    pub fn has_class(&self, class: &str) -> bool {
        self.classes.contains_key(class)
    }

    pub fn has_property(&self, property: &str) -> bool {
        self.properties.contains_key(property)
    }

    pub fn has_restriction(&self, class: &str) -> bool {
        self.classes
            .get(class)
            .is_some_and(|class| !class.properties.is_empty())
    }

    pub fn process_triple(&mut self, triple: &Triple) {
        if !Self::is_schema_triple(triple) {
            return;
        }
        let subject = triple.subject.clone();
        let predicate = triple.predicate.as_str();
        let object_resource = triple.object.as_resource().map(ToOwned::to_owned);
        match predicate {
            RDF_TYPE => {
                if let Some(object) = object_resource {
                    self.add_property_type(&subject, &object);
                }
            }
            RDFS_DOMAIN => {
                if let Some(object) = object_resource
                    && object != OWL_THING
                {
                    self.get_class(object.clone());
                    self.get_property(subject).domain.insert(object);
                }
            }
            RDFS_RANGE => {
                if let Some(object) = object_resource
                    && object != OWL_THING
                {
                    self.get_class(object.clone());
                    self.get_property(subject).range.insert(object);
                }
            }
            RDFS_SUBCLASS_OF => {
                if let Some(object) = object_resource
                    && object != OWL_THING
                {
                    self.get_class(object.clone());
                    self.get_class(subject).super_classes.insert(object);
                }
            }
            RDFS_SUBPROPERTY_OF => {
                if let Some(object) = object_resource {
                    self.get_property(object.clone());
                    self.get_property(subject).super_properties.insert(object);
                }
            }
            OWL_EQUIVALENT_CLASS => {
                if let Some(object) = object_resource {
                    self.get_class(subject.clone())
                        .super_classes
                        .insert(object.clone());
                    self.get_class(object).super_classes.insert(subject);
                }
            }
            OWL_EQUIVALENT_PROPERTY => {
                if let Some(object) = object_resource {
                    self.get_property(subject.clone())
                        .super_properties
                        .insert(object.clone());
                    self.get_property(object).super_properties.insert(subject);
                }
            }
            OWL_INVERSE_OF => {
                if let Some(object) = object_resource {
                    self.get_property(subject.clone())
                        .inverse_properties
                        .insert(object.clone());
                    self.get_property(object).inverse_properties.insert(subject);
                }
            }
            OWL_COMPLEMENT_OF => {
                if let Some(object) = object_resource {
                    self.get_class(subject.clone())
                        .complementary_classes
                        .insert(object.clone());
                    self.get_class(object).complementary_classes.insert(subject);
                }
            }
            OWL_DISJOINT_WITH => {
                if let Some(object) = object_resource {
                    self.get_class(subject.clone())
                        .disjoint_classes
                        .insert(object.clone());
                    self.get_class(object).disjoint_classes.insert(subject);
                }
            }
            OWL_PROPERTY_DISJOINT_WITH => {
                if let Some(object) = object_resource {
                    self.get_property(subject.clone())
                        .disjoint_properties
                        .insert(object.clone());
                    self.get_property(object)
                        .disjoint_properties
                        .insert(subject);
                }
            }
            OWL_ON_PROPERTY => {
                if let Some(object) = object_resource {
                    self.get_property(object.clone())
                        .restrictions
                        .insert(subject.clone());
                    self.get_class(subject).properties.insert(object);
                }
            }
            OWL_SOME_VALUES_FROM => {
                if let Some(object) = object_resource {
                    self.get_class(object.clone())
                        .svf_restrictions
                        .insert(subject.clone());
                    self.get_class(subject).svf_classes.insert(object);
                }
            }
            OWL_ALL_VALUES_FROM => {
                if let Some(object) = object_resource {
                    self.get_class(object.clone())
                        .avf_restrictions
                        .insert(subject.clone());
                    self.get_class(subject).avf_classes.insert(object);
                }
            }
            OWL_ON_CLASS => {
                if let Some(object) = object_resource {
                    self.get_class(object.clone())
                        .qc_restrictions
                        .insert(subject.clone());
                    self.get_class(subject).qc_classes.insert(object);
                }
            }
            OWL_HAS_VALUE => {
                self.get_class(subject).values.insert(triple.object.clone());
            }
            OWL_MAX_CARDINALITY => {
                let value = triple.object.string_value().parse::<i32>().ok();
                if let Some(value) = value {
                    let class = self.get_class(subject);
                    if class
                        .max_cardinality
                        .is_none_or(|old| value >= 0 && value < old)
                    {
                        class.max_cardinality = Some(value);
                    }
                }
            }
            OWL_MAX_QUALIFIED_CARDINALITY => {
                let value = triple.object.string_value().parse::<i32>().ok();
                if let Some(value) = value {
                    let class = self.get_class(subject);
                    if class
                        .max_qualified_cardinality
                        .is_none_or(|old| value >= 0 && value < old)
                    {
                        class.max_qualified_cardinality = Some(value);
                    }
                }
            }
            _ => {}
        }
    }

    pub fn closure(&mut self) {
        self.compute_super_properties();
        self.compare_restrictions_by_subproperty();

        loop {
            self.compute_super_classes();
            if !self.compare_restrictions_by_same_property() {
                break;
            }
        }

        self.inherit_domain_range();
    }

    pub fn contains_triple(&self, triple: &Triple) -> bool {
        if !Self::is_schema_triple(triple) {
            return false;
        }
        let object = triple
            .object
            .as_resource()
            .unwrap_or(triple.object.string_value());
        if let Some(prop) = self.properties.get(&triple.subject) {
            if triple.predicate == RDF_TYPE {
                return (object == OWL_TRANSITIVE_PROPERTY && prop.transitive)
                    || (object == OWL_IRREFLEXIVE_PROPERTY && prop.irreflexive)
                    || (object == OWL_SYMMETRIC_PROPERTY && prop.symmetric)
                    || (object == OWL_ASYMMETRIC_PROPERTY && prop.asymmetric)
                    || (object == OWL_FUNCTIONAL_PROPERTY && prop.functional)
                    || (object == OWL_INVERSE_FUNCTIONAL_PROPERTY && prop.inverse_functional);
            }
            if (triple.predicate == RDFS_SUBPROPERTY_OF && prop.super_properties().contains(object))
                || (triple.predicate == OWL_PROPERTY_DISJOINT_WITH
                    && prop.disjoint_properties.contains(object))
                || (triple.predicate == OWL_EQUIVALENT_PROPERTY
                    && prop
                        .equivalent_properties(&self.properties)
                        .contains(object))
                || (triple.predicate == OWL_INVERSE_OF && prop.inverse_properties.contains(object))
                || (triple.predicate == RDFS_DOMAIN && prop.domain.contains(object))
                || (triple.predicate == RDFS_RANGE && prop.range.contains(object))
            {
                return true;
            }
        }
        if let Some(class) = self.classes.get(&triple.subject)
            && ((triple.predicate == OWL_EQUIVALENT_CLASS
                && class.equivalent_classes(&self.classes).contains(object))
                || (triple.predicate == OWL_DISJOINT_WITH
                    && class.disjoint_classes.contains(object))
                || (triple.predicate == OWL_COMPLEMENT_OF
                    && class.complementary_classes.contains(object))
                || (triple.predicate == RDFS_SUBCLASS_OF && class.super_classes().contains(object)))
        {
            return true;
        }
        false
    }

    pub fn summary(&self) -> String {
        let restrictions = self
            .classes
            .values()
            .filter(|class| !class.properties.is_empty())
            .count();
        format!(
            "Schema summary:\n\tClasses: {}\n\t\tProperty Restrictions: {}\n\tProperties: {}",
            self.classes.len(),
            restrictions,
            self.properties.len()
        )
    }

    pub fn explain_restriction(&self, class: &str) -> String {
        let Some(class) = self.classes.get(class) else {
            return String::new();
        };
        let mut out = "owl:Restriction".to_string();
        for property in &class.properties {
            out.push_str(&format!(" (owl:onProperty {property})"));
        }
        for value in &class.values {
            out.push_str(&format!(" (owl:hasValue {})", value.encoded()));
        }
        for target in &class.svf_classes {
            out.push_str(&format!(" (owl:someValuesFrom {target})"));
        }
        for target in &class.avf_classes {
            out.push_str(&format!(" (owl:allValuesFrom {target})"));
        }
        if let Some(value) = class.max_cardinality {
            out.push_str(&format!(" (owl:maxCardinality {value})"));
        }
        if let Some(value) = class.max_qualified_cardinality {
            out.push_str(&format!(" (owl:maxQualifiedCardinality {value}"));
            for target in &class.qc_classes {
                out.push_str(&format!(" owl:onClass {target})"));
            }
        }
        out
    }

    fn add_property_type(&mut self, property: &str, class: &str) {
        let prop = self.get_property(property.to_string());
        match class {
            OWL_TRANSITIVE_PROPERTY => prop.transitive = true,
            OWL_SYMMETRIC_PROPERTY => prop.symmetric = true,
            OWL_ASYMMETRIC_PROPERTY => prop.asymmetric = true,
            OWL_FUNCTIONAL_PROPERTY => prop.functional = true,
            OWL_INVERSE_FUNCTIONAL_PROPERTY => prop.inverse_functional = true,
            OWL_IRREFLEXIVE_PROPERTY => prop.irreflexive = true,
            _ => {}
        }
    }

    fn compute_super_properties(&mut self) {
        let snapshot = self.properties.clone();
        for (property, prop) in &snapshot {
            let ancestors = transitive_closure(&prop.super_properties, |parent| {
                snapshot
                    .get(parent)
                    .map(|p| p.super_properties.clone())
                    .unwrap_or_default()
            });
            self.get_property(property.clone()).super_properties = ancestors;
        }
    }

    fn compute_super_classes(&mut self) {
        let snapshot = self.classes.clone();
        for (class, value) in &snapshot {
            let ancestors = transitive_closure(&value.super_classes, |parent| {
                snapshot
                    .get(parent)
                    .map(|c| c.super_classes.clone())
                    .unwrap_or_default()
            });
            self.get_class(class.clone()).super_classes = ancestors;
        }
    }

    fn compare_restrictions_by_subproperty(&mut self) -> bool {
        let mut changes = Vec::new();
        let snapshot = self.classes.clone();
        for (left_id, left) in &snapshot {
            for (right_id, right) in &snapshot {
                if restrictions_on_subproperty(left, right, self) {
                    if !left.values.is_empty() && !left.values.is_disjoint(&right.values) {
                        changes.push((left_id.clone(), right_id.clone()));
                    } else if !left.svf_classes.is_empty()
                        && !left.svf_classes.is_disjoint(&right.svf_classes)
                    {
                        changes.push((left_id.clone(), right_id.clone()));
                    }
                    if !left.avf_classes.is_empty()
                        && !left.avf_classes.is_disjoint(&right.avf_classes)
                    {
                        changes.push((right_id.clone(), left_id.clone()));
                    }
                }
            }
        }
        self.apply_class_edges(changes)
    }

    fn compare_restrictions_by_same_property(&mut self) -> bool {
        let mut changes = Vec::new();
        let snapshot = self.classes.clone();
        for prop in self.properties.values() {
            let restrictions: Vec<_> = prop.restrictions.iter().collect();
            for left_id in &restrictions {
                for right_id in &restrictions {
                    if left_id == right_id {
                        continue;
                    }
                    let Some(left) = snapshot.get(*left_id) else {
                        continue;
                    };
                    let Some(right) = snapshot.get(*right_id) else {
                        continue;
                    };
                    if restriction_target_subclass(left, right, self) {
                        changes.push(((*left_id).clone(), (*right_id).clone()));
                    }
                }
            }
        }
        self.apply_class_edges(changes)
    }

    fn apply_class_edges(&mut self, changes: Vec<(String, String)>) -> bool {
        let mut changed = false;
        for (left, right) in changes {
            changed |= self.get_class(left).super_classes.insert(right);
        }
        changed
    }

    fn inherit_domain_range(&mut self) {
        let snapshot = self.properties.clone();
        let classes = self.classes.clone();
        for (property, prop) in snapshot {
            let mut domain = prop.domain.clone();
            let mut range = prop.range.clone();
            for parent in &prop.super_properties {
                if let Some(parent_prop) = self.properties.get(parent) {
                    domain.extend(parent_prop.domain.iter().cloned());
                    range.extend(parent_prop.range.iter().cloned());
                }
            }
            for class in domain.clone() {
                if let Some(schema_class) = classes.get(&class) {
                    domain.extend(schema_class.super_classes());
                }
            }
            for class in range.clone() {
                if let Some(schema_class) = classes.get(&class) {
                    range.extend(schema_class.super_classes());
                }
            }
            domain.remove(OWL_THING);
            range.remove(OWL_THING);
            let prop = self.get_property(property);
            prop.domain = domain;
            prop.range = range;
        }
    }
}

fn transitive_closure<F>(initial: &BTreeSet<String>, mut next: F) -> BTreeSet<String>
where
    F: FnMut(&str) -> BTreeSet<String>,
{
    let mut ancestors = BTreeSet::new();
    let mut frontier = initial.clone();
    while let Some(item) = frontier.pop_first() {
        if ancestors.insert(item.clone()) {
            frontier.extend(next(&item).difference(&ancestors).cloned());
        }
    }
    ancestors
}

fn restrictions_on_subproperty(left: &OwlClass, right: &OwlClass, schema: &Schema) -> bool {
    left.properties.iter().any(|property| {
        schema.properties.get(property).is_some_and(|prop| {
            prop.super_properties()
                .iter()
                .any(|p| right.properties.contains(p))
        })
    })
}

fn restriction_target_subclass(left: &OwlClass, right: &OwlClass, schema: &Schema) -> bool {
    let svf_matches = left.svf_classes.iter().any(|target| {
        schema
            .classes
            .get(target)
            .map_or(
                BTreeSet::from([target.clone(), OWL_THING.to_string()]),
                |class| class.super_classes(),
            )
            .iter()
            .any(|superclass| right.svf_classes.contains(superclass))
    });
    let avf_matches = left.avf_classes.iter().any(|target| {
        schema
            .classes
            .get(target)
            .map_or(
                BTreeSet::from([target.clone(), OWL_THING.to_string()]),
                |class| class.super_classes(),
            )
            .iter()
            .any(|superclass| right.avf_classes.contains(superclass))
    });
    svf_matches || avf_matches
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Relevance {
    None,
    Subject,
    Object,
    Both,
}

impl Relevance {
    fn get(subject: bool, object: bool) -> Self {
        match (subject, object) {
            (true, true) => Self::Both,
            (true, false) => Self::Subject,
            (false, true) => Self::Object,
            (false, false) => Self::None,
        }
    }

    pub fn subject(self) -> bool {
        matches!(self, Self::Subject | Self::Both)
    }

    pub fn object(self) -> bool {
        matches!(self, Self::Object | Self::Both)
    }
}

#[derive(Clone, Debug)]
pub struct LocalReasoner {
    node: String,
    schema: Schema,
    current_iteration: u32,
    min_iteration: u32,
    new_facts: BTreeSet<Fact>,
    inconsistencies: BTreeSet<Derivation>,
    known_types: BTreeMap<String, Fact>,
    possible_inferences: BTreeMap<String, Vec<Fact>>,
    possible_inconsistencies: BTreeMap<String, Vec<Derivation>>,
    transitive_incoming: BTreeMap<String, Vec<Fact>>,
    asymmetric_incoming: BTreeMap<String, Vec<Fact>>,
    disjoint_outgoing: BTreeMap<String, Vec<Fact>>,
    min_transitive_left: usize,
    min_transitive_right: usize,
}

impl LocalReasoner {
    pub fn new(
        node: impl Into<String>,
        schema: Schema,
        iteration: u32,
        schema_update: u32,
    ) -> Self {
        let node = node.into();
        let n = iteration.saturating_sub(schema_update).max(1);
        Self {
            node,
            schema,
            current_iteration: iteration,
            min_iteration: if schema_update < iteration.saturating_sub(1) {
                0
            } else {
                iteration.saturating_sub(1)
            },
            new_facts: BTreeSet::new(),
            inconsistencies: BTreeSet::new(),
            known_types: BTreeMap::new(),
            possible_inferences: BTreeMap::new(),
            possible_inconsistencies: BTreeMap::new(),
            transitive_incoming: BTreeMap::new(),
            asymmetric_incoming: BTreeMap::new(),
            disjoint_outgoing: BTreeMap::new(),
            min_transitive_left: 2usize.pow(n - 1),
            min_transitive_right: 1,
        }
    }

    pub fn relevant_fact(fact: &Fact, schema: &Schema) -> Relevance {
        let Some(triple) = &fact.triple else {
            return Relevance::None;
        };
        if Schema::is_schema_triple(triple) {
            return Relevance::None;
        }
        let mut subject = false;
        let mut object = false;
        let literal_object = triple.object.is_literal();
        if triple.predicate == RDF_TYPE
            && let Some(type_uri) = triple.object.as_resource()
            && (type_uri == OWL_NOTHING || schema.has_class(type_uri))
        {
            subject = true;
        }
        if let Some(prop) = schema.property(&triple.predicate) {
            if prop.asymmetric || prop.transitive || !prop.restrictions.is_empty() {
                subject = true;
                object = !literal_object;
            }
            if !subject
                && (!prop.domain.is_empty()
                    || prop.super_properties().len() > 1
                    || !prop.disjoint_properties.is_empty())
            {
                subject = true;
            }
            if !literal_object
                && !object
                && (!prop.range.is_empty()
                    || !prop.inverse_properties.is_empty()
                    || prop.symmetric
                    || (prop.irreflexive
                        && triple
                            .object
                            .as_resource()
                            .is_some_and(|o| o == triple.subject)))
            {
                object = true;
            }
        }
        Relevance::get(subject, object)
    }

    pub fn relevant_join_rule(fact: &Fact, schema: &Schema) -> Relevance {
        let Some(triple) = &fact.triple else {
            return Relevance::None;
        };
        if Schema::is_schema_triple(triple) {
            return Relevance::None;
        }
        let mut subject = false;
        let mut object = false;
        let literal_object = triple.object.is_literal();
        if triple.predicate == RDF_TYPE
            && let Some(type_uri) = triple.object.as_resource()
            && let Some(class) = schema.class(type_uri)
            && (!class.properties.is_empty()
                || !class.svf_restrictions.is_empty()
                || !class.avf_restrictions.is_empty()
                || !class.qc_restrictions.is_empty()
                || !class.disjoint_classes.is_empty()
                || !class.complementary_classes.is_empty())
        {
            subject = true;
        }
        if let Some(prop) = schema.property(&triple.predicate) {
            if prop.transitive {
                subject = true;
                object = !literal_object;
            } else {
                if !prop.disjoint_properties.is_empty() {
                    subject = true;
                }
                for restriction_id in &prop.restrictions {
                    let Some(restriction) = schema.class(restriction_id) else {
                        continue;
                    };
                    if !restriction.avf_classes.is_empty() {
                        subject = true;
                    }
                    if !literal_object
                        && (restriction.max_cardinality.is_some()
                            || restriction.max_qualified_cardinality.is_some()
                            || !restriction.svf_classes.is_empty())
                    {
                        object = true;
                    }
                }
            }
        }
        Relevance::get(subject, object)
    }

    pub fn process_fact(&mut self, fact: Fact) {
        let subject = fact.subject().to_string();
        let predicate = fact.predicate().to_string();
        let object_resource = fact.object().as_resource().map(ToOwned::to_owned);
        let recursive = fact.iteration() == self.current_iteration;
        let incoming = object_resource.as_deref() == Some(self.node.as_str());
        let outgoing = subject == self.node;
        let skip_reflexive = incoming && outgoing && recursive;

        if incoming && !skip_reflexive {
            self.process_incoming(fact.clone());
        }
        if outgoing {
            if predicate == RDF_TYPE {
                self.process_type(fact);
            } else {
                self.process_outgoing(fact);
            }
        }

        let results = self.take_facts_unmarked();
        for fact in &results {
            self.process_fact(fact.clone());
        }
        self.new_facts.extend(results);
    }

    pub fn collect_types(&mut self) {
        for fact in self.known_types.values().cloned().collect::<Vec<_>>() {
            self.collect(fact);
        }
    }

    pub fn has_new_facts(&self) -> bool {
        !self.new_facts.is_empty()
    }

    pub fn has_inconsistencies(&self) -> bool {
        !self.inconsistencies.is_empty()
    }

    pub fn facts(&mut self) -> BTreeSet<Fact> {
        self.take_facts_unmarked()
            .into_iter()
            .map(|mut fact| {
                fact.set_useful(self.relevant_to_future(&fact));
                fact
            })
            .collect()
    }

    pub fn inconsistencies(&mut self) -> BTreeSet<Derivation> {
        std::mem::take(&mut self.inconsistencies)
    }

    pub fn known_types(&self) -> &BTreeMap<String, Fact> {
        &self.known_types
    }

    pub fn get_num_stored(&self) -> usize {
        self.known_types.len()
            + self
                .possible_inferences
                .values()
                .map(Vec::len)
                .sum::<usize>()
            + self
                .possible_inconsistencies
                .values()
                .map(Vec::len)
                .sum::<usize>()
            + self
                .transitive_incoming
                .values()
                .map(Vec::len)
                .sum::<usize>()
            + self
                .asymmetric_incoming
                .values()
                .map(Vec::len)
                .sum::<usize>()
            + self.disjoint_outgoing.values().map(Vec::len).sum::<usize>()
    }

    fn take_facts_unmarked(&mut self) -> BTreeSet<Fact> {
        std::mem::take(&mut self.new_facts)
    }

    fn process_outgoing(&mut self, fact: Fact) {
        let pred = fact.predicate().to_string();
        let object = fact.object().clone();
        let prop = self
            .schema
            .property(&pred)
            .cloned()
            .unwrap_or_else(|| OwlProperty::new(&pred));

        for type_uri in &prop.domain {
            self.process_generated_type(type_uri.clone(), OwlRule::PrpDom, fact.clone());
        }

        if !fact.has_rule(OwlRule::PrpSpo1) {
            for super_prop in prop.super_properties() {
                if super_prop != pred {
                    self.collect(self.triple(
                        &self.node,
                        &super_prop,
                        object.clone(),
                        OwlRule::PrpSpo1,
                        fact.clone(),
                    ));
                }
            }
        }

        for disjoint in &prop.disjoint_properties {
            if let Some(others) = self.disjoint_outgoing.get(disjoint).cloned() {
                for other in others {
                    if other.object() == &object {
                        let mut inc = self.inconsistency(OwlRule::PrpPdw, fact.clone());
                        inc.add_source(other);
                        self.collect_inconsistency(inc);
                    }
                }
            }
        }
        if !prop.disjoint_properties.is_empty() {
            self.disjoint_outgoing
                .entry(pred.clone())
                .or_default()
                .push(fact.clone());
        }

        for restriction_id in &prop.restrictions {
            let Some(restriction) = self.schema.class(restriction_id).cloned() else {
                continue;
            };
            if restriction.svf_classes.contains(OWL_THING) {
                self.process_generated_type(restriction_id.clone(), OwlRule::ClsSvf2, fact.clone());
            }
            if restriction.values.contains(&object) {
                self.process_generated_type(restriction_id.clone(), OwlRule::ClsHv2, fact.clone());
            }
            for class in &restriction.avf_classes {
                if let Some(resource_object) = object.as_resource() {
                    let out = self.triple(
                        resource_object,
                        RDF_TYPE,
                        RdfValue::resource(class),
                        OwlRule::ClsAvf,
                        fact.clone(),
                    );
                    self.on_type(restriction_id.clone(), out);
                }
            }
            if restriction.max_cardinality == Some(0) {
                let inc = self.inconsistency(OwlRule::ClsMaxc1, fact.clone());
                self.inconsistent_on_type(restriction_id.clone(), inc);
            }
            if restriction.max_qualified_cardinality == Some(0)
                && restriction.qc_classes.contains(OWL_THING)
            {
                let inc = self.inconsistency(OwlRule::ClsMaxqc2, fact.clone());
                self.inconsistent_on_type(restriction_id.clone(), inc);
            }
        }

        if prop.transitive
            && object.as_resource() != Some(self.node.as_str())
            && fact.span() >= self.min_transitive_right
            && let Some(others) = self.transitive_incoming.get(&pred).cloned()
        {
            for other in others {
                let mut out = self.triple(
                    other.subject(),
                    &pred,
                    object.clone(),
                    OwlRule::PrpTrp,
                    fact.clone(),
                );
                out.add_source(other);
                self.collect(out);
            }
        }

        if prop.asymmetric
            && let Some(others) = self.asymmetric_incoming.get(&pred).cloned()
        {
            for other in others {
                if object.as_resource() == Some(other.subject()) {
                    let mut inc = self.inconsistency(OwlRule::PrpAsyp, fact.clone());
                    inc.add_source(other);
                    self.collect_inconsistency(inc);
                }
            }
        }
    }

    fn process_incoming(&mut self, fact: Fact) {
        let subject = fact.subject().to_string();
        let pred = fact.predicate().to_string();
        let prop = self
            .schema
            .property(&pred)
            .cloned()
            .unwrap_or_else(|| OwlProperty::new(&pred));

        for type_uri in &prop.range {
            self.process_generated_type(type_uri.clone(), OwlRule::PrpRng, fact.clone());
        }
        for inverse in &prop.inverse_properties {
            self.collect(self.triple(
                &self.node,
                inverse,
                RdfValue::resource(subject.clone()),
                OwlRule::PrpInv,
                fact.clone(),
            ));
        }
        if prop.symmetric && !fact.has_rule(OwlRule::PrpSymp) && subject != self.node {
            self.collect(self.triple(
                &self.node,
                &pred,
                RdfValue::resource(subject.clone()),
                OwlRule::PrpSymp,
                fact.clone(),
            ));
        }
        if prop.irreflexive && subject == self.node {
            let inc = self.inconsistency(OwlRule::PrpIrp, fact.clone());
            self.collect_inconsistency(inc);
        }
        if prop.transitive && subject != self.node && fact.span() >= self.min_transitive_left {
            self.transitive_incoming
                .entry(pred.clone())
                .or_default()
                .push(fact.clone());
        }
        if prop.asymmetric {
            self.asymmetric_incoming
                .entry(pred)
                .or_default()
                .push(fact.clone());
        }
        for restriction_id in &prop.restrictions {
            let svf_classes = self
                .schema
                .class(restriction_id)
                .map(|restriction| restriction.svf_classes.clone())
                .unwrap_or_default();
            for class in svf_classes
                .iter()
                .filter(|class| class.as_str() != OWL_THING)
            {
                let out = self.triple(
                    &subject,
                    RDF_TYPE,
                    RdfValue::resource(restriction_id),
                    OwlRule::ClsSvf1,
                    fact.clone(),
                );
                self.on_type(class.clone(), out);
            }
        }
    }

    fn process_type(&mut self, fact: Fact) {
        let Some(type_uri) = fact.object().as_resource().map(ToOwned::to_owned) else {
            return;
        };
        let new_type = !self.known_types.contains_key(&type_uri);
        let should_replace = new_type
            || self
                .known_types
                .get(&type_uri)
                .is_some_and(|old| fact.iteration() < old.iteration());
        if should_replace {
            self.known_types.insert(type_uri, fact.clone());
            self.type_inference(fact);
        }
    }

    fn process_generated_type(&mut self, type_uri: String, rule: OwlRule, source: Fact) {
        let fact = self.triple(
            &self.node,
            RDF_TYPE,
            RdfValue::resource(type_uri),
            rule,
            source,
        );
        self.process_type(fact);
    }

    fn type_inference(&mut self, fact: Fact) {
        let Some(type_uri) = fact.object().as_resource().map(ToOwned::to_owned) else {
            return;
        };
        let class = self
            .schema
            .class(&type_uri)
            .cloned()
            .unwrap_or_else(|| OwlClass::new(&type_uri));

        if type_uri == OWL_NOTHING && self.frontier(&fact) {
            let inc = self.inconsistency(OwlRule::ClsNothing2, fact.clone());
            self.collect_inconsistency(inc);
        }

        for other in class
            .disjoint_classes
            .intersection(&self.known_types.keys().cloned().collect())
        {
            if let Some(other_fact) = self.known_types.get(other).cloned() {
                let mut inc = self.inconsistency(OwlRule::CaxDw, fact.clone());
                inc.add_source(other_fact);
                self.collect_inconsistency(inc);
            }
        }
        for other in class
            .complementary_classes
            .intersection(&self.known_types.keys().cloned().collect())
        {
            if let Some(other_fact) = self.known_types.get(other).cloned() {
                let mut inc = self.inconsistency(OwlRule::ClsCom, fact.clone());
                inc.add_source(other_fact);
                self.collect_inconsistency(inc);
            }
        }

        if !fact.has_rule(OwlRule::CaxSco) && self.frontier(&fact) {
            for supertype in class.super_classes() {
                if supertype != type_uri && supertype != OWL_THING {
                    self.process_generated_type(supertype, OwlRule::CaxSco, fact.clone());
                }
            }
        }

        for prop in &class.properties {
            for value in &class.values {
                self.collect(self.triple(
                    &self.node,
                    prop,
                    value.clone(),
                    OwlRule::ClsHv1,
                    fact.clone(),
                ));
            }
        }

        if let Some(inferences) = self.possible_inferences.get(&type_uri).cloned() {
            for mut pending in inferences {
                pending.add_source(fact.clone());
                self.collect(pending);
            }
        }
        if let Some(inconsistencies) = self.possible_inconsistencies.get(&type_uri).cloned() {
            for mut pending in inconsistencies {
                pending.add_source(fact.clone());
                self.collect_inconsistency(pending);
            }
        }
    }

    fn on_type(&mut self, type_uri: String, fact: Fact) {
        self.possible_inferences
            .entry(type_uri.clone())
            .or_default()
            .push(fact.clone());
        if let Some(type_fact) = self.known_types.get(&type_uri).cloned() {
            let mut join = fact;
            join.add_source(type_fact);
            self.collect(join);
        }
    }

    fn inconsistent_on_type(&mut self, type_uri: String, derivation: Derivation) {
        self.possible_inconsistencies
            .entry(type_uri.clone())
            .or_default()
            .push(derivation.clone());
        if let Some(type_fact) = self.known_types.get(&type_uri).cloned() {
            let mut inc = derivation;
            inc.add_source(type_fact);
            self.collect_inconsistency(inc);
        }
    }

    fn triple(
        &self,
        subject: &str,
        predicate: &str,
        object: RdfValue,
        rule: OwlRule,
        source: Fact,
    ) -> Fact {
        let mut fact = Fact::inferred(
            subject.to_string(),
            predicate.to_string(),
            object,
            self.current_iteration,
            rule,
            self.node.clone(),
        );
        fact.add_source(source);
        fact
    }

    fn inconsistency(&self, rule: OwlRule, source: Fact) -> Derivation {
        let mut derivation = Derivation::new(self.current_iteration, rule, self.node.clone());
        derivation.add_source(source);
        derivation
    }

    fn collect(&mut self, fact: Fact) -> bool {
        if fact.iteration() == self.current_iteration
            && !fact.is_cycle()
            && fact
                .derivation
                .as_ref()
                .is_some_and(|d| d.sources.iter().any(|source| self.frontier(source)))
        {
            self.new_facts.insert(fact)
        } else {
            false
        }
    }

    fn collect_inconsistency(&mut self, inconsistency: Derivation) -> bool {
        if inconsistency
            .sources
            .iter()
            .any(|source| self.frontier(source))
        {
            self.inconsistencies.insert(inconsistency)
        } else {
            false
        }
    }

    fn frontier(&self, fact: &Fact) -> bool {
        let t = fact.iteration();
        let derivation_node = fact.derivation.as_ref().and_then(|d| d.node.as_deref());
        self.min_iteration == 0
            || t == self.current_iteration
            || (t >= self.min_iteration && derivation_node != Some(self.node.as_str()))
    }

    fn relevant_to_future(&self, fact: &Fact) -> bool {
        if Self::relevant_join_rule(fact, &self.schema) != Relevance::None {
            return true;
        }
        let general = Self::relevant_fact(fact, &self.schema);
        let subject = fact.subject();
        let object = fact.object().as_resource();
        if object != Some(subject) {
            if general == Relevance::Subject && self.node == subject {
                return false;
            }
            if general == Relevance::Object && object == Some(self.node.as_str()) {
                return false;
            }
        }
        general != Relevance::None
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceWritable {
    pub resource: Option<String>,
    pub sort_key: i32,
}

impl ResourceWritable {
    pub fn new(resource: impl Into<String>) -> Self {
        Self {
            resource: Some(resource.into()),
            sort_key: 0,
        }
    }

    pub fn with_sort_key(resource: impl Into<String>, sort_key: i32) -> Self {
        Self {
            resource: Some(resource.into()),
            sort_key,
        }
    }
}

impl Ord for ResourceWritable {
    fn cmp(&self, other: &Self) -> Ordering {
        self.resource
            .cmp(&other.resource)
            .then_with(|| self.sort_key.cmp(&other.sort_key))
    }
}

impl PartialOrd for ResourceWritable {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn forward_chain_map(fact: &Fact, schema: &Schema) -> Vec<(ResourceWritable, Fact)> {
    let relevance = LocalReasoner::relevant_fact(fact, schema);
    let mut out = Vec::new();
    if relevance.subject() {
        out.push((
            ResourceWritable::with_sort_key(fact.subject(), 1),
            fact.clone(),
        ));
    }
    if relevance.object()
        && let Some(object) = fact.object().as_resource()
    {
        out.push((ResourceWritable::with_sort_key(object, -1), fact.clone()));
    }
    out
}

pub fn forward_chain_reduce(
    node: &str,
    facts: impl IntoIterator<Item = Fact>,
    schema: Schema,
    iteration: u32,
    schema_update: u32,
) -> (BTreeSet<Fact>, BTreeSet<Derivation>) {
    let mut reasoner = LocalReasoner::new(node, schema, iteration, schema_update);
    for fact in facts {
        reasoner.process_fact(fact);
    }
    reasoner.collect_types();
    (reasoner.facts(), reasoner.inconsistencies())
}

pub fn duplicate_elimination_reduce(
    fact: Fact,
    derivations: impl IntoIterator<Item = Derivation>,
    current_iteration: u32,
) -> Option<EitherFactOrInconsistency> {
    let mut best: Option<Derivation> = None;
    for derivation in derivations {
        if derivation.iteration < current_iteration {
            return None;
        }
        if best
            .as_ref()
            .is_none_or(|current_best| current_best.span() > derivation.span())
        {
            best = Some(derivation);
        }
    }
    let best = best?;
    if fact.is_empty() {
        Some(EitherFactOrInconsistency::Inconsistency(best))
    } else {
        let mut out = fact;
        out.derivation = Some(best);
        Some(EitherFactOrInconsistency::Fact(out))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EitherFactOrInconsistency {
    Fact(Fact),
    Inconsistency(Derivation),
}

pub fn schema_filter(facts: impl IntoIterator<Item = Fact>) -> Schema {
    let mut schema = Schema::default();
    for fact in facts {
        if let Some(triple) = &fact.triple {
            schema.process_triple(triple);
        }
    }
    schema.closure();
    schema
}

pub fn reasoning_module_dependencies() -> Vec<RustDependency> {
    vec![
        RustDependency {
            crate_name: "omrya",
            version_req: env!("CARGO_PKG_VERSION"),
            feature: None,
        },
        RustDependency {
            crate_name: "fjall",
            version_req: "3.1",
            feature: Some("secure-keyspaces"),
        },
        RustDependency {
            crate_name: "tree-sitter",
            version_req: "0.26",
            feature: None,
        },
        RustDependency {
            crate_name: "tree-sitter-rust",
            version_req: "0.24",
            feature: None,
        },
    ]
}

pub fn reasoning_test_dependency() -> RustDependency {
    RustDependency {
        crate_name: "omrya",
        version_req: env!("CARGO_PKG_VERSION"),
        feature: Some(REASONING_TEST_FEATURE),
    }
}

fn schema_predicates() -> BTreeSet<&'static str> {
    BTreeSet::from([
        RDFS_SUBCLASS_OF,
        RDFS_SUBPROPERTY_OF,
        RDFS_DOMAIN,
        RDFS_RANGE,
        OWL_EQUIVALENT_CLASS,
        OWL_EQUIVALENT_PROPERTY,
        OWL_INVERSE_OF,
        OWL_DISJOINT_WITH,
        OWL_COMPLEMENT_OF,
        OWL_ON_PROPERTY,
        OWL_SOME_VALUES_FROM,
        OWL_ALL_VALUES_FROM,
        OWL_HAS_VALUE,
        OWL_MAX_CARDINALITY,
        OWL_MAX_QUALIFIED_CARDINALITY,
        OWL_PROPERTY_DISJOINT_WITH,
        OWL_ON_CLASS,
    ])
}

fn schema_types() -> BTreeSet<&'static str> {
    BTreeSet::from([
        OWL_TRANSITIVE_PROPERTY,
        OWL_IRREFLEXIVE_PROPERTY,
        OWL_SYMMETRIC_PROPERTY,
        OWL_ASYMMETRIC_PROPERTY,
        OWL_FUNCTIONAL_PROPERTY,
        OWL_INVERSE_FUNCTIONAL_PROPERTY,
    ])
}

#[cfg(test)]
#[path = "../tests/reasoning_tests.rs"]
mod tests;
