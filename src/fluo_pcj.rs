use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{RyaIri, RyaStatement, RyaType, XSD_ANY_URI, XSD_STRING};
use crate::pcj::{
    BindingSet, InMemoryPcjTables, PcjMetadata, VariableOrder, VisibilityBindingSet,
    binding_set_from_string, binding_set_to_string, make_pcj_table_name,
    visibility_binding_set_from_string, visibility_binding_set_to_string,
};

pub const DELIM: &str = ":::";
pub const VAR_DELIM: &str = ";";
pub const NODEID_BS_DELIM: &str = "<<:>>";
pub const JOIN_DELIM: &str = "<:>J<:>";
pub const TYPE_DELIM: &str = "<<~>>";
pub const SP_PREFIX: &str = "STATEMENT_PATTERN";
pub const JOIN_PREFIX: &str = "JOIN";
pub const FILTER_PREFIX: &str = "FILTER";
pub const QUERY_PREFIX: &str = "QUERY";
pub const URI_TYPE: &str = XSD_ANY_URI;
pub const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
pub const XSD_INT: &str = "http://www.w3.org/2001/XMLSchema#int";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PcjFluoModule {
    pub module: &'static str,
    pub crate_name: &'static str,
    pub crate_type: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PcjFluoIntegrationDependency {
    pub crate_name: &'static str,
    pub scope: Option<&'static str>,
}

pub const PCJ_FLUO_VERSION: &str = "1.0.0-beta-2";
pub const PCJ_FLUO_MODULES: &[PcjFluoModule] = &[
    PcjFluoModule {
        module: "pcj_fluo_api",
        crate_name: "omrya-pcj-fluo-api",
        crate_type: "lib",
    },
    PcjFluoModule {
        module: "pcj_fluo_app",
        crate_name: "omrya-pcj-fluo-app",
        crate_type: "bin",
    },
    PcjFluoModule {
        module: "pcj_fluo_client",
        crate_name: "omrya-pcj-fluo-client",
        crate_type: "bin",
    },
    PcjFluoModule {
        module: "pcj_fluo_integration",
        crate_name: "omrya-pcj-fluo-integration",
        crate_type: "integration-test",
    },
    PcjFluoModule {
        module: "pcj_fluo_demo",
        crate_name: "omrya-pcj-fluo-demo",
        crate_type: "example",
    },
];
pub const PCJ_FLUO_INTEGRATION_DEPENDENCIES: &[PcjFluoIntegrationDependency] = &[
    PcjFluoIntegrationDependency {
        crate_name: "omrya-pcj-fluo-api",
        scope: None,
    },
    PcjFluoIntegrationDependency {
        crate_name: "omrya-pcj-fluo-client",
        scope: None,
    },
    PcjFluoIntegrationDependency {
        crate_name: "omrya-indexing",
        scope: None,
    },
    PcjFluoIntegrationDependency {
        crate_name: "fluo-mini",
        scope: Some("test"),
    },
    PcjFluoIntegrationDependency {
        crate_name: "fluo-api",
        scope: None,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FluoColumn {
    pub family: &'static str,
    pub qualifier: &'static str,
}

impl FluoColumn {
    pub const fn new(family: &'static str, qualifier: &'static str) -> Self {
        Self { family, qualifier }
    }
}

pub mod columns {
    use super::FluoColumn;

    pub const TRIPLES: FluoColumn = FluoColumn::new("triples", "SPO");
    pub const QUERY_RYA_EXPORT_TABLE_NAME: FluoColumn =
        FluoColumn::new("query", "ryaExportTableName");
    pub const QUERY_ID: FluoColumn = FluoColumn::new("sparql", "queryId");

    pub const QUERY_NODE_ID: FluoColumn = FluoColumn::new("queryMetadata", "nodeId");
    pub const QUERY_VARIABLE_ORDER: FluoColumn = FluoColumn::new("queryMetadata", "variableOrder");
    pub const QUERY_SPARQL: FluoColumn = FluoColumn::new("queryMetadata", "sparql");
    pub const QUERY_CHILD_NODE_ID: FluoColumn = FluoColumn::new("queryMetadata", "childNodeId");
    pub const QUERY_BINDING_SET: FluoColumn = FluoColumn::new("queryMetadata", "bindingSet");

    pub const FILTER_NODE_ID: FluoColumn = FluoColumn::new("filterMetadata", "nodeId");
    pub const FILTER_VARIABLE_ORDER: FluoColumn =
        FluoColumn::new("filterMetadata", "veriableOrder");
    pub const FILTER_ORIGINAL_SPARQL: FluoColumn =
        FluoColumn::new("filterMetadata", "originalSparql");
    pub const FILTER_INDEX_WITHIN_SPARQL: FluoColumn =
        FluoColumn::new("filterMetadata", "filterIndexWithinSparql");
    pub const FILTER_PARENT_NODE_ID: FluoColumn = FluoColumn::new("filterMetadata", "parentNodeId");
    pub const FILTER_CHILD_NODE_ID: FluoColumn = FluoColumn::new("filterMetadata", "childNodeId");
    pub const FILTER_BINDING_SET: FluoColumn = FluoColumn::new("filterMetadata", "bindingSet");

    pub const JOIN_NODE_ID: FluoColumn = FluoColumn::new("joinMetadata", "nodeId");
    pub const JOIN_VARIABLE_ORDER: FluoColumn = FluoColumn::new("joinMetadata", "variableOrder");
    pub const JOIN_TYPE: FluoColumn = FluoColumn::new("joinMetadata", "joinType");
    pub const JOIN_PARENT_NODE_ID: FluoColumn = FluoColumn::new("joinMetadata", "parentNodeId");
    pub const JOIN_LEFT_CHILD_NODE_ID: FluoColumn =
        FluoColumn::new("joinMetadata", "leftChildNodeId");
    pub const JOIN_RIGHT_CHILD_NODE_ID: FluoColumn =
        FluoColumn::new("joinMetadata", "rightChildNodeId");
    pub const JOIN_BINDING_SET: FluoColumn = FluoColumn::new("joinMetadata", "bindingSet");

    pub const STATEMENT_PATTERN_NODE_ID: FluoColumn =
        FluoColumn::new("statementPatternMetadata", "nodeId");
    pub const STATEMENT_PATTERN_VARIABLE_ORDER: FluoColumn =
        FluoColumn::new("statementPatternMetadata", "variableOrder");
    pub const STATEMENT_PATTERN_PATTERN: FluoColumn =
        FluoColumn::new("statementPatternMetadata", "pattern");
    pub const STATEMENT_PATTERN_PARENT_NODE_ID: FluoColumn =
        FluoColumn::new("statementPatternMetadata", "parentNodeId");
    pub const STATEMENT_PATTERN_BINDING_SET: FluoColumn =
        FluoColumn::new("statementPatternMetadata", "bindingSet");
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InMemoryFluo {
    cells: BTreeMap<(String, FluoColumn), String>,
}

impl InMemoryFluo {
    pub fn set(&mut self, row: impl Into<String>, column: FluoColumn, value: impl Into<String>) {
        self.cells.insert((row.into(), column), value.into());
    }

    pub fn get(&self, row: &str, column: FluoColumn) -> Option<&str> {
        self.cells
            .get(&(row.to_string(), column))
            .map(String::as_str)
    }

    pub fn delete(&mut self, row: &str, column: FluoColumn) {
        self.cells.remove(&(row.to_string(), column));
    }

    pub fn scan_column(&self, column: FluoColumn) -> Vec<(&str, &str)> {
        self.cells
            .iter()
            .filter_map(|((row, col), value)| {
                (*col == column).then_some((row.as_str(), value.as_str()))
            })
            .collect()
    }

    pub fn scan_prefix(&self, prefix: &str, column: FluoColumn) -> Vec<(&str, &str)> {
        self.scan_column(column)
            .into_iter()
            .filter(|(row, _)| row.starts_with(prefix))
            .collect()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeType {
    Filter,
    Join,
    StatementPattern,
    Query,
}

impl NodeType {
    pub fn from_node_id(node_id: &str) -> Option<Self> {
        if node_id.starts_with(SP_PREFIX) {
            Some(Self::StatementPattern)
        } else if node_id.starts_with(FILTER_PREFIX) {
            Some(Self::Filter)
        } else if node_id.starts_with(JOIN_PREFIX) {
            Some(Self::Join)
        } else if node_id.starts_with(QUERY_PREFIX) {
            Some(Self::Query)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum FluoTerm {
    Var(String),
    Constant(RyaType),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct FluoStatementPattern {
    pub subject: FluoTerm,
    pub predicate: FluoTerm,
    pub object: FluoTerm,
}

impl FluoStatementPattern {
    pub fn new(subject: FluoTerm, predicate: FluoTerm, object: FluoTerm) -> Self {
        Self {
            subject,
            predicate,
            object,
        }
    }

    pub fn variables(&self) -> BTreeSet<String> {
        [&self.subject, &self.predicate, &self.object]
            .into_iter()
            .filter_map(|term| match term {
                FluoTerm::Var(name) => Some(name.clone()),
                FluoTerm::Constant(_) => None,
            })
            .collect()
    }
}

pub fn to_var_order_string(vars: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    vars.into_iter()
        .map(|var| var.as_ref().to_string())
        .collect::<Vec<_>>()
        .join(VAR_DELIM)
}

pub fn to_var_order(value: &str) -> Vec<String> {
    if value.is_empty() {
        Vec::new()
    } else {
        value.split(VAR_DELIM).map(str::to_string).collect()
    }
}

pub fn to_binding_set_string(
    binding_set: &BindingSet,
    order: &VariableOrder,
) -> Result<String, String> {
    binding_set_to_string(binding_set, order)
}

pub fn to_binding_set(value: &str, order: &VariableOrder) -> Result<BindingSet, String> {
    binding_set_from_string(value, order)
}

pub fn to_binding_strings(value: &str) -> Vec<String> {
    if value.is_empty() {
        Vec::new()
    } else {
        value.split(DELIM).map(str::to_string).collect()
    }
}

pub fn value_to_fluo_string(value: &RyaType) -> String {
    format!(
        "{}{}{}",
        value.data(),
        TYPE_DELIM,
        value.data_type().unwrap_or(XSD_STRING)
    )
}

pub fn value_from_fluo_string(value: &str) -> Result<RyaType, String> {
    let parts = value.split(TYPE_DELIM).collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err("Array must contain data and type info!".to_string());
    }
    if parts[1] == URI_TYPE {
        Ok(iri(parts[0])?.into_type())
    } else {
        Ok(RyaType::custom(parts[1], parts[0]))
    }
}

pub fn statement_pattern_to_string(pattern: &FluoStatementPattern) -> String {
    [
        term_to_pattern_part(&pattern.subject, true),
        term_to_pattern_part(&pattern.predicate, true),
        term_to_pattern_part(&pattern.object, false),
    ]
    .join(DELIM)
}

pub fn statement_pattern_from_string(value: &str) -> Result<FluoStatementPattern, String> {
    let parts = value.split(DELIM).collect::<Vec<_>>();
    if parts.len() != 3 {
        return Err("Statement pattern must contain subject, predicate, and object".to_string());
    }
    Ok(FluoStatementPattern::new(
        term_from_pattern_part(parts[0])?,
        term_from_pattern_part(parts[1])?,
        term_from_pattern_part(parts[2])?,
    ))
}

fn term_to_pattern_part(term: &FluoTerm, uri_position: bool) -> String {
    match term {
        FluoTerm::Var(name) => name.clone(),
        FluoTerm::Constant(value) => {
            let data_type = if uri_position {
                URI_TYPE
            } else {
                value.data_type().unwrap_or(XSD_STRING)
            };
            format!("-const-{}{}{}", value.data(), TYPE_DELIM, data_type)
        }
    }
}

fn term_from_pattern_part(value: &str) -> Result<FluoTerm, String> {
    if let Some(rest) = value.strip_prefix("-const-") {
        let parts = rest.split(TYPE_DELIM).collect::<Vec<_>>();
        if parts.len() != 2 {
            return Err("Array must contain data and type info!".to_string());
        }
        let data = parts[0];
        let data_type = parts[1];
        if data_type == URI_TYPE {
            Ok(FluoTerm::Constant(iri(data)?.into_type()))
        } else {
            Ok(FluoTerm::Constant(RyaType::custom(data_type, data)))
        }
    } else {
        Ok(FluoTerm::Var(value.to_string()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BindingSetRow {
    pub node_id: String,
    pub binding_set_string: String,
}

impl BindingSetRow {
    pub fn make(row: &str) -> Result<Self, String> {
        let parts = row.split(NODEID_BS_DELIM).collect::<Vec<_>>();
        if parts.len() != 2 {
            return Err("A row must contain a single NODEID_BS_DELIM.".to_string());
        }
        Ok(Self {
            node_id: parts[0].to_string(),
            binding_set_string: parts[1].to_string(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryMetadata {
    pub node_id: String,
    pub variable_order: VariableOrder,
    pub sparql: String,
    pub child_node_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatementPatternMetadata {
    pub node_id: String,
    pub variable_order: VariableOrder,
    pub statement_pattern: String,
    pub parent_node_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilterMetadata {
    pub node_id: String,
    pub variable_order: VariableOrder,
    pub original_sparql: String,
    pub filter_index_within_sparql: usize,
    pub parent_node_id: String,
    pub child_node_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JoinMetadata {
    pub node_id: String,
    pub variable_order: VariableOrder,
    pub join_type: JoinType,
    pub parent_node_id: String,
    pub left_child_node_id: String,
    pub right_child_node_id: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JoinType {
    NaturalJoin,
    LeftOuterJoin,
}

impl JoinType {
    pub fn as_storage_name(self) -> &'static str {
        match self {
            Self::NaturalJoin => "NATURAL_JOIN",
            Self::LeftOuterJoin => "LEFT_OUTER_JOIN",
        }
    }

    pub fn parse_storage_name(value: &str) -> Result<Self, String> {
        match value {
            "NATURAL_JOIN" => Ok(Self::NaturalJoin),
            "LEFT_OUTER_JOIN" => Ok(Self::LeftOuterJoin),
            _ => Err(format!("Unsupported JoinType: {value}")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FluoQuery {
    pub query_metadata: QueryMetadata,
    pub statement_patterns: BTreeMap<String, StatementPatternMetadata>,
    pub filters: BTreeMap<String, FilterMetadata>,
    pub joins: BTreeMap<String, JoinMetadata>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FluoQueryMetadataDao;

impl FluoQueryMetadataDao {
    pub fn write_query(&self, fluo: &mut InMemoryFluo, metadata: &QueryMetadata) {
        let row = &metadata.node_id;
        fluo.set(row, columns::QUERY_NODE_ID, row);
        fluo.set(
            row,
            columns::QUERY_VARIABLE_ORDER,
            metadata.variable_order.to_string(),
        );
        fluo.set(row, columns::QUERY_SPARQL, &metadata.sparql);
        fluo.set(row, columns::QUERY_CHILD_NODE_ID, &metadata.child_node_id);
    }

    pub fn read_query(&self, fluo: &InMemoryFluo, node_id: &str) -> Result<QueryMetadata, String> {
        Ok(QueryMetadata {
            node_id: node_id.to_string(),
            variable_order: VariableOrder::parse(required(
                fluo,
                node_id,
                columns::QUERY_VARIABLE_ORDER,
            )?),
            sparql: required(fluo, node_id, columns::QUERY_SPARQL)?.to_string(),
            child_node_id: required(fluo, node_id, columns::QUERY_CHILD_NODE_ID)?.to_string(),
        })
    }

    pub fn write_statement_pattern(
        &self,
        fluo: &mut InMemoryFluo,
        metadata: &StatementPatternMetadata,
    ) {
        let row = &metadata.node_id;
        fluo.set(row, columns::STATEMENT_PATTERN_NODE_ID, row);
        fluo.set(
            row,
            columns::STATEMENT_PATTERN_VARIABLE_ORDER,
            metadata.variable_order.to_string(),
        );
        fluo.set(
            row,
            columns::STATEMENT_PATTERN_PATTERN,
            &metadata.statement_pattern,
        );
        fluo.set(
            row,
            columns::STATEMENT_PATTERN_PARENT_NODE_ID,
            &metadata.parent_node_id,
        );
    }

    pub fn read_statement_pattern(
        &self,
        fluo: &InMemoryFluo,
        node_id: &str,
    ) -> Result<StatementPatternMetadata, String> {
        Ok(StatementPatternMetadata {
            node_id: node_id.to_string(),
            variable_order: VariableOrder::parse(required(
                fluo,
                node_id,
                columns::STATEMENT_PATTERN_VARIABLE_ORDER,
            )?),
            statement_pattern: required(fluo, node_id, columns::STATEMENT_PATTERN_PATTERN)?
                .to_string(),
            parent_node_id: required(fluo, node_id, columns::STATEMENT_PATTERN_PARENT_NODE_ID)?
                .to_string(),
        })
    }

    pub fn write_filter(&self, fluo: &mut InMemoryFluo, metadata: &FilterMetadata) {
        let row = &metadata.node_id;
        fluo.set(row, columns::FILTER_NODE_ID, row);
        fluo.set(
            row,
            columns::FILTER_VARIABLE_ORDER,
            metadata.variable_order.to_string(),
        );
        fluo.set(
            row,
            columns::FILTER_ORIGINAL_SPARQL,
            &metadata.original_sparql,
        );
        fluo.set(
            row,
            columns::FILTER_INDEX_WITHIN_SPARQL,
            metadata.filter_index_within_sparql.to_string(),
        );
        fluo.set(
            row,
            columns::FILTER_PARENT_NODE_ID,
            &metadata.parent_node_id,
        );
        fluo.set(row, columns::FILTER_CHILD_NODE_ID, &metadata.child_node_id);
    }

    pub fn read_filter(
        &self,
        fluo: &InMemoryFluo,
        node_id: &str,
    ) -> Result<FilterMetadata, String> {
        Ok(FilterMetadata {
            node_id: node_id.to_string(),
            variable_order: VariableOrder::parse(required(
                fluo,
                node_id,
                columns::FILTER_VARIABLE_ORDER,
            )?),
            original_sparql: required(fluo, node_id, columns::FILTER_ORIGINAL_SPARQL)?.to_string(),
            filter_index_within_sparql: required(
                fluo,
                node_id,
                columns::FILTER_INDEX_WITHIN_SPARQL,
            )?
            .parse()
            .map_err(|_| "Invalid filter index".to_string())?,
            parent_node_id: required(fluo, node_id, columns::FILTER_PARENT_NODE_ID)?.to_string(),
            child_node_id: required(fluo, node_id, columns::FILTER_CHILD_NODE_ID)?.to_string(),
        })
    }

    pub fn write_join(&self, fluo: &mut InMemoryFluo, metadata: &JoinMetadata) {
        let row = &metadata.node_id;
        fluo.set(row, columns::JOIN_NODE_ID, row);
        fluo.set(
            row,
            columns::JOIN_VARIABLE_ORDER,
            metadata.variable_order.to_string(),
        );
        fluo.set(row, columns::JOIN_TYPE, metadata.join_type.as_storage_name());
        fluo.set(row, columns::JOIN_PARENT_NODE_ID, &metadata.parent_node_id);
        fluo.set(
            row,
            columns::JOIN_LEFT_CHILD_NODE_ID,
            &metadata.left_child_node_id,
        );
        fluo.set(
            row,
            columns::JOIN_RIGHT_CHILD_NODE_ID,
            &metadata.right_child_node_id,
        );
    }

    pub fn read_join(&self, fluo: &InMemoryFluo, node_id: &str) -> Result<JoinMetadata, String> {
        Ok(JoinMetadata {
            node_id: node_id.to_string(),
            variable_order: VariableOrder::parse(required(
                fluo,
                node_id,
                columns::JOIN_VARIABLE_ORDER,
            )?),
            join_type: JoinType::parse_storage_name(required(fluo, node_id, columns::JOIN_TYPE)?)?,
            parent_node_id: required(fluo, node_id, columns::JOIN_PARENT_NODE_ID)?.to_string(),
            left_child_node_id: required(fluo, node_id, columns::JOIN_LEFT_CHILD_NODE_ID)?
                .to_string(),
            right_child_node_id: required(fluo, node_id, columns::JOIN_RIGHT_CHILD_NODE_ID)?
                .to_string(),
        })
    }

    pub fn write_fluo_query(&self, fluo: &mut InMemoryFluo, query: &FluoQuery) {
        fluo.set(
            &query.query_metadata.sparql,
            columns::QUERY_ID,
            &query.query_metadata.node_id,
        );
        self.write_query(fluo, &query.query_metadata);
        for metadata in query.statement_patterns.values() {
            self.write_statement_pattern(fluo, metadata);
        }
        for metadata in query.filters.values() {
            self.write_filter(fluo, metadata);
        }
        for metadata in query.joins.values() {
            self.write_join(fluo, metadata);
        }
    }
}

fn required<'a>(fluo: &'a InMemoryFluo, row: &str, column: FluoColumn) -> Result<&'a str, String> {
    fluo.get(row, column).ok_or_else(|| {
        format!(
            "Missing Fluo value at row '{row}', column '{}:{}'",
            column.family, column.qualifier
        )
    })
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryFluoPcjApp {
    pub fluo: InMemoryFluo,
    pub pcj_tables: InMemoryPcjTables,
    rya_statements: Vec<VisibleStatement>,
    streamed_statements: Vec<VisibleStatement>,
    queries: BTreeMap<String, QueryPlan>,
    next_id: usize,
}

impl InMemoryFluoPcjApp {
    pub fn add_historic_statement(&mut self, statement: RyaStatement) {
        self.rya_statements
            .push(VisibleStatement::new(statement, ""));
    }

    pub fn insert_triples(&mut self, triples: impl IntoIterator<Item = RyaStatement>) {
        self.insert_triples_with_visibility(
            triples.into_iter().map(|triple| (triple, String::new())),
        )
    }

    pub fn insert_triples_with_visibility(
        &mut self,
        triples: impl IntoIterator<Item = (RyaStatement, String)>,
    ) {
        for (triple, visibility) in triples {
            let row = spo_format(&triple);
            self.fluo.set(row, columns::TRIPLES, &visibility);
            self.streamed_statements
                .push(VisibleStatement::new(triple, visibility));
        }
        self.recompute_all_queries();
    }

    pub fn count_unprocessed_statements(&self) -> u128 {
        self.fluo.scan_column(columns::TRIPLES).len() as u128
    }

    pub fn create_pcj(
        &mut self,
        rya_table_prefix: &str,
        var_orders: impl IntoIterator<Item = VariableOrder>,
        sparql: &str,
    ) -> Result<String, String> {
        let mut plan = QueryPlan::parse(sparql)?;
        let query_id = self.make_node_id(QUERY_PREFIX);
        let child_id = self.populate_plan_ids(&mut plan, &query_id);
        let query_order = VariableOrder::new(plan.projected_vars.clone());
        let metadata = plan.to_fluo_query(query_id.clone(), child_id, query_order.clone());
        FluoQueryMetadataDao.write_fluo_query(&mut self.fluo, &metadata);

        let export_table = make_pcj_table_name(rya_table_prefix, &query_id);
        let export_orders = {
            let supplied = var_orders.into_iter().collect::<BTreeSet<_>>();
            if supplied.is_empty() {
                query_order.shifted_orders()
            } else {
                supplied
            }
        };
        self.pcj_tables
            .create_pcj_table(&export_table, export_orders, sparql);
        self.fluo.set(
            &query_id,
            columns::QUERY_RYA_EXPORT_TABLE_NAME,
            &export_table,
        );

        self.queries.insert(query_id.clone(), plan);
        self.recompute_query(&query_id)?;
        Ok(query_id)
    }

    pub fn list_query_ids(&self) -> Vec<String> {
        let mut ids = self
            .fluo
            .scan_column(columns::QUERY_ID)
            .into_iter()
            .map(|(_, value)| value.to_string())
            .collect::<Vec<_>>();
        ids.sort();
        ids
    }

    pub fn get_pcj_metadata(&self, query_id: &str) -> Result<PcjMetadata, String> {
        let table = self
            .fluo
            .get(query_id, columns::QUERY_RYA_EXPORT_TABLE_NAME)
            .ok_or_else(|| {
                format!(
                    "Could not get the PcjMetadata for queryId '{query_id}' because a PCJ export table name was not stored in the Fluo table."
                )
            })?;
        self.pcj_tables.get_pcj_metadata(table).map_err(|err| {
            format!(
                "Could not get the PcjMetadata for queryId '{query_id}' because the metadata was missing from the Fjall table: {err}"
            )
        })
    }

    pub fn get_all_pcj_metadata(&self) -> Result<BTreeMap<String, PcjMetadata>, String> {
        let mut metadata = BTreeMap::new();
        for query_id in self.list_query_ids() {
            metadata.insert(query_id.clone(), self.get_pcj_metadata(&query_id)?);
        }
        Ok(metadata)
    }

    pub fn get_query_report(&self, query_id: &str) -> Result<QueryReport, String> {
        let plan = self
            .queries
            .get(query_id)
            .ok_or_else(|| format!("Unknown query id: {query_id}"))?;
        let fluo_query = plan.to_fluo_query(
            query_id.to_string(),
            plan.root_child_id.clone(),
            VariableOrder::new(plan.projected_vars.clone()),
        );
        let mut counts = BTreeMap::new();
        counts.insert(
            query_id.to_string(),
            count_rows_for(&self.fluo, query_id, columns::QUERY_BINDING_SET),
        );
        for metadata in fluo_query.statement_patterns.values() {
            counts.insert(
                metadata.node_id.clone(),
                count_rows_for(
                    &self.fluo,
                    &metadata.node_id,
                    columns::STATEMENT_PATTERN_BINDING_SET,
                ),
            );
        }
        for metadata in fluo_query.joins.values() {
            counts.insert(
                metadata.node_id.clone(),
                count_rows_for(&self.fluo, &metadata.node_id, columns::JOIN_BINDING_SET),
            );
        }
        for metadata in fluo_query.filters.values() {
            counts.insert(
                metadata.node_id.clone(),
                count_rows_for(&self.fluo, &metadata.node_id, columns::FILTER_BINDING_SET),
            );
        }
        Ok(QueryReport { fluo_query, counts })
    }

    pub fn query_results(&self, sparql: &str) -> Result<BTreeSet<BindingSet>, String> {
        let query_id = self
            .fluo
            .get(sparql, columns::QUERY_ID)
            .ok_or_else(|| "Query not found".to_string())?;
        let order = self
            .queries
            .get(query_id)
            .map(|plan| VariableOrder::new(plan.projected_vars.clone()))
            .ok_or_else(|| "Query plan not found".to_string())?;
        self.fluo
            .scan_prefix(
                &format!("{query_id}{NODEID_BS_DELIM}"),
                columns::QUERY_BINDING_SET,
            )
            .into_iter()
            .map(|(_, value)| {
                visibility_binding_set_from_string(value, &order).map(|row| row.bindings)
            })
            .collect()
    }

    fn make_node_id(&mut self, prefix: &str) -> String {
        self.next_id += 1;
        format!("{prefix}_{:032x}", self.next_id)
    }

    fn populate_plan_ids(&mut self, plan: &mut QueryPlan, query_id: &str) -> String {
        for pattern in &mut plan.patterns {
            pattern.node_id = self.make_node_id(SP_PREFIX);
        }
        for pattern in &mut plan.optional_patterns {
            pattern.node_id = self.make_node_id(SP_PREFIX);
        }
        for filter in &mut plan.filters {
            filter.node_id = self.make_node_id(FILTER_PREFIX);
        }
        plan.join_node_id = (plan.patterns.len() + plan.optional_patterns.len() > 1)
            .then(|| self.make_node_id(JOIN_PREFIX));
        plan.query_id = query_id.to_string();
        plan.root_child_id = plan
            .filters
            .last()
            .map(|filter| filter.node_id.clone())
            .or_else(|| plan.join_node_id.clone())
            .unwrap_or_else(|| plan.patterns[0].node_id.clone());
        plan.root_child_id.clone()
    }

    fn recompute_all_queries(&mut self) {
        let ids = self.queries.keys().cloned().collect::<Vec<_>>();
        for id in ids {
            let _ = self.recompute_query(&id);
        }
    }

    fn recompute_query(&mut self, query_id: &str) -> Result<(), String> {
        let plan = self
            .queries
            .get(query_id)
            .cloned()
            .ok_or_else(|| format!("Unknown query id: {query_id}"))?;
        self.clear_result_rows_for(&plan);

        let all_statements = self
            .rya_statements
            .iter()
            .chain(&self.streamed_statements)
            .cloned()
            .collect::<Vec<_>>();

        let mut pattern_results = Vec::new();
        for pattern in &plan.patterns {
            let rows = pattern
                .pattern
                .match_statements(&all_statements)
                .into_iter()
                .collect::<BTreeSet<_>>();
            write_binding_rows(
                &mut self.fluo,
                &pattern.node_id,
                columns::STATEMENT_PATTERN_BINDING_SET,
                &pattern.variable_order(),
                &rows,
            )?;
            pattern_results.push(rows);
        }

        let mut required_joined = if let Some(first) = pattern_results.first() {
            first.clone()
        } else {
            BTreeSet::new()
        };
        for rows in pattern_results.iter().skip(1) {
            required_joined = visible_natural_join(&required_joined, rows);
        }

        let mut optional_results = Vec::new();
        for pattern in &plan.optional_patterns {
            let rows = pattern
                .pattern
                .match_statements(&all_statements)
                .into_iter()
                .collect::<BTreeSet<_>>();
            write_binding_rows(
                &mut self.fluo,
                &pattern.node_id,
                columns::STATEMENT_PATTERN_BINDING_SET,
                &pattern.variable_order(),
                &rows,
            )?;
            optional_results.push(rows);
        }

        let joined = if let Some(first_optional) = optional_results.first() {
            let mut optional_joined = first_optional.clone();
            for rows in optional_results.iter().skip(1) {
                optional_joined = visible_natural_join(&optional_joined, rows);
            }
            visible_left_join(&required_joined, &optional_joined)
        } else {
            required_joined
        };

        if let Some(join_id) = &plan.join_node_id {
            write_binding_rows(
                &mut self.fluo,
                join_id,
                columns::JOIN_BINDING_SET,
                &VariableOrder::new(plan.join_variables()),
                &joined,
            )?;
        }

        let mut filtered = joined;
        for filter in &plan.filters {
            filtered = filtered
                .into_iter()
                .filter(|row| filter.expression.accepts(&row.bindings))
                .collect();
            write_binding_rows(
                &mut self.fluo,
                &filter.node_id,
                columns::FILTER_BINDING_SET,
                &VariableOrder::new(plan.join_variables()),
                &filtered,
            )?;
        }

        let projected = filtered
            .into_iter()
            .map(|row| {
                VisibilityBindingSet::new(
                    project_binding_set(&row.bindings, &plan.projected_vars),
                    row.visibility,
                )
            })
            .collect::<BTreeSet<_>>();

        write_binding_rows(
            &mut self.fluo,
            query_id,
            columns::QUERY_BINDING_SET,
            &VariableOrder::new(plan.projected_vars.clone()),
            &projected,
        )?;

        if let Some(export_table) = self
            .fluo
            .get(query_id, columns::QUERY_RYA_EXPORT_TABLE_NAME)
            .map(str::to_string)
        {
            self.pcj_tables.purge_pcj_table(&export_table)?;
            self.pcj_tables
                .add_visibility_results(&export_table, projected.into_iter())?;
        }

        Ok(())
    }

    fn clear_result_rows_for(&mut self, plan: &QueryPlan) {
        let mut rows = Vec::new();
        for node_id in plan
            .patterns
            .iter()
            .chain(plan.optional_patterns.iter())
            .map(|pattern| pattern.node_id.as_str())
            .chain(plan.join_node_id.iter().map(String::as_str))
            .chain(plan.filters.iter().map(|filter| filter.node_id.as_str()))
            .chain(std::iter::once(plan.query_id.as_str()))
        {
            for column in [
                columns::STATEMENT_PATTERN_BINDING_SET,
                columns::JOIN_BINDING_SET,
                columns::FILTER_BINDING_SET,
                columns::QUERY_BINDING_SET,
            ] {
                for (row, _) in self
                    .fluo
                    .scan_prefix(&format!("{node_id}{NODEID_BS_DELIM}"), column)
                {
                    rows.push((row.to_string(), column));
                }
            }
        }
        for (row, column) in rows {
            self.fluo.delete(&row, column);
        }
    }
}

fn count_rows_for(fluo: &InMemoryFluo, node_id: &str, column: FluoColumn) -> u128 {
    fluo.scan_prefix(&format!("{node_id}{NODEID_BS_DELIM}"), column)
        .len() as u128
}

fn write_binding_rows(
    fluo: &mut InMemoryFluo,
    node_id: &str,
    column: FluoColumn,
    order: &VariableOrder,
    rows: &BTreeSet<VisibilityBindingSet>,
) -> Result<(), String> {
    for row in rows {
        let binding_string = binding_set_to_string(&row.bindings, order)?;
        let value_string = visibility_binding_set_to_string(row, order)?;
        fluo.set(
            format!("{node_id}{NODEID_BS_DELIM}{binding_string}"),
            column,
            value_string,
        );
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct VisibleStatement {
    statement: RyaStatement,
    visibility: String,
}

impl VisibleStatement {
    fn new(statement: RyaStatement, visibility: impl Into<String>) -> Self {
        Self {
            statement,
            visibility: visibility.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueryReport {
    pub fluo_query: FluoQuery,
    pub counts: BTreeMap<String, u128>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct QueryPlan {
    sparql: String,
    projected_vars: Vec<String>,
    patterns: Vec<PlannedPattern>,
    optional_patterns: Vec<PlannedPattern>,
    filters: Vec<PlannedFilter>,
    query_id: String,
    root_child_id: String,
    join_node_id: Option<String>,
}

impl QueryPlan {
    fn parse(sparql: &str) -> Result<Self, String> {
        let projected_vars = parse_select_vars(sparql)?;
        let body = sparql_body(sparql)?;
        let (filter_expressions, body_without_filters) = extract_filters(body)?;
        let (required_body, optional_patterns) = extract_optional_patterns(&body_without_filters)?;
        let patterns = parse_patterns(&required_body)?;
        if patterns.is_empty() {
            return Err("SPARQL query must contain at least one statement pattern".to_string());
        }
        Ok(Self {
            sparql: sparql.to_string(),
            projected_vars,
            patterns: patterns
                .into_iter()
                .map(|pattern| PlannedPattern {
                    node_id: String::new(),
                    pattern,
                })
                .collect(),
            optional_patterns: optional_patterns
                .into_iter()
                .map(|pattern| PlannedPattern {
                    node_id: String::new(),
                    pattern,
                })
                .collect(),
            filters: filter_expressions
                .into_iter()
                .enumerate()
                .map(|(index, expression)| PlannedFilter {
                    node_id: String::new(),
                    index,
                    expression,
                })
                .collect(),
            query_id: String::new(),
            root_child_id: String::new(),
            join_node_id: None,
        })
    }

    fn join_variables(&self) -> Vec<String> {
        self.patterns
            .iter()
            .chain(self.optional_patterns.iter())
            .flat_map(|pattern| pattern.pattern.variables())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn to_fluo_query(
        &self,
        query_id: String,
        child_id: String,
        query_order: VariableOrder,
    ) -> FluoQuery {
        let mut statement_patterns = BTreeMap::new();
        let mut filters = BTreeMap::new();
        let mut joins = BTreeMap::new();

        let root_after_join = self
            .filters
            .first()
            .map(|filter| filter.node_id.clone())
            .unwrap_or_else(|| query_id.clone());
        let join_or_query_parent = self
            .join_node_id
            .as_ref()
            .map(|_| root_after_join.clone())
            .unwrap_or_else(|| query_id.clone());

        for pattern in self.patterns.iter().chain(self.optional_patterns.iter()) {
            let parent = self
                .join_node_id
                .clone()
                .unwrap_or_else(|| join_or_query_parent.clone());
            statement_patterns.insert(
                pattern.node_id.clone(),
                StatementPatternMetadata {
                    node_id: pattern.node_id.clone(),
                    variable_order: pattern.variable_order(),
                    statement_pattern: statement_pattern_to_string(&pattern.pattern),
                    parent_node_id: parent,
                },
            );
        }

        if let Some(join_id) = &self.join_node_id {
            let right_child_node_id = self
                .optional_patterns
                .first()
                .or_else(|| self.patterns.get(1))
                .map(|pattern| pattern.node_id.clone())
                .unwrap_or_else(|| self.patterns[0].node_id.clone());
            joins.insert(
                join_id.clone(),
                JoinMetadata {
                    node_id: join_id.clone(),
                    variable_order: VariableOrder::new(self.join_variables()),
                    join_type: if self.optional_patterns.is_empty() {
                        JoinType::NaturalJoin
                    } else {
                        JoinType::LeftOuterJoin
                    },
                    parent_node_id: root_after_join,
                    left_child_node_id: self.patterns[0].node_id.clone(),
                    right_child_node_id,
                },
            );
        }

        for (idx, filter) in self.filters.iter().enumerate() {
            let parent = self
                .filters
                .get(idx + 1)
                .map(|next| next.node_id.clone())
                .unwrap_or_else(|| query_id.clone());
            let child = if idx == 0 {
                self.join_node_id
                    .clone()
                    .unwrap_or_else(|| self.patterns[0].node_id.clone())
            } else {
                self.filters[idx - 1].node_id.clone()
            };
            filters.insert(
                filter.node_id.clone(),
                FilterMetadata {
                    node_id: filter.node_id.clone(),
                    variable_order: VariableOrder::new(self.join_variables()),
                    original_sparql: self.sparql.clone(),
                    filter_index_within_sparql: filter.index,
                    parent_node_id: parent,
                    child_node_id: child,
                },
            );
        }

        FluoQuery {
            query_metadata: QueryMetadata {
                node_id: query_id,
                variable_order: query_order,
                sparql: self.sparql.clone(),
                child_node_id: child_id,
            },
            statement_patterns,
            filters,
            joins,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PlannedPattern {
    node_id: String,
    pattern: FluoStatementPattern,
}

impl PlannedPattern {
    fn variable_order(&self) -> VariableOrder {
        VariableOrder::new(self.pattern.variables())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PlannedFilter {
    node_id: String,
    index: usize,
    expression: FilterExpression,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum FilterExpression {
    Eq(String, RyaType),
    Lt(String, i64),
}

impl FilterExpression {
    fn accepts(&self, row: &BindingSet) -> bool {
        match self {
            Self::Eq(name, expected) => row.get(name).is_some_and(|value| value == expected),
            Self::Lt(name, max) => row
                .get(name)
                .and_then(|value| value.data().parse::<i64>().ok())
                .is_some_and(|value| value < *max),
        }
    }
}

impl FluoStatementPattern {
    fn match_statements(&self, statements: &[VisibleStatement]) -> Vec<VisibilityBindingSet> {
        statements
            .iter()
            .filter_map(|statement| self.match_statement(statement))
            .collect()
    }

    fn match_statement(&self, statement: &VisibleStatement) -> Option<VisibilityBindingSet> {
        let mut row = BindingSet::new();
        bind_term(
            &self.subject,
            statement.statement.subject.as_type(),
            &mut row,
        )?;
        bind_term(
            &self.predicate,
            statement.statement.predicate.as_type(),
            &mut row,
        )?;
        bind_term(&self.object, &statement.statement.object, &mut row)?;
        Some(VisibilityBindingSet::new(row, statement.visibility.clone()))
    }
}

fn visible_natural_join(
    left: &BTreeSet<VisibilityBindingSet>,
    right: &BTreeSet<VisibilityBindingSet>,
) -> BTreeSet<VisibilityBindingSet> {
    let mut out = BTreeSet::new();
    for left_row in left {
        for right_row in right {
            if let Some(joined) = join_visible_compatible(left_row, right_row) {
                out.insert(joined);
            }
        }
    }
    out
}

fn visible_left_join(
    left: &BTreeSet<VisibilityBindingSet>,
    right: &BTreeSet<VisibilityBindingSet>,
) -> BTreeSet<VisibilityBindingSet> {
    let mut out = BTreeSet::new();
    for left_row in left {
        let mut matched = false;
        for right_row in right {
            if let Some(joined) = join_visible_compatible(left_row, right_row) {
                out.insert(joined);
                matched = true;
            }
        }
        if !matched {
            out.insert(left_row.clone());
        }
    }
    out
}

fn join_visible_compatible(
    left: &VisibilityBindingSet,
    right: &VisibilityBindingSet,
) -> Option<VisibilityBindingSet> {
    for (name, value) in &left.bindings {
        if right
            .bindings
            .get(name)
            .is_some_and(|right_value| right_value != value)
        {
            return None;
        }
    }
    let mut joined = left.bindings.clone();
    for (name, value) in &right.bindings {
        joined.entry(name.clone()).or_insert_with(|| value.clone());
    }
    Some(VisibilityBindingSet::new(
        joined,
        join_visibility(&left.visibility, &right.visibility),
    ))
}

fn join_visibility(left: &str, right: &str) -> String {
    match (left.is_empty(), right.is_empty()) {
        (true, true) => String::new(),
        (true, false) => right.trim().to_string(),
        (false, true) => left.trim().to_string(),
        (false, false) => format!("({left})&({right})"),
    }
}

fn bind_term(term: &FluoTerm, value: &RyaType, row: &mut BindingSet) -> Option<()> {
    match term {
        FluoTerm::Constant(expected) => (expected == value).then_some(()),
        FluoTerm::Var(name) => {
            if row.get(name).is_none_or(|stored| stored == value) {
                row.insert(name.clone(), value.clone());
                Some(())
            } else {
                None
            }
        }
    }
}

fn project_binding_set(row: &BindingSet, projected_vars: &[String]) -> BindingSet {
    projected_vars
        .iter()
        .filter_map(|var| row.get(var).map(|value| (var.clone(), value.clone())))
        .collect()
}

pub fn spo_format(statement: &RyaStatement) -> String {
    [
        value_to_fluo_string(statement.subject.as_type()),
        value_to_fluo_string(statement.predicate.as_type()),
        value_to_fluo_string(&statement.object),
    ]
    .join(DELIM)
}

pub fn triple_string(statement: &RyaStatement) -> String {
    spo_format(statement)
}

fn parse_select_vars(sparql: &str) -> Result<Vec<String>, String> {
    let upper = sparql.to_ascii_uppercase();
    let select = upper
        .find("SELECT")
        .ok_or_else(|| "Only SELECT query fixtures are supported".to_string())?;
    let end = upper[select..]
        .find("WHERE")
        .map(|idx| select + idx)
        .or_else(|| sparql[select..].find('{').map(|idx| select + idx))
        .ok_or_else(|| "SELECT query fixture is missing WHERE".to_string())?;
    let clause = sparql[select + "SELECT".len()..end].trim();
    let vars = clause
        .split_whitespace()
        .filter(|token| !token.eq_ignore_ascii_case("DISTINCT"))
        .filter_map(|token| token.strip_prefix('?'))
        .map(clean_var)
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    if vars.is_empty() {
        Err("SELECT query fixture does not project any variables".to_string())
    } else {
        Ok(vars)
    }
}

fn sparql_body(sparql: &str) -> Result<&str, String> {
    let start = sparql
        .find('{')
        .ok_or_else(|| "SPARQL query is missing '{'".to_string())?;
    let end = sparql
        .rfind('}')
        .ok_or_else(|| "SPARQL query is missing '}'".to_string())?;
    if start >= end {
        return Err("SPARQL query body is malformed".to_string());
    }
    Ok(&sparql[start + 1..end])
}

fn extract_filters(body: &str) -> Result<(Vec<FilterExpression>, String), String> {
    let mut filters = Vec::new();
    let mut stripped = String::new();
    let mut index = 0;
    while let Some(found) = body[index..].to_ascii_uppercase().find("FILTER(") {
        let start = index + found;
        stripped.push_str(&body[index..start]);
        let expr_start = start + "FILTER(".len();
        let expr_end = body[expr_start..]
            .find(')')
            .map(|offset| expr_start + offset)
            .ok_or_else(|| "FILTER expression is missing ')'".to_string())?;
        filters.push(parse_filter_expr(&body[expr_start..expr_end])?);
        index = expr_end + 1;
    }
    stripped.push_str(&body[index..]);
    Ok((filters, stripped))
}

fn parse_filter_expr(expr: &str) -> Result<FilterExpression, String> {
    let expr = expr.trim();
    if let Some((left, right)) = expr.split_once('<').filter(|(left, _)| left.contains('?')) {
        if left.trim_end().ends_with('=') {
            let name = clean_var(left.trim().trim_end_matches('=').trim_start_matches('?'));
            let iri_value = right
                .split_once('>')
                .map(|(value, _)| value)
                .ok_or_else(|| "URI filter is missing '>'".to_string())?;
            return Ok(FilterExpression::Eq(name, iri(iri_value)?.into_type()));
        }
    }
    if let Some((left, right)) = expr.split_once('=') {
        let name = clean_var(left.trim().trim_start_matches('?'));
        return Ok(FilterExpression::Eq(
            name,
            parse_filter_value(right.trim())?,
        ));
    }
    if let Some((left, right)) = expr.split_once('<') {
        let name = clean_var(left.trim().trim_start_matches('?'));
        let max = right
            .trim()
            .parse::<i64>()
            .map_err(|_| "Numeric FILTER threshold must be an integer".to_string())?;
        return Ok(FilterExpression::Lt(name, max));
    }
    Err(format!("Unsupported FILTER expression: {expr}"))
}

fn parse_filter_value(token: &str) -> Result<RyaType, String> {
    match parse_query_term(token)? {
        FluoTerm::Constant(value) => Ok(value),
        FluoTerm::Var(name) => Err(format!(
            "FILTER value must be constant, found variable {name}"
        )),
    }
}

fn extract_optional_patterns(body: &str) -> Result<(String, Vec<FluoStatementPattern>), String> {
    let mut required_body = String::new();
    let mut optional_patterns = Vec::new();
    let mut index = 0;

    while let Some(found) = body[index..].to_ascii_uppercase().find("OPTIONAL") {
        let start = index + found;
        required_body.push_str(&body[index..start]);

        let mut cursor = start + "OPTIONAL".len();
        while body[cursor..].starts_with(char::is_whitespace) {
            cursor += body[cursor..]
                .chars()
                .next()
                .expect("cursor within string")
                .len_utf8();
        }
        if !body[cursor..].starts_with('{') {
            return Err("OPTIONAL block is missing '{'".to_string());
        }

        let open = cursor;
        let mut depth = 0usize;
        let mut close = None;
        for (offset, ch) in body[open..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        close = Some(open + offset);
                        break;
                    }
                }
                _ => {}
            }
        }
        let close = close.ok_or_else(|| "OPTIONAL block is missing '}'".to_string())?;
        optional_patterns.extend(parse_patterns(&body[open + 1..close])?);
        index = close + 1;
    }

    required_body.push_str(&body[index..]);
    Ok((required_body, optional_patterns))
}

fn parse_patterns(body: &str) -> Result<Vec<FluoStatementPattern>, String> {
    body.split('.')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(parse_pattern)
        .collect()
}

fn parse_pattern(part: &str) -> Result<FluoStatementPattern, String> {
    let tokens = tokenize_pattern(part);
    if tokens.len() != 3 {
        return Err(format!("Statement pattern must contain 3 terms: {part}"));
    }
    Ok(FluoStatementPattern::new(
        parse_query_term(&tokens[0])?,
        parse_query_term(&tokens[1])?,
        parse_query_term(&tokens[2])?,
    ))
}

fn tokenize_pattern(part: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for ch in part.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                current.push(ch);
            }
            ch if ch.is_whitespace() && !in_quotes => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

fn parse_query_term(token: &str) -> Result<FluoTerm, String> {
    let token = token.trim();
    if let Some(name) = token.strip_prefix('?') {
        Ok(FluoTerm::Var(clean_var(name)))
    } else if token.starts_with('<') && token.ends_with('>') {
        Ok(FluoTerm::Constant(
            iri(token.trim_start_matches('<').trim_end_matches('>'))?.into_type(),
        ))
    } else if token.starts_with('"') && token.ends_with('"') {
        Ok(FluoTerm::Constant(RyaType::new(token.trim_matches('"'))))
    } else if token.parse::<i64>().is_ok() {
        Ok(FluoTerm::Constant(RyaType::custom(XSD_INTEGER, token)))
    } else {
        Ok(FluoTerm::Constant(RyaType::new(token)))
    }
}

fn clean_var(value: &str) -> String {
    value
        .trim()
        .trim_start_matches('?')
        .trim_end_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .to_string()
}

fn iri(value: &str) -> Result<RyaIri, String> {
    RyaIri::new(value)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RyaExportParameters {
    params: BTreeMap<String, String>,
}

impl RyaExportParameters {
    pub const CONF_EXPORT_TO_RYA: &'static str = "pcj.fluo.export.rya.enabled";
    pub const CONF_FJALL_INSTANCE_NAME: &'static str = "pcj.fluo.export.rya.fjallInstanceName";
    pub const CONF_COORDINATOR_SERVERS: &'static str = "pcj.fluo.export.rya.coordinatorServers";
    pub const CONF_EXPORTER_USERNAME: &'static str = "pcj.fluo.export.rya.exporterUsername";
    pub const CONF_EXPORTER_PASSWORD: &'static str = "pcj.fluo.export.rya.exporterPassword";

    pub fn new(params: BTreeMap<String, String>) -> Self {
        Self { params }
    }

    pub fn into_inner(self) -> BTreeMap<String, String> {
        self.params
    }

    pub fn set_export_to_rya(&mut self, value: bool) {
        self.params
            .insert(Self::CONF_EXPORT_TO_RYA.to_string(), value.to_string());
    }

    pub fn is_export_to_rya(&self) -> bool {
        self.params
            .get(Self::CONF_EXPORT_TO_RYA)
            .is_some_and(|value| value == "true")
    }

    pub fn set_fjall_instance_name(&mut self, value: impl Into<String>) {
        self.params
            .insert(Self::CONF_FJALL_INSTANCE_NAME.to_string(), value.into());
    }

    pub fn fjall_instance_name(&self) -> Option<&str> {
        self.params
            .get(Self::CONF_FJALL_INSTANCE_NAME)
            .map(String::as_str)
    }

    pub fn set_coordinator_servers(&mut self, value: impl Into<String>) {
        self.params
            .insert(Self::CONF_COORDINATOR_SERVERS.to_string(), value.into());
    }

    pub fn coordinator_servers(&self) -> Option<&str> {
        self.params
            .get(Self::CONF_COORDINATOR_SERVERS)
            .map(String::as_str)
    }

    pub fn set_exporter_username(&mut self, value: impl Into<String>) {
        self.params
            .insert(Self::CONF_EXPORTER_USERNAME.to_string(), value.into());
    }

    pub fn exporter_username(&self) -> Option<&str> {
        self.params
            .get(Self::CONF_EXPORTER_USERNAME)
            .map(String::as_str)
    }

    pub fn set_exporter_password(&mut self, value: impl Into<String>) {
        self.params
            .insert(Self::CONF_EXPORTER_PASSWORD.to_string(), value.into());
    }

    pub fn exporter_password(&self) -> Option<&str> {
        self.params
            .get(Self::CONF_EXPORTER_PASSWORD)
            .map(String::as_str)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedQueryRequest {
    pub sparql: String,
    pub var_orders: BTreeSet<VariableOrder>,
}

impl ParsedQueryRequest {
    pub fn parse(text: &str) -> Self {
        let mut var_orders = BTreeSet::new();
        let mut sparql_start = 0;
        for (line_start, line) in line_offsets(text) {
            let trimmed = line.trim();
            if let Some(order) = trimmed.strip_prefix("#prefix ") {
                var_orders.insert(VariableOrder::new(
                    order.split(',').map(|part| part.trim().to_string()),
                ));
                sparql_start = line_start + line.len();
            }
        }
        Self {
            sparql: text[sparql_start..].trim().to_string(),
            var_orders,
        }
    }
}

fn line_offsets(text: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = 0;
    for line in text.split_inclusive('\n') {
        out.push((start, line.trim_end_matches('\n')));
        start += line.len();
    }
    if start < text.len() {
        out.push((start, &text[start..]));
    }
    out
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Report {
    items: Vec<ReportItem>,
}

impl Report {
    pub fn builder() -> ReportBuilder {
        ReportBuilder { items: Vec::new() }
    }
}

impl std::fmt::Display for Report {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let max_title = self
            .items
            .iter()
            .map(|item| item.title.len())
            .max()
            .unwrap_or(1);
        let max_value = self
            .items
            .iter()
            .flat_map(|item| item.value_lines.iter())
            .map(|line| line.len())
            .max()
            .unwrap_or(1);
        let line_len = 2 + max_title + 3 + max_value + 2;
        let dash = "-".repeat(line_len);
        writeln!(f, "{dash}")?;
        for item in &self.items {
            match item.value_lines.as_slice() {
                [] => writeln!(f, "| {:<max_title$} | {:<max_value$} |", item.title, "")?,
                [line] => writeln!(f, "| {:<max_title$} | {:<max_value$} |", item.title, line)?,
                [first, rest @ ..] => {
                    writeln!(f, "| {:<max_title$} | {:<max_value$} |", item.title, first)?;
                    for line in rest {
                        writeln!(f, "| {:<max_title$} | {:<max_value$} |", "", line)?;
                    }
                }
            }
        }
        writeln!(f, "{dash}")?;
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReportItem {
    title: String,
    value_lines: Vec<String>,
}

impl ReportItem {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            value_lines: Vec::new(),
        }
    }

    pub fn with_value(title: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            value_lines: vec![value.into()],
        }
    }

    pub fn with_lines(
        title: impl Into<String>,
        value_lines: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            title: title.into(),
            value_lines: value_lines.into_iter().map(Into::into).collect(),
        }
    }
}

pub struct ReportBuilder {
    items: Vec<ReportItem>,
}

impl ReportBuilder {
    pub fn append_item(mut self, item: ReportItem) -> Self {
        self.items.push(item);
        self
    }

    pub fn build(self) -> Report {
        Report { items: self.items }
    }
}

pub struct PcjMetadataRenderer;

impl PcjMetadataRenderer {
    pub fn render_one(&self, query_id: &str, metadata: &PcjMetadata) -> String {
        let builder = Report::builder()
            .append_item(ReportItem::with_value("Query ID", query_id))
            .append_item(ReportItem::with_value(
                "Cardinality",
                format_number(metadata.cardinality),
            ))
            .append_item(ReportItem::with_lines(
                "Export Variable Orders",
                metadata.var_orders.iter().map(ToString::to_string),
            ))
            .append_item(ReportItem::with_lines(
                "SPARQL",
                pretty_format_sparql(&metadata.sparql),
            ));
        builder.build().to_string()
    }

    pub fn render_many(&self, metadata: &BTreeMap<String, PcjMetadata>) -> String {
        let mut rendered = String::new();
        for (query_id, metadata) in metadata {
            rendered.push_str(&self.render_one(query_id, metadata));
            rendered.push('\n');
        }
        rendered
    }
}

pub struct QueryReportRenderer;

impl QueryReportRenderer {
    pub fn render(&self, query_report: &QueryReport) -> String {
        let metadata = &query_report.fluo_query;
        let mut builder = Report::builder();
        let query = &metadata.query_metadata;
        builder = builder
            .append_item(ReportItem::new("QUERY NODE"))
            .append_item(ReportItem::with_value("Node ID", &query.node_id))
            .append_item(ReportItem::with_value(
                "Variable Order",
                query.variable_order.to_string(),
            ))
            .append_item(ReportItem::with_lines(
                "SPARQL",
                pretty_format_sparql(&query.sparql),
            ))
            .append_item(ReportItem::with_value(
                "Child Node ID",
                &query.child_node_id,
            ))
            .append_item(ReportItem::with_value(
                "Count",
                query_report.count(&query.node_id).to_string(),
            ));
        for filter in metadata.filters.values() {
            builder = builder
                .append_item(ReportItem::new(""))
                .append_item(ReportItem::new("FILTER NODE"))
                .append_item(ReportItem::with_value("Node ID", &filter.node_id))
                .append_item(ReportItem::with_value(
                    "Variable Order",
                    filter.variable_order.to_string(),
                ))
                .append_item(ReportItem::with_lines(
                    "Original SPARQL",
                    pretty_format_sparql(&filter.original_sparql),
                ))
                .append_item(ReportItem::with_value(
                    "Filter Index",
                    filter.filter_index_within_sparql.to_string(),
                ))
                .append_item(ReportItem::with_value(
                    "Parent Node ID",
                    &filter.parent_node_id,
                ))
                .append_item(ReportItem::with_value(
                    "Child Node ID",
                    &filter.child_node_id,
                ))
                .append_item(ReportItem::with_value(
                    "Count",
                    query_report.count(&filter.node_id).to_string(),
                ));
        }
        for join in metadata.joins.values() {
            builder = builder
                .append_item(ReportItem::new(""))
                .append_item(ReportItem::new("JOIN NODE"))
                .append_item(ReportItem::with_value("Node ID", &join.node_id))
                .append_item(ReportItem::with_value(
                    "Variable Order",
                    join.variable_order.to_string(),
                ))
                .append_item(ReportItem::with_value(
                    "Join Type",
                    join.join_type.as_storage_name(),
                ))
                .append_item(ReportItem::with_value(
                    "Parent Node ID",
                    &join.parent_node_id,
                ))
                .append_item(ReportItem::with_value(
                    "Left Child Node ID",
                    &join.left_child_node_id,
                ))
                .append_item(ReportItem::with_value(
                    "Right Child Node ID",
                    &join.right_child_node_id,
                ))
                .append_item(ReportItem::with_value(
                    "Count",
                    query_report.count(&join.node_id).to_string(),
                ));
        }
        for sp in metadata.statement_patterns.values() {
            builder = builder
                .append_item(ReportItem::new(""))
                .append_item(ReportItem::new("STATEMENT PATTERN NODE"))
                .append_item(ReportItem::with_value("Node ID", &sp.node_id))
                .append_item(ReportItem::with_value(
                    "Variable Order",
                    sp.variable_order.to_string(),
                ))
                .append_item(ReportItem::with_value(
                    "Statement Pattern",
                    &sp.statement_pattern,
                ))
                .append_item(ReportItem::with_value("Parent Node ID", &sp.parent_node_id))
                .append_item(ReportItem::with_value(
                    "Count",
                    query_report.count(&sp.node_id).to_string(),
                ));
        }
        builder.build().to_string()
    }
}

impl QueryReport {
    pub fn count(&self, node_id: &str) -> u128 {
        self.counts.get(node_id).copied().unwrap_or(0)
    }
}

fn pretty_format_sparql(sparql: &str) -> Vec<String> {
    let Ok(vars) = parse_select_vars(sparql) else {
        return vec![sparql.to_string()];
    };
    let Ok(body) = sparql_body(sparql) else {
        return vec![sparql.to_string()];
    };
    let Ok((filters, no_filters)) = extract_filters(body) else {
        return vec![sparql.to_string()];
    };
    let mut lines = vec![format!(
        "select {}",
        vars.iter()
            .map(|v| format!("?{v}"))
            .collect::<Vec<_>>()
            .join(" ")
    )];
    lines.push("where {".to_string());
    for filter in filters {
        lines.push(format!("  {}.", render_filter(&filter)));
    }
    for pattern in parse_patterns(&no_filters).unwrap_or_default() {
        lines.push(format!("  {}.", render_pattern(&pattern)));
    }
    lines.push("}".to_string());
    lines
}

fn render_filter(filter: &FilterExpression) -> String {
    match filter {
        FilterExpression::Eq(name, value) if value.data_type() == Some(URI_TYPE) => {
            format!("FILTER(?{name} = <{}>)", value.data())
        }
        FilterExpression::Eq(name, value) => format!("FILTER(?{name} = \"{}\")", value.data()),
        FilterExpression::Lt(name, value) => format!("FILTER(?{name} < {value})"),
    }
}

fn render_pattern(pattern: &FluoStatementPattern) -> String {
    format!(
        "{} {} {}",
        render_term(&pattern.subject),
        render_term(&pattern.predicate),
        render_term(&pattern.object)
    )
}

fn render_term(term: &FluoTerm) -> String {
    match term {
        FluoTerm::Var(name) => format!("?{name}"),
        FluoTerm::Constant(value) if value.data_type() == Some(URI_TYPE) => {
            format!("<{}>", value.data())
        }
        FluoTerm::Constant(value) => format!("\"{}\"", value.data()),
    }
}

fn format_number(value: u64) -> String {
    let chars = value.to_string().chars().rev().collect::<Vec<_>>();
    let mut out = String::new();
    for (index, ch) in chars.iter().enumerate() {
        if index != 0 && index % 3 == 0 {
            out.push(',');
        }
        out.push(*ch);
    }
    out.chars().rev().collect()
}

#[cfg(test)]
#[path = "tests/fluo_pcj_tests.rs"]
mod tests;
