use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{RDF_LANG_STRING, RyaIri, RyaStatement, RyaType, XSD_ANY_URI, XSD_STRING};
use crate::resolver::{DELIM_BYTE, TYPE_DELIM_BYTE, deserialize, serialize_type};

pub const STATEMENT_SEPARATOR: char = '\0';
pub const GEO_WKT_LITERAL: &str = "http://www.opengis.net/ont/geosparql#wktLiteral";
pub const USE_GEO: &str = "sc.use_geo";
pub const USE_FREETEXT: &str = "sc.use_freetext";
pub const USE_TEMPORAL: &str = "sc.use_temporal";
pub const USE_ENTITY: &str = "sc.use_entity";
pub const GEO_PREDICATES_LIST: &str = "sc.geo.predicates";
pub const FREETEXT_PREDICATES_LIST: &str = "sc.freetext.predicates";
pub const TEMPORAL_PREDICATES_LIST: &str = "sc.temporal.predicates";
pub const RDF_CLOUD_TRIPLE_STORE_CONF_OPTIMIZERS: &str = "query.optimizers";
pub const FILTER_FUNCTION_OPTIMIZER_CLASS: &str = "mvm.rya.indexing.FilterFunctionOptimizer";
pub const ENTITY_CENTRIC_INDEXER_CLASS: &str = "mvm.rya.indexing.fjall.entity.EntityCentricIndex";
pub const ENTITY_OPTIMIZER_CLASS: &str = "mvm.rya.indexing.fjall.entity.EntityOptimizer";
pub const FJALL_GEOMESA_INDEXER_CLASS: &str = "mvm.rya.indexing.fjall.geo.GeoMesaGeoIndexer";
pub const FJALL_FREETEXT_INDEXER_CLASS: &str =
    "mvm.rya.indexing.fjall.freetext.FjallFreeTextIndexer";
pub const FJALL_TEMPORAL_INDEXER_CLASS: &str =
    "mvm.rya.indexing.fjall.temporal.FjallTemporalIndexer";
pub const USE_PCJ: &str = "sc.use_pcj";
pub const USE_OPTIMAL_PCJ: &str = "sc.use.optimal.pcj";
pub const FLUO_APP_NAME: &str = "rya.indexing.pcj.fluo.fluoAppName";
pub const USE_PCJ_FLUO_UPDATER: &str = "rya.indexing.pcj.updater.fluo";
pub const PCJ_STORAGE_TYPE: &str = "rya.indexing.pcj.storageType";
pub const PCJ_UPDATER_TYPE: &str = "rya.indexing.pcj.updaterType";
pub const PCJ_OPTIMIZER_CLASS: &str = "mvm.rya.indexing.pcj.matching.PCJOptimizer";
pub const LEGACY_PRECOMP_JOIN_OPTIMIZER_CLASS: &str =
    "mvm.rya.indexing.external.PrecompJoinOptimizer";
pub const PRECOMPUTED_JOIN_INDEXER_CLASS: &str = "mvm.rya.indexing.external.PrecomputedJoinIndexer";

pub fn simple_tokenize(input: &str) -> BTreeSet<String> {
    input
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '*' || ch == '_'))
        .map(|token| token.to_ascii_lowercase())
        .filter(|token| !token.is_empty())
        .collect()
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IndexingConfiguration {
    values: BTreeMap<String, String>,
}

impl IndexingConfiguration {
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    pub fn set_bool(&mut self, key: &str, value: bool) {
        self.set(key, value.to_string());
    }

    pub fn set_csv(&mut self, key: &str, values: impl IntoIterator<Item = impl AsRef<str>>) {
        self.set(
            key,
            values
                .into_iter()
                .map(|value| value.as_ref().to_string())
                .collect::<Vec<_>>()
                .join(","),
        );
    }

    pub fn get_bool(&self, key: &str) -> bool {
        self.values
            .get(key)
            .is_some_and(|value| value.eq_ignore_ascii_case("true"))
    }

    pub fn csv_iris(&self, key: &str) -> BTreeSet<RyaIri> {
        self.values
            .get(key)
            .into_iter()
            .flat_map(|raw| raw.split(','))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .filter_map(|value| RyaIri::new(value).ok())
            .collect()
    }

    pub fn fluo_app_name(&self) -> Option<&str> {
        self.values.get(FLUO_APP_NAME).map(String::as_str)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdditionalIndexers {
    pub classes: Vec<&'static str>,
    pub use_filter_index: bool,
}

pub fn configured_additional_indexers(conf: &IndexingConfiguration) -> AdditionalIndexers {
    let mut classes = Vec::new();
    let mut use_filter_index = false;
    if conf.get_bool(USE_GEO) {
        classes.push(FJALL_GEOMESA_INDEXER_CLASS);
        use_filter_index = true;
    }
    if conf.get_bool(USE_FREETEXT) {
        classes.push(FJALL_FREETEXT_INDEXER_CLASS);
        use_filter_index = true;
    }
    if conf.get_bool(USE_TEMPORAL) {
        classes.push(FJALL_TEMPORAL_INDEXER_CLASS);
        use_filter_index = true;
    }
    AdditionalIndexers {
        classes,
        use_filter_index,
    }
}

pub fn configured_pcj_optimizer(conf: &IndexingConfiguration) -> Option<&'static str> {
    (conf.get_bool(USE_PCJ) || conf.get_bool(USE_OPTIMAL_PCJ)).then_some(PCJ_OPTIMIZER_CLASS)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IndexerConfigurationPlan {
    pub additional_indexers_key: &'static str,
    pub optimizer_key: &'static str,
    pub additional_indexers: Vec<&'static str>,
    pub optimizers: Vec<&'static str>,
    pub pcj_optimizer: Option<&'static str>,
}

pub fn configured_indexer_plan(conf: &IndexingConfiguration) -> IndexerConfigurationPlan {
    let mut additional = configured_additional_indexers(conf);
    let mut optimizers = Vec::new();

    if additional.use_filter_index {
        optimizers.push(FILTER_FUNCTION_OPTIMIZER_CLASS);
    }

    if conf.get_bool(USE_ENTITY) {
        additional.classes.push(ENTITY_CENTRIC_INDEXER_CLASS);
        optimizers.push(ENTITY_OPTIMIZER_CLASS);
    }

    let pcj_optimizer = configured_pcj_optimizer(conf);
    if pcj_optimizer.is_some() {
        additional.classes.push(PRECOMPUTED_JOIN_INDEXER_CLASS);
    }

    IndexerConfigurationPlan {
        additional_indexers_key: crate::fjall::CONF_ADDITIONAL_INDEXERS,
        optimizer_key: RDF_CLOUD_TRIPLE_STORE_CONF_OPTIMIZERS,
        additional_indexers: additional.classes,
        optimizers,
        pcj_optimizer,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilterFunctionOptimizerFixture {
    pub initialized: bool,
    pub geo_indexer_class: Option<&'static str>,
    pub free_text_indexer_class: Option<&'static str>,
    pub temporal_indexer_class: Option<&'static str>,
}

impl FilterFunctionOptimizerFixture {
    pub fn from_config(conf: &IndexingConfiguration) -> Self {
        let _ = conf;
        Self {
            initialized: true,
            geo_indexer_class: Some(FJALL_GEOMESA_INDEXER_CLASS),
            free_text_indexer_class: Some(FJALL_FREETEXT_INDEXER_CLASS),
            temporal_indexer_class: Some(FJALL_TEMPORAL_INDEXER_CLASS),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StatementConstraints {
    pub subject: Option<RyaIri>,
    pub context: Option<RyaIri>,
    pub predicates: BTreeSet<RyaIri>,
}

impl StatementConstraints {
    pub fn with_subject(mut self, subject: RyaIri) -> Self {
        self.subject = Some(subject);
        self
    }

    pub fn with_context(mut self, context: RyaIri) -> Self {
        self.context = Some(context);
        self
    }

    pub fn with_predicates(mut self, predicates: impl IntoIterator<Item = RyaIri>) -> Self {
        self.predicates = predicates.into_iter().collect();
        self
    }

    pub fn allows(&self, statement: &RyaStatement) -> bool {
        self.subject
            .as_ref()
            .is_none_or(|subject| subject == &statement.subject)
            && self.context.as_ref().is_none_or(|context| {
                statement
                    .context
                    .as_ref()
                    .is_some_and(|stmt_context| stmt_context == context)
            })
            && (self.predicates.is_empty() || self.predicates.contains(&statement.predicate))
    }
}

pub fn write_statement(statement: &RyaStatement) -> String {
    [
        statement
            .context
            .as_ref()
            .map(RyaIri::data)
            .unwrap_or_default(),
        statement.subject.data(),
        statement.predicate.data(),
        &write_object(&statement.object),
    ]
    .join(&STATEMENT_SEPARATOR.to_string())
}

pub fn read_statement(input: &str) -> Result<RyaStatement, String> {
    let parts = input.split(STATEMENT_SEPARATOR).collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err(format!("Not a valid statement: {input}"));
    }
    read_statement_parts(parts[1], parts[2], parts[3], parts[0])
}

pub fn read_statement_parts(
    subject: &str,
    predicate: &str,
    object: &str,
    context: &str,
) -> Result<RyaStatement, String> {
    let mut statement = RyaStatement::new(
        RyaIri::new(subject)?,
        RyaIri::new(predicate)?,
        parse_object(object)?,
    );
    if !context.is_empty() {
        statement.context = Some(RyaIri::new(context)?);
    }
    Ok(statement)
}

pub fn create_statement_regex(
    context: Option<&str>,
    subject: Option<&str>,
    predicates: &[&str],
) -> Option<String> {
    if context.is_none() && subject.is_none() && predicates.is_empty() {
        return None;
    }
    let any = format!("[^{STATEMENT_SEPARATOR}]*");
    let context = context.map(str::to_string).unwrap_or_else(|| any.clone());
    let subject = subject.map(str::to_string).unwrap_or_else(|| any.clone());
    let predicate = if predicates.is_empty() {
        any.clone()
    } else {
        format!("({})", predicates.join("|"))
    };
    Some(format!(
        "^{context}{STATEMENT_SEPARATOR}{subject}{STATEMENT_SEPARATOR}{predicate}{STATEMENT_SEPARATOR}.*"
    ))
}

pub fn get_well_known_text(statement: &RyaStatement) -> Result<&str, String> {
    if statement.object.data_type() != Some(GEO_WKT_LITERAL) {
        return Err(format!(
            "Literal is not of type {GEO_WKT_LITERAL}: {}",
            write_statement(statement)
        ));
    }
    Ok(statement.object.data())
}

pub trait InMemorySecondaryIndexer {
    fn store_statement(&mut self, statement: &RyaStatement);
    fn delete_statement(&mut self, statement: &RyaStatement);
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryFreeTextIndexer {
    valid_predicates: BTreeSet<RyaIri>,
    documents: BTreeMap<String, RyaStatement>,
    term_to_docs: BTreeMap<String, BTreeSet<String>>,
    term_dictionary: BTreeSet<String>,
}

impl InMemoryFreeTextIndexer {
    pub fn new(valid_predicates: impl IntoIterator<Item = RyaIri>) -> Self {
        Self {
            valid_predicates: valid_predicates.into_iter().collect(),
            ..Self::default()
        }
    }

    pub fn query_text(&self, query: &str, constraints: &StatementConstraints) -> Vec<RyaStatement> {
        let mut required = Vec::new();
        let mut excluded = Vec::new();
        for raw in query.split_whitespace() {
            let negated = raw.starts_with('!');
            for token in simple_tokenize(raw.trim_start_matches('!')) {
                if negated {
                    excluded.push(token);
                } else {
                    required.push(token);
                }
            }
        }

        let mut matching_doc_ids = if let Some(first) = required.first() {
            self.doc_ids_for_query_token(first)
        } else {
            self.documents.keys().cloned().collect()
        };

        for token in &required[1..] {
            let token_doc_ids = self.doc_ids_for_query_token(token);
            matching_doc_ids = matching_doc_ids
                .intersection(&token_doc_ids)
                .cloned()
                .collect();
        }
        for token in excluded {
            let token_doc_ids = self.doc_ids_for_query_token(&token);
            matching_doc_ids = matching_doc_ids
                .difference(&token_doc_ids)
                .cloned()
                .collect();
        }

        matching_doc_ids
            .into_iter()
            .filter_map(|doc_id| self.documents.get(&doc_id))
            .filter(|statement| constraints.allows(statement))
            .cloned()
            .collect()
    }

    pub fn term_dictionary(&self) -> &BTreeSet<String> {
        &self.term_dictionary
    }

    fn doc_ids_for_query_token(&self, token: &str) -> BTreeSet<String> {
        self.matching_terms(token)
            .into_iter()
            .flat_map(|term| self.term_to_docs.get(&term).cloned().unwrap_or_default())
            .collect()
    }

    fn matching_terms(&self, token: &str) -> Vec<String> {
        let token = token.to_ascii_lowercase();
        match (token.strip_prefix('*'), token.strip_suffix('*')) {
            (Some(suffix), _) if !suffix.is_empty() => self
                .term_dictionary
                .iter()
                .filter(|term| term.ends_with(suffix))
                .cloned()
                .collect(),
            (_, Some(prefix)) if !prefix.is_empty() => self
                .term_dictionary
                .iter()
                .filter(|term| term.starts_with(prefix))
                .cloned()
                .collect(),
            _ => vec![token],
        }
    }

    fn is_indexable(&self, statement: &RyaStatement) -> bool {
        (self.valid_predicates.is_empty() || self.valid_predicates.contains(&statement.predicate))
            && statement.object.data_type() != Some(XSD_ANY_URI)
    }
}

impl InMemorySecondaryIndexer for InMemoryFreeTextIndexer {
    fn store_statement(&mut self, statement: &RyaStatement) {
        if !self.is_indexable(statement) {
            return;
        }
        let tokens = simple_tokenize(statement.object.data());
        if tokens.is_empty() {
            return;
        }
        let doc_id = write_statement(statement);
        self.documents.insert(doc_id.clone(), statement.clone());
        for token in tokens {
            self.term_to_docs
                .entry(token.clone())
                .or_default()
                .insert(doc_id.clone());
            self.term_dictionary.insert(token);
        }
    }

    fn delete_statement(&mut self, statement: &RyaStatement) {
        if !self.is_indexable(statement) {
            return;
        }
        let doc_id = write_statement(statement);
        let tokens = simple_tokenize(statement.object.data());
        self.documents.remove(&doc_id);
        for token in tokens {
            let mut should_remove_term = false;
            if let Some(doc_ids) = self.term_to_docs.get_mut(&token) {
                doc_ids.remove(&doc_id);
                should_remove_term = doc_ids.is_empty();
            }
            if should_remove_term {
                self.term_to_docs.remove(&token);
                self.term_dictionary.remove(&token);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoPoint {
    pub x: f64,
    pub y: f64,
}

impl GeoPoint {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeoPolygon {
    pub exterior: Vec<GeoPoint>,
    pub holes: Vec<Vec<GeoPoint>>,
}

impl GeoPolygon {
    pub fn new(exterior: Vec<GeoPoint>, holes: Vec<Vec<GeoPoint>>) -> Self {
        Self { exterior, holes }
    }

    fn contains_point(&self, point: GeoPoint) -> bool {
        point_in_ring(point, &self.exterior)
            && !self.holes.iter().any(|hole| point_in_ring(point, hole))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum GeoShape {
    Point(GeoPoint),
    LineString(Vec<GeoPoint>),
    Polygon(GeoPolygon),
}

impl GeoShape {
    pub(crate) fn coordinates(&self) -> Vec<GeoPoint> {
        match self {
            Self::Point(point) => vec![*point],
            Self::LineString(points) => points.clone(),
            Self::Polygon(polygon) => polygon.exterior.clone(),
        }
    }
}

pub const GEOMESA_SAFE_WORLD_MAX_LONGITUDE: f64 = 179.0;

pub fn geomesa_safe_world_polygon() -> Vec<GeoPoint> {
    vec![
        GeoPoint::new(-180.0, 90.0),
        GeoPoint::new(GEOMESA_SAFE_WORLD_MAX_LONGITUDE, 90.0),
        GeoPoint::new(GEOMESA_SAFE_WORLD_MAX_LONGITUDE, -90.0),
        GeoPoint::new(-180.0, -90.0),
        GeoPoint::new(-180.0, 90.0),
    ]
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryGeoIndexer {
    valid_predicates: BTreeSet<RyaIri>,
    features: BTreeMap<String, (RyaStatement, GeoShape)>,
}

impl InMemoryGeoIndexer {
    pub fn new(valid_predicates: impl IntoIterator<Item = RyaIri>) -> Self {
        Self {
            valid_predicates: valid_predicates.into_iter().collect(),
            ..Self::default()
        }
    }

    pub fn query_within(
        &self,
        polygon: &[GeoPoint],
        constraints: &StatementConstraints,
    ) -> Vec<RyaStatement> {
        self.query_within_polygon(&GeoPolygon::new(polygon.to_vec(), Vec::new()), constraints)
    }

    pub fn query_within_polygon(
        &self,
        polygon: &GeoPolygon,
        constraints: &StatementConstraints,
    ) -> Vec<RyaStatement> {
        self.features
            .values()
            .filter(|(statement, shape)| {
                constraints.allows(statement)
                    && shape
                        .coordinates()
                        .into_iter()
                        .all(|point| polygon.contains_point(point))
            })
            .map(|(statement, _)| statement.clone())
            .collect()
    }

    pub fn query_equals(
        &self,
        shape: &GeoShape,
        constraints: &StatementConstraints,
    ) -> Vec<RyaStatement> {
        self.features
            .values()
            .filter(|(statement, candidate)| constraints.allows(statement) && candidate == shape)
            .map(|(statement, _)| statement.clone())
            .collect()
    }

    fn is_indexable(&self, statement: &RyaStatement) -> bool {
        (self.valid_predicates.is_empty() || self.valid_predicates.contains(&statement.predicate))
            && statement.object.data_type() != Some(XSD_ANY_URI)
    }
}

impl InMemorySecondaryIndexer for InMemoryGeoIndexer {
    fn store_statement(&mut self, statement: &RyaStatement) {
        if !self.is_indexable(statement) {
            return;
        }
        let Ok(shape) = parse_wkt_geometry(statement.object.data()) else {
            return;
        };
        self.features
            .insert(write_statement(statement), (statement.clone(), shape));
    }

    fn delete_statement(&mut self, statement: &RyaStatement) {
        if self.is_indexable(statement) {
            self.features.remove(&write_statement(statement));
        }
    }
}

pub fn parse_wkt_geometry(input: &str) -> Result<GeoShape, String> {
    let trimmed = input.trim();
    if let Some(inner) = wkt_inner(trimmed, "POINT") {
        return Ok(GeoShape::Point(parse_coordinate_pair(inner)?));
    }
    if let Some(inner) = wkt_inner(trimmed, "LINESTRING") {
        return Ok(GeoShape::LineString(parse_coordinate_list(inner)?));
    }
    if let Some(inner) = wkt_inner(trimmed, "POLYGON") {
        let rings = parse_polygon_rings(inner)?;
        let Some((exterior, holes)) = rings.split_first() else {
            return Err(format!("Unsupported WKT polygon: {input}"));
        };
        return Ok(GeoShape::Polygon(GeoPolygon::new(
            exterior.clone(),
            holes.to_vec(),
        )));
    }
    Err(format!("Unsupported WKT geometry: {input}"))
}

fn wkt_inner<'a>(input: &'a str, kind: &str) -> Option<&'a str> {
    let trimmed = input.trim();
    let prefix_len = kind.len();
    if trimmed.len() < prefix_len || !trimmed[..prefix_len].eq_ignore_ascii_case(kind) {
        return None;
    }
    let rest = trimmed[prefix_len..].trim_start();
    rest.strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
        .map(str::trim)
}

fn parse_polygon_rings(input: &str) -> Result<Vec<Vec<GeoPoint>>, String> {
    let mut rings = Vec::new();
    let mut depth = 0_i32;
    let mut ring_start = None;
    for (index, ch) in input.char_indices() {
        match ch {
            '(' => {
                if depth == 0 {
                    ring_start = Some(index + ch.len_utf8());
                }
                depth += 1;
            }
            ')' => {
                depth -= 1;
                if depth < 0 {
                    return Err(format!("Unsupported WKT polygon rings: {input}"));
                }
                if depth == 0 {
                    let start = ring_start
                        .ok_or_else(|| format!("Unsupported WKT polygon rings: {input}"))?;
                    rings.push(parse_coordinate_list(&input[start..index])?);
                    ring_start = None;
                }
            }
            _ => {}
        }
    }
    if depth != 0 {
        return Err(format!("Unsupported WKT polygon rings: {input}"));
    }
    if rings.is_empty() {
        return Ok(vec![parse_coordinate_list(input)?]);
    }
    Ok(rings)
}

fn parse_coordinate_list(input: &str) -> Result<Vec<GeoPoint>, String> {
    input
        .split(',')
        .map(parse_coordinate_pair)
        .collect::<Result<Vec<_>, _>>()
}

fn parse_coordinate_pair(input: &str) -> Result<GeoPoint, String> {
    let coords = input.split_whitespace().collect::<Vec<_>>();
    if coords.len() != 2 {
        return Err(format!("Unsupported WKT coordinate pair: {input}"));
    }
    Ok(GeoPoint::new(
        coords[0].parse::<f64>().map_err(|e| e.to_string())?,
        coords[1].parse::<f64>().map_err(|e| e.to_string())?,
    ))
}

fn point_in_ring(point: GeoPoint, polygon: &[GeoPoint]) -> bool {
    if polygon.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut previous = *polygon.last().unwrap();
    for current in polygon {
        let crosses = (current.y > point.y) != (previous.y > point.y);
        if crosses {
            let x_intersection = (previous.x - current.x) * (point.y - current.y)
                / (previous.y - current.y)
                + current.x;
            if point.x < x_intersection {
                inside = !inside;
            }
        }
        previous = *current;
    }
    inside
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct TemporalIndexEntry {
    pub row: String,
    pub column_family: String,
    pub column_qualifier: String,
    pub value: String,
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryTemporalIndexer {
    valid_predicates: BTreeSet<RyaIri>,
    entries: BTreeSet<TemporalIndexEntry>,
}

impl InMemoryTemporalIndexer {
    pub fn new(valid_predicates: impl IntoIterator<Item = RyaIri>) -> Self {
        Self {
            valid_predicates: valid_predicates.into_iter().collect(),
            ..Self::default()
        }
    }

    pub fn entries(&self) -> &BTreeSet<TemporalIndexEntry> {
        &self.entries
    }

    pub fn row_count(&self) -> usize {
        self.entries.len()
    }

    fn is_indexable(&self, statement: &RyaStatement) -> bool {
        (self.valid_predicates.is_empty() || self.valid_predicates.contains(&statement.predicate))
            && statement.object.data_type() != Some(XSD_ANY_URI)
    }
}

impl InMemorySecondaryIndexer for InMemoryTemporalIndexer {
    fn store_statement(&mut self, statement: &RyaStatement) {
        if !self.is_indexable(statement) {
            return;
        }
        for entry in temporal_entries(statement) {
            self.entries.insert(entry);
        }
    }

    fn delete_statement(&mut self, statement: &RyaStatement) {
        if !self.is_indexable(statement) {
            return;
        }
        for entry in temporal_entries(statement) {
            self.entries.remove(&entry);
        }
    }
}

fn temporal_entries(statement: &RyaStatement) -> Vec<TemporalIndexEntry> {
    if let Ok(interval) = TemporalInstantRfc3339::parse_interval(statement.object.data()) {
        return vec![
            TemporalIndexEntry {
                row: temporal_unique_row(statement, &interval.as_key_beginning()),
                column_family: statement_context(statement),
                column_qualifier: "begin".to_string(),
                value: write_statement(statement),
            },
            TemporalIndexEntry {
                row: temporal_unique_row(statement, &interval.as_key_end()),
                column_family: statement_context(statement),
                column_qualifier: "end".to_string(),
                value: write_statement(statement),
            },
        ];
    }
    let Ok(instant) = TemporalInstantRfc3339::parse(statement.object.data()) else {
        return Vec::new();
    };
    let time = instant.as_key_string();
    [
        format!("o|{time}"),
        format!("s|{}|{time}", statement.subject.data()),
        format!("p|{}|{time}", statement.predicate.data()),
        format!(
            "sp|{}|{}|{time}",
            statement.subject.data(),
            statement.predicate.data()
        ),
    ]
    .into_iter()
    .map(|row| TemporalIndexEntry {
        row: temporal_unique_row(statement, &row),
        column_family: statement_context(statement),
        column_qualifier: "instant".to_string(),
        value: write_statement(statement),
    })
    .collect()
}

fn temporal_unique_row(statement: &RyaStatement, prefix: &str) -> String {
    format!("{prefix}|{}", write_statement(statement))
}

fn statement_context(statement: &RyaStatement) -> String {
    statement
        .context
        .as_ref()
        .map(|context| context.data().to_string())
        .unwrap_or_default()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EntityIndexMutation {
    pub row: Vec<u8>,
    pub column_family: Vec<u8>,
    pub column_qualifier: Vec<u8>,
    pub column_visibility: Vec<u8>,
    pub value: Vec<u8>,
    pub timestamp: u64,
    pub delete: bool,
}

pub struct EntityCentricIndex;

impl EntityCentricIndex {
    pub fn create_mutations(statement: &RyaStatement) -> Result<Vec<EntityIndexMutation>, String> {
        let subject = statement.subject.data().as_bytes();
        let predicate = statement.predicate.data().as_bytes();
        let (object_data, object_type) = serialize_type(&statement.object)?;
        let context = statement
            .context
            .as_ref()
            .map(|context| context.data().as_bytes().to_vec())
            .unwrap_or_default();
        let column_visibility = statement.column_visibility.clone().unwrap_or_default();
        let value = statement.value.clone().unwrap_or_default();

        Ok(vec![
            EntityIndexMutation {
                row: subject.to_vec(),
                column_family: predicate.to_vec(),
                column_qualifier: concat_entity_qualifier(
                    &context,
                    b"object",
                    &[&object_data, &object_type],
                ),
                column_visibility: column_visibility.clone(),
                value: value.clone(),
                timestamp: statement.timestamp,
                delete: false,
            },
            EntityIndexMutation {
                row: object_data,
                column_family: predicate.to_vec(),
                column_qualifier: concat_entity_qualifier(
                    &context,
                    b"subject",
                    &[subject, &object_type],
                ),
                column_visibility,
                value,
                timestamp: statement.timestamp,
                delete: false,
            },
        ])
    }

    pub fn create_delete_mutation(statement: &RyaStatement) -> EntityIndexMutation {
        EntityIndexMutation {
            row: statement.subject.data().as_bytes().to_vec(),
            column_family: statement.predicate.data().as_bytes().to_vec(),
            column_qualifier: statement.object.data().as_bytes().to_vec(),
            column_visibility: statement.column_visibility.clone().unwrap_or_default(),
            value: Vec::new(),
            timestamp: statement.timestamp,
            delete: true,
        }
    }

    pub fn deserialize_statement(mutation: &EntityIndexMutation) -> Result<RyaStatement, String> {
        let data = &mutation.column_qualifier;
        let split = data
            .iter()
            .position(|byte| *byte == DELIM_BYTE)
            .ok_or_else(|| "Entity-centric row is missing context delimiter".to_string())?;
        let context_bytes = &data[..split];
        let edge_bytes = &data[split + 1..];
        let split = edge_bytes
            .iter()
            .position(|byte| *byte == DELIM_BYTE)
            .ok_or_else(|| "Entity-centric row is missing edge delimiter".to_string())?;
        let other_node_var =
            std::str::from_utf8(&edge_bytes[..split]).map_err(|error| error.to_string())?;
        let other_node_and_type = &edge_bytes[split + 1..];

        let predicate = bytes_to_iri(&mutation.column_family)?;
        let (subject, object_bytes) = match other_node_var {
            "subject" => {
                let type_index = other_node_and_type
                    .iter()
                    .position(|byte| *byte == TYPE_DELIM_BYTE)
                    .ok_or_else(|| {
                        "Entity-centric object row is missing object datatype marker".to_string()
                    })?;
                let subject = bytes_to_iri(&other_node_and_type[..type_index])?;
                let mut object_bytes = mutation.row.clone();
                object_bytes.extend_from_slice(&other_node_and_type[type_index..]);
                (subject, object_bytes)
            }
            "object" => {
                let subject = bytes_to_iri(&mutation.row)?;
                (subject, other_node_and_type.to_vec())
            }
            other => {
                return Err(format!(
                    "Failed to deserialize entity-centric index row. Expected 'subject' or 'object', encountered: '{other}'"
                ));
            }
        };
        let object = deserialize(&object_bytes)?;
        let mut statement =
            RyaStatement::new(subject, predicate, object).with_timestamp(mutation.timestamp);
        statement.context = if context_bytes.is_empty() {
            None
        } else {
            Some(bytes_to_iri(context_bytes)?)
        };
        statement.column_visibility = Some(mutation.column_visibility.clone());
        statement.value = Some(mutation.value.clone());
        Ok(statement)
    }
}

fn concat_entity_qualifier(context: &[u8], other_node_var: &[u8], node_parts: &[&[u8]]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(context);
    bytes.push(DELIM_BYTE);
    bytes.extend_from_slice(other_node_var);
    bytes.push(DELIM_BYTE);
    for part in node_parts {
        bytes.extend_from_slice(part);
    }
    bytes
}

fn bytes_to_iri(bytes: &[u8]) -> Result<RyaIri, String> {
    let value = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    RyaIri::new(value)
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TemporalInstantRfc3339 {
    key: String,
    readable: String,
}

impl TemporalInstantRfc3339 {
    pub fn new_utc(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Self {
        let key = format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z");
        Self {
            readable: key.clone(),
            key,
        }
    }

    pub fn parse(input: &str) -> Result<Self, String> {
        let parsed = parse_rfc3339_seconds(input)?;
        let key = format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            parsed.year, parsed.month, parsed.day, parsed.hour, parsed.minute, parsed.second
        );
        Ok(Self {
            key,
            readable: input.to_string(),
        })
    }

    pub fn as_key_string(&self) -> &str {
        &self.key
    }

    pub fn as_key_bytes(&self) -> &[u8] {
        self.key.as_bytes()
    }

    pub fn as_readable(&self) -> &str {
        &self.readable
    }

    pub fn parse_interval(input: &str) -> Result<TemporalInterval, String> {
        let Some(inner) = input.strip_prefix('[').and_then(|s| s.split_once(']')) else {
            return Err(format!(
                "Can't parse interval, expecting '[ISO8601dateTime1,ISO8601dateTime2]', actual: {input}"
            ));
        };
        let Some((start, end)) = inner.0.split_once(',') else {
            return Err(format!(
                "Can't parse interval, expecting '[ISO8601dateTime1,ISO8601dateTime2]', actual: {input}"
            ));
        };
        TemporalInterval::new(Self::parse(start)?, Self::parse(end)?)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct TemporalInterval {
    beginning: TemporalInstantRfc3339,
    end: TemporalInstantRfc3339,
}

impl TemporalInterval {
    pub const DELIMITER: &'static str = "/";

    pub fn new(
        beginning: TemporalInstantRfc3339,
        end: TemporalInstantRfc3339,
    ) -> Result<Self, String> {
        if beginning > end {
            return Err(
                "The Beginning instance must not compare greater than the end.".to_string(),
            );
        }
        Ok(Self { beginning, end })
    }

    pub fn beginning(&self) -> &TemporalInstantRfc3339 {
        &self.beginning
    }

    pub fn end(&self) -> &TemporalInstantRfc3339 {
        &self.end
    }

    pub fn as_key_beginning(&self) -> String {
        format!(
            "{}{}{}",
            self.beginning.as_key_string(),
            Self::DELIMITER,
            self.end.as_key_string()
        )
    }

    pub fn as_key_end(&self) -> String {
        format!(
            "{}{}{}",
            self.end.as_key_string(),
            Self::DELIMITER,
            self.beginning.as_key_string()
        )
    }

    pub fn as_pair(&self) -> String {
        format!(
            "[{},{}]",
            self.beginning.as_readable(),
            self.end.as_readable()
        )
    }
}

fn write_object(object: &RyaType) -> String {
    match object.data_type() {
        Some(XSD_ANY_URI) => object.data().to_string(),
        Some(XSD_STRING) => format!("\"{}\"", object.data()),
        Some(RDF_LANG_STRING) => format!(
            "\"{}\"@{}",
            object.data(),
            object.language().unwrap_or("und")
        ),
        Some(data_type) => format!("\"{}\"^^<{}>", object.data(), data_type),
        None => format!("\"{}\"", object.data()),
    }
}

fn parse_object(object: &str) -> Result<RyaType, String> {
    if !object.starts_with('"') {
        return Ok(RyaIri::new(object)?.into_type());
    }
    let end_quote = object
        .rfind('"')
        .ok_or_else(|| format!("Invalid literal object: {object}"))?;
    if end_quote == 0 {
        return Err(format!("Invalid literal object: {object}"));
    }
    let label = &object[1..end_quote];
    let suffix = &object[end_quote + 1..];
    if suffix.is_empty() {
        return Ok(RyaType::new(label));
    }
    if let Some(language) = suffix.strip_prefix('@') {
        return Ok(RyaType::with_data_type_and_language(
            RDF_LANG_STRING,
            label,
            Some(language.to_string()),
        ));
    }
    if let Some(data_type) = suffix.strip_prefix("^^<").and_then(|s| s.strip_suffix('>')) {
        return Ok(RyaType::custom(data_type, label));
    }
    Err(format!("Invalid literal object: {object}"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ParsedDateTime {
    year: i32,
    month: i32,
    day: i32,
    hour: i32,
    minute: i32,
    second: i32,
}

fn parse_rfc3339_seconds(input: &str) -> Result<ParsedDateTime, String> {
    if input.len() < 20 {
        return Err(format!("Invalid RFC3339 timestamp: {input}"));
    }
    let year = parse_i32(input, 0, 4)?;
    let month = parse_i32(input, 5, 7)?;
    let day = parse_i32(input, 8, 10)?;
    let hour = parse_i32(input, 11, 13)?;
    let minute = parse_i32(input, 14, 16)?;
    let second = parse_i32(input, 17, 19)?;
    if &input[4..5] != "-"
        || &input[7..8] != "-"
        || &input[10..11] != "T"
        || &input[13..14] != ":"
        || &input[16..17] != ":"
    {
        return Err(format!("Invalid RFC3339 timestamp: {input}"));
    }
    let mut parsed = ParsedDateTime {
        year,
        month,
        day,
        hour,
        minute,
        second,
    };
    let tz = &input[19..];
    if tz == "Z" {
        return Ok(parsed);
    }
    if tz.len() == 6 && (&tz[..1] == "+" || &tz[..1] == "-") && &tz[3..4] == ":" {
        let sign = if &tz[..1] == "+" { -1 } else { 1 };
        let offset_minutes = sign * (parse_i32(tz, 1, 3)? * 60 + parse_i32(tz, 4, 6)?);
        add_minutes(&mut parsed, offset_minutes);
        return Ok(parsed);
    }
    Err(format!("Invalid RFC3339 timezone: {input}"))
}

fn parse_i32(input: &str, start: usize, end: usize) -> Result<i32, String> {
    input[start..end].parse::<i32>().map_err(|e| e.to_string())
}

fn add_minutes(dt: &mut ParsedDateTime, minutes: i32) {
    dt.minute += minutes;
    while dt.minute < 0 {
        dt.minute += 60;
        dt.hour -= 1;
    }
    while dt.minute >= 60 {
        dt.minute -= 60;
        dt.hour += 1;
    }
    while dt.hour < 0 {
        dt.hour += 24;
        dt.day -= 1;
    }
    while dt.hour >= 24 {
        dt.hour -= 24;
        dt.day += 1;
    }
    normalize_day(dt);
}

fn normalize_day(dt: &mut ParsedDateTime) {
    while dt.day < 1 {
        dt.month -= 1;
        if dt.month < 1 {
            dt.month = 12;
            dt.year -= 1;
        }
        dt.day += days_in_month(dt.year, dt.month);
    }
    loop {
        let max = days_in_month(dt.year, dt.month);
        if dt.day <= max {
            break;
        }
        dt.day -= max;
        dt.month += 1;
        if dt.month > 12 {
            dt.month = 1;
            dt.year += 1;
        }
    }
}

fn days_in_month(year: i32, month: i32) -> i32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 30,
    }
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
#[path = "tests/indexing_tests.rs"]
mod tests;
