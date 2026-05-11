use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{RyaIri, RyaType, XSD_ANY_URI};
use crate::resolver::{DELIM_BYTE, deserialize, serialize_type};
use crate::storage::fjall::{CONF_QUERY_AUTH, FjallRdfConfiguration};

pub mod fluo;
pub mod indexing;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VariableOrder(Vec<String>);

impl VariableOrder {
    pub const DELIMITER: &'static str = ";";

    pub fn new(vars: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self(vars.into_iter().map(Into::into).collect())
    }

    pub fn parse(value: &str) -> Self {
        if value.is_empty() {
            Self(Vec::new())
        } else {
            Self(value.split(Self::DELIMITER).map(str::to_string).collect())
        }
    }

    pub fn as_slice(&self) -> &[String] {
        &self.0
    }

    pub fn shifted_orders(&self) -> BTreeSet<Self> {
        let mut orders = BTreeSet::new();
        let mut vars = self.0.clone();
        for _ in 0..vars.len() {
            orders.insert(Self(vars.clone()));
            let first = vars.remove(0);
            vars.push(first);
        }
        orders
    }
}

impl std::fmt::Display for VariableOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0.join(Self::DELIMITER))
    }
}

pub type BindingSet = BTreeMap<String, RyaType>;

const BINDING_STRING_DELIMITER: &str = ":::";
const VALUE_TYPE_DELIMITER: &str = "<<~>>";
pub const NULL_BINDING_STRING: &str = "\0";
pub const VISIBILITY_DELIMITER: char = '\u{1}';

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VisibilityBindingSet {
    pub bindings: BindingSet,
    pub visibility: String,
}

impl VisibilityBindingSet {
    pub fn new(bindings: BindingSet, visibility: impl Into<String>) -> Self {
        Self {
            bindings,
            visibility: visibility.into(),
        }
    }

    pub fn no_visibility(bindings: BindingSet) -> Self {
        Self::new(bindings, "")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedQuery {
    sparql: String,
    tuple_expr: ParsedTupleExpr,
}

impl ParsedQuery {
    pub fn new(sparql: impl Into<String>, tuple_expr: ParsedTupleExpr) -> Self {
        Self {
            sparql: sparql.into(),
            tuple_expr,
        }
    }

    pub fn parse_select_query_shape(sparql: impl Into<String>) -> Result<Self, String> {
        let sparql = sparql.into();
        let upper = sparql.to_ascii_uppercase();
        let select_pos = upper
            .find("SELECT")
            .ok_or_else(|| "Only SELECT query fixtures are supported".to_string())?;
        let where_pos = upper
            .find("WHERE")
            .ok_or_else(|| "SELECT query fixture is missing WHERE".to_string())?;
        let mut select_clause = sparql[select_pos + "SELECT".len()..where_pos].trim();
        let first_token = select_clause.split_whitespace().next();
        let is_distinct = first_token.is_some_and(|value| value.eq_ignore_ascii_case("DISTINCT"));
        if is_distinct {
            select_clause = select_clause["DISTINCT".len()..].trim();
        }

        let variables = select_clause
            .split_whitespace()
            .filter_map(|token| token.strip_prefix('?'))
            .map(|name| {
                name.trim_end_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
                    .to_string()
            })
            .filter(|name| !name.is_empty())
            .collect::<BTreeSet<_>>();
        if variables.is_empty() {
            return Err("SELECT query fixture does not project any variables".to_string());
        }

        let projection = ParsedTupleExpr::Projection {
            projection: ParsedProjection::new(variables),
            arg: Box::new(ParsedTupleExpr::StatementPattern),
        };
        let tuple_expr = if is_distinct {
            ParsedTupleExpr::Distinct(Box::new(projection))
        } else {
            projection
        };

        Ok(Self { sparql, tuple_expr })
    }

    pub fn sparql(&self) -> &str {
        &self.sparql
    }

    pub fn tuple_expr(&self) -> &ParsedTupleExpr {
        &self.tuple_expr
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParsedTupleExpr {
    Projection {
        projection: ParsedProjection,
        arg: Box<ParsedTupleExpr>,
    },
    Distinct(Box<ParsedTupleExpr>),
    Reduced(Box<ParsedTupleExpr>),
    Slice(Box<ParsedTupleExpr>),
    Filter(Box<ParsedTupleExpr>),
    Join(Box<ParsedTupleExpr>, Box<ParsedTupleExpr>),
    StatementPattern,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedProjection {
    variables: BTreeSet<String>,
}

impl ParsedProjection {
    pub fn new(variables: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            variables: variables.into_iter().map(Into::into).collect(),
        }
    }

    pub fn variables(&self) -> &BTreeSet<String> {
        &self.variables
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ParsedQueryUtil;

impl ParsedQueryUtil {
    pub fn find_projection<'a>(&self, query: &'a ParsedQuery) -> Option<&'a ParsedProjection> {
        find_projection_in_expr(query.tuple_expr())
    }
}

fn find_projection_in_expr(expr: &ParsedTupleExpr) -> Option<&ParsedProjection> {
    match expr {
        ParsedTupleExpr::Projection { projection, .. } => Some(projection),
        ParsedTupleExpr::Distinct(arg)
        | ParsedTupleExpr::Reduced(arg)
        | ParsedTupleExpr::Slice(arg)
        | ParsedTupleExpr::Filter(arg) => find_projection_in_expr(arg),
        ParsedTupleExpr::Join(left, right) => {
            find_projection_in_expr(left).or_else(|| find_projection_in_expr(right))
        }
        ParsedTupleExpr::StatementPattern => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PcjMetadata {
    pub sparql: String,
    pub cardinality: u64,
    pub var_orders: BTreeSet<VariableOrder>,
}

impl PcjMetadata {
    pub fn new(
        sparql: impl Into<String>,
        cardinality: u64,
        var_orders: impl IntoIterator<Item = VariableOrder>,
    ) -> Self {
        Self {
            sparql: sparql.into(),
            cardinality,
            var_orders: var_orders.into_iter().collect(),
        }
    }

    fn with_cardinality(&self, cardinality: u64) -> Self {
        Self {
            sparql: self.sparql.clone(),
            cardinality,
            var_orders: self.var_orders.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PcjCardinalityUpdateStrategy {
    ConditionalWriter,
    MockBatchWriter,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PcjCardinalityUpdate {
    pub strategy: PcjCardinalityUpdateStrategy,
    pub previous_cardinality: u64,
    pub delta: u64,
    pub new_cardinality: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PcjEntry {
    pub row: Vec<u8>,
    pub column_family: String,
    pub column_visibility: String,
    pub value: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PcjTable {
    metadata: PcjMetadata,
    entries: BTreeSet<PcjEntry>,
    cardinality_updates: Vec<PcjCardinalityUpdate>,
}

impl PcjTable {
    pub fn metadata(&self) -> &PcjMetadata {
        &self.metadata
    }

    pub fn entries(&self) -> &BTreeSet<PcjEntry> {
        &self.entries
    }

    pub fn cardinality_updates(&self) -> &[PcjCardinalityUpdate] {
        &self.cardinality_updates
    }

    pub fn results_for_order(&self, order: &VariableOrder) -> Result<BTreeSet<BindingSet>, String> {
        self.visible_results_for_order(order)
            .map(|rows| rows.into_iter().map(|row| row.bindings).collect())
    }

    pub fn visible_results_for_order(
        &self,
        order: &VariableOrder,
    ) -> Result<BTreeSet<VisibilityBindingSet>, String> {
        self.entries
            .iter()
            .filter(|entry| entry.column_family == order.to_string())
            .map(|entry| {
                Ok(VisibilityBindingSet::new(
                    deserialize_binding_set(&entry.row, order)?,
                    entry.column_visibility.clone(),
                ))
            })
            .collect()
    }

    pub fn results_for_order_with_auths(
        &self,
        order: &VariableOrder,
        auths: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<BTreeSet<BindingSet>, String> {
        let auths = auths
            .into_iter()
            .map(|auth| auth.as_ref().to_string())
            .collect::<BTreeSet<_>>();
        self.entries
            .iter()
            .filter(|entry| entry.column_family == order.to_string())
            .filter(|entry| visibility_allowed(&entry.column_visibility, &auths))
            .map(|entry| deserialize_binding_set(&entry.row, order))
            .collect()
    }
}

impl Ord for PcjEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.row
            .cmp(&other.row)
            .then_with(|| self.column_family.cmp(&other.column_family))
            .then_with(|| self.column_visibility.cmp(&other.column_visibility))
            .then_with(|| self.value.cmp(&other.value))
    }
}

impl PartialOrd for PcjEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub fn make_pcj_table_name(rya_prefix: &str, pcj_id: &str) -> String {
    format!("{}INDEX_{}", rya_prefix, pcj_id.replace('-', ""))
}

pub fn pcj_id_from_table_name(table_name: &str) -> Option<&str> {
    table_name.split_once("INDEX_").map(|(_, id)| id)
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryPcjTables {
    tables: BTreeMap<String, PcjTable>,
}

impl InMemoryPcjTables {
    pub fn create_pcj_table(
        &mut self,
        name: impl Into<String>,
        var_orders: impl IntoIterator<Item = VariableOrder>,
        sparql: impl Into<String>,
    ) {
        let name = name.into();
        self.tables.entry(name).or_insert_with(|| PcjTable {
            metadata: PcjMetadata::new(sparql, 0, var_orders),
            entries: BTreeSet::new(),
            cardinality_updates: Vec::new(),
        });
    }

    pub fn get_pcj_metadata(&self, name: &str) -> Result<PcjMetadata, String> {
        self.tables
            .get(name)
            .map(|table| table.metadata.clone())
            .ok_or_else(|| format!("PCJ table does not exist: {name}"))
    }

    pub fn table(&self, name: &str) -> Result<&PcjTable, String> {
        self.tables
            .get(name)
            .ok_or_else(|| format!("PCJ table does not exist: {name}"))
    }

    pub fn add_results(
        &mut self,
        name: &str,
        results: impl IntoIterator<Item = BindingSet>,
    ) -> Result<(), String> {
        self.add_visibility_results(
            name,
            results.into_iter().map(VisibilityBindingSet::no_visibility),
        )
    }

    pub fn add_visibility_results(
        &mut self,
        name: &str,
        results: impl IntoIterator<Item = VisibilityBindingSet>,
    ) -> Result<(), String> {
        self.add_visibility_results_with_strategy(
            name,
            results,
            PcjCardinalityUpdateStrategy::ConditionalWriter,
        )
    }

    pub fn add_visibility_results_with_strategy(
        &mut self,
        name: &str,
        results: impl IntoIterator<Item = VisibilityBindingSet>,
        strategy: PcjCardinalityUpdateStrategy,
    ) -> Result<(), String> {
        let table = self
            .tables
            .get_mut(name)
            .ok_or_else(|| format!("PCJ table does not exist: {name}"))?;
        let results = results.into_iter().collect::<Vec<_>>();
        for result in &results {
            for order in &table.metadata.var_orders {
                let row = serialize_binding_set(&result.bindings, order)?;
                table.entries.insert(PcjEntry {
                    row: row.clone(),
                    column_family: order.to_string(),
                    column_visibility: result.visibility.clone(),
                    value: serialize_full_binding_set(&result.bindings)?,
                });
            }
        }
        let previous_cardinality = table.metadata.cardinality;
        let delta = results.len() as u64;
        let new_cardinality = previous_cardinality + delta;
        table.metadata = table.metadata.with_cardinality(new_cardinality);
        table.cardinality_updates.push(PcjCardinalityUpdate {
            strategy,
            previous_cardinality,
            delta,
            new_cardinality,
        });
        Ok(())
    }

    pub fn purge_pcj_table(&mut self, name: &str) -> Result<(), String> {
        let table = self
            .tables
            .get_mut(name)
            .ok_or_else(|| format!("PCJ table does not exist: {name}"))?;
        table.entries.clear();
        table.metadata = table.metadata.with_cardinality(0);
        Ok(())
    }

    pub fn drop_pcj_table(&mut self, name: &str) -> bool {
        self.tables.remove(name).is_some()
    }

    pub fn list_pcj_tables(&self, rya_prefix: &str) -> Vec<String> {
        let prefix = format!("{rya_prefix}INDEX");
        self.tables
            .keys()
            .filter(|name| name.starts_with(&prefix))
            .cloned()
            .collect()
    }

    pub fn list_pcj_ids(&self, rya_prefix: &str) -> Vec<String> {
        self.list_pcj_tables(rya_prefix)
            .into_iter()
            .filter_map(|name| pcj_id_from_table_name(&name).map(str::to_string))
            .collect()
    }
}

pub fn serialize_binding_set(
    binding_set: &BindingSet,
    order: &VariableOrder,
) -> Result<Vec<u8>, String> {
    check_bindings_subset_of_var_order(binding_set, order)?;

    let mut row = Vec::new();
    for variable in order.as_slice() {
        if let Some(value) = binding_set.get(variable) {
            let (data, suffix) = serialize_type(value)?;
            row.extend(data);
            row.extend(suffix);
        }
        row.push(DELIM_BYTE);
    }
    Ok(row)
}

pub fn deserialize_binding_set(bytes: &[u8], order: &VariableOrder) -> Result<BindingSet, String> {
    let values = split_by_delimiter(bytes);
    if values.len() != order.as_slice().len() {
        return Err(format!(
            "Serialized binding count {} does not match variable order {}",
            values.len(),
            order
        ));
    }

    let mut binding_set = BindingSet::new();
    for (variable, value_bytes) in order.as_slice().iter().zip(values) {
        if !value_bytes.is_empty() {
            binding_set.insert(variable.clone(), deserialize(value_bytes)?);
        }
    }
    Ok(binding_set)
}

pub fn binding_set_to_string(
    binding_set: &BindingSet,
    order: &VariableOrder,
) -> Result<String, String> {
    check_bindings_subset_of_var_order(binding_set, order)?;

    let mut values = Vec::with_capacity(order.as_slice().len());
    for variable in order.as_slice() {
        if let Some(value) = binding_set.get(variable) {
            values.push(format!(
                "{}{}{}",
                value.data(),
                VALUE_TYPE_DELIMITER,
                value.data_type().unwrap_or(crate::domain::XSD_STRING)
            ));
        } else {
            values.push(NULL_BINDING_STRING.to_string());
        }
    }

    Ok(values.join(BINDING_STRING_DELIMITER))
}

pub fn binding_set_from_string(
    binding_set_string: &str,
    order: &VariableOrder,
) -> Result<BindingSet, String> {
    let binding_strings = if binding_set_string.is_empty() {
        Vec::new()
    } else {
        binding_set_string
            .split(BINDING_STRING_DELIMITER)
            .collect::<Vec<_>>()
    };
    if binding_strings.len() != order.as_slice().len() {
        return Err(
            "The number of Bindings must match the length of the VariableOrder.".to_string(),
        );
    }

    let mut binding_set = BindingSet::new();
    for (name, value_string) in order.as_slice().iter().zip(binding_strings) {
        if value_string != NULL_BINDING_STRING {
            binding_set.insert(name.clone(), value_from_binding_string(value_string)?);
        }
    }
    Ok(binding_set)
}

pub fn visibility_binding_set_to_string(
    binding_set: &VisibilityBindingSet,
    order: &VariableOrder,
) -> Result<String, String> {
    let mut encoded = binding_set_to_string(&binding_set.bindings, order)?;
    if !binding_set.visibility.is_empty() {
        encoded.push(VISIBILITY_DELIMITER);
        encoded.push_str(&binding_set.visibility);
    }
    Ok(encoded)
}

pub fn visibility_binding_set_from_string(
    binding_set_string: &str,
    order: &VariableOrder,
) -> Result<VisibilityBindingSet, String> {
    let (bindings, visibility) = binding_set_string
        .split_once(VISIBILITY_DELIMITER)
        .map(|(bindings, visibility)| (bindings, visibility.to_string()))
        .unwrap_or((binding_set_string, String::new()));
    Ok(VisibilityBindingSet::new(
        binding_set_from_string(bindings, order)?,
        visibility,
    ))
}

fn check_bindings_subset_of_var_order(
    binding_set: &BindingSet,
    order: &VariableOrder,
) -> Result<(), String> {
    let order_vars = order.as_slice().iter().collect::<BTreeSet<_>>();
    if binding_set.keys().all(|name| order_vars.contains(name)) {
        Ok(())
    } else {
        Err(
            "The BindingSet contains a Binding whose name is not part of the VariableOrder."
                .to_string(),
        )
    }
}

fn value_from_binding_string(value_string: &str) -> Result<RyaType, String> {
    let value_and_type = value_string.split(VALUE_TYPE_DELIMITER).collect::<Vec<_>>();
    if value_and_type.len() != 2 {
        return Err("Array must contain data and type info!".to_string());
    }

    let data = value_and_type[0];
    let data_type = value_and_type[1];
    if data_type == XSD_ANY_URI {
        Ok(RyaIri::new(data)?.into_type())
    } else {
        Ok(RyaType::custom(data_type, data))
    }
}

fn serialize_full_binding_set(binding_set: &BindingSet) -> Result<Vec<u8>, String> {
    let order = VariableOrder::new(binding_set.keys().cloned());
    serialize_binding_set(binding_set, &order)
}

fn split_by_delimiter(bytes: &[u8]) -> Vec<&[u8]> {
    let mut values = Vec::new();
    let mut start = 0;
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == DELIM_BYTE {
            values.push(&bytes[start..index]);
            start = index + 1;
        }
    }
    values
}

pub fn visibility_allowed(visibility: &str, auths: &BTreeSet<String>) -> bool {
    let visibility = visibility.trim();
    if visibility.is_empty() {
        return true;
    }
    if auths.is_empty() {
        return false;
    }
    visibility.split('|').any(|disjunct| {
        disjunct
            .split('&')
            .all(|term| auths.contains(clean_visibility_term(term)))
    })
}

fn clean_visibility_term(term: &str) -> &str {
    term.trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim()
}

#[derive(Clone, Debug)]
pub struct FjallIndexSet<'a> {
    table: &'a PcjTable,
    auths: BTreeSet<String>,
    projection_variables: Option<BTreeSet<String>>,
    query_to_table_vars: BTreeMap<String, String>,
    constant_constraints: BindingSet,
    unassured_variables: BTreeSet<String>,
}

impl<'a> FjallIndexSet<'a> {
    pub fn new(table: &'a PcjTable) -> Self {
        Self {
            table,
            auths: BTreeSet::new(),
            projection_variables: None,
            query_to_table_vars: BTreeMap::new(),
            constant_constraints: BindingSet::new(),
            unassured_variables: BTreeSet::new(),
        }
    }

    pub fn for_parsed_query(table: &'a PcjTable, query: &ParsedQuery) -> Result<Self, String> {
        let projection = ParsedQueryUtil.find_projection(query).ok_or_else(|| {
            format!(
                "SPARQL query '{}' does not contain a Projection.",
                query.sparql()
            )
        })?;

        Ok(Self::new(table).with_projection_variables(projection.variables().iter().cloned()))
    }

    pub fn from_config(table: &'a PcjTable, conf: &FjallRdfConfiguration) -> Self {
        Self::new(table).with_authorizations(authorizations_from_config(conf))
    }

    pub fn for_parsed_query_from_config(
        table: &'a PcjTable,
        query: &ParsedQuery,
        conf: &FjallRdfConfiguration,
    ) -> Result<Self, String> {
        Ok(Self::for_parsed_query(table, query)?
            .with_authorizations(authorizations_from_config(conf)))
    }

    pub fn with_authorizations(
        mut self,
        auths: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.auths = auths
            .into_iter()
            .map(Into::into)
            .filter(|auth| !auth.is_empty())
            .collect();
        self
    }

    pub fn with_projection_variables(
        mut self,
        variables: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.projection_variables = Some(variables.into_iter().map(Into::into).collect());
        self
    }

    pub fn with_table_var_map(
        mut self,
        query_to_table_vars: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        self.query_to_table_vars = query_to_table_vars
            .into_iter()
            .map(|(query, table)| (query.into(), table.into()))
            .collect();
        self
    }

    pub fn with_constant_constraints(
        mut self,
        constraints: impl IntoIterator<Item = (impl Into<String>, RyaType)>,
    ) -> Self {
        self.constant_constraints = constraints
            .into_iter()
            .map(|(name, value)| (name.into(), value))
            .collect();
        self
    }

    pub fn with_unassured_variables(
        mut self,
        variables: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.unassured_variables = variables.into_iter().map(Into::into).collect();
        self
    }

    pub fn cardinality(&self) -> u64 {
        self.table.metadata.cardinality
    }

    pub fn evaluate_one(&self, binding_set: &BindingSet) -> Result<BTreeSet<BindingSet>, String> {
        self.evaluate(std::iter::once(binding_set.clone()))
    }

    pub fn evaluate(
        &self,
        binding_sets: impl IntoIterator<Item = BindingSet>,
    ) -> Result<BTreeSet<BindingSet>, String> {
        let binding_sets = binding_sets.into_iter().collect::<Vec<_>>();
        if binding_sets.is_empty() {
            return Ok(BTreeSet::new());
        }

        let order = self
            .best_order()
            .ok_or_else(|| "PCJ table has no variable orders".to_string())?;
        let pcj_rows = self
            .table
            .results_for_order_with_auths(order, &self.auths)?;
        let mut out = BTreeSet::new();

        for input in binding_sets {
            for pcj_row in &pcj_rows {
                if !self.matches_input(&input, pcj_row) || !self.matches_constants(pcj_row) {
                    continue;
                }
                out.insert(self.join_binding_sets(&input, pcj_row));
            }
        }

        Ok(out)
    }

    fn best_order(&self) -> Option<&VariableOrder> {
        self.table.metadata.var_orders.iter().next()
    }

    fn matches_constants(&self, pcj_row: &BindingSet) -> bool {
        self.constant_constraints.iter().all(|(name, value)| {
            pcj_row
                .get(self.table_var_name(name))
                .is_some_and(|v| v == value)
        })
    }

    fn matches_input(&self, input: &BindingSet, pcj_row: &BindingSet) -> bool {
        input.iter().all(|(query_name, query_value)| {
            if self.unassured_variables.contains(query_name) {
                return true;
            }
            let table_name = self.table_var_name(query_name);
            pcj_row
                .get(table_name)
                .is_none_or(|pcj_value| pcj_value == query_value)
        })
    }

    fn join_binding_sets(&self, input: &BindingSet, pcj_row: &BindingSet) -> BindingSet {
        let mut joined = input.clone();
        for (table_name, value) in pcj_row {
            let query_name = self.query_var_name(table_name);
            if !query_name.starts_with("-const-")
                && self
                    .projection_variables
                    .as_ref()
                    .is_none_or(|projection| projection.contains(query_name))
            {
                joined
                    .entry(query_name.to_string())
                    .or_insert_with(|| value.clone());
            }
        }
        joined
    }

    fn table_var_name<'b>(&'b self, query_name: &'b str) -> &'b str {
        self.query_to_table_vars
            .get(query_name)
            .map(String::as_str)
            .unwrap_or(query_name)
    }

    fn query_var_name<'b>(&'b self, table_name: &'b str) -> &'b str {
        self.query_to_table_vars
            .iter()
            .find_map(|(query, table)| (table == table_name).then_some(query.as_str()))
            .unwrap_or(table_name)
    }
}

pub fn authorizations_from_config(conf: &FjallRdfConfiguration) -> BTreeSet<String> {
    conf.get(CONF_QUERY_AUTH)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|auth| !auth.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn natural_join(
    left: &BTreeSet<BindingSet>,
    right: &BTreeSet<BindingSet>,
) -> BTreeSet<BindingSet> {
    let mut out = BTreeSet::new();
    for left_row in left {
        for right_row in right {
            if let Some(joined) = join_compatible(left_row, right_row) {
                out.insert(joined);
            }
        }
    }
    out
}

pub fn union_binding_sets(
    left: &BTreeSet<BindingSet>,
    right: &BTreeSet<BindingSet>,
) -> BTreeSet<BindingSet> {
    left.union(right).cloned().collect()
}

pub fn left_join(
    left: &BTreeSet<BindingSet>,
    right: &BTreeSet<BindingSet>,
) -> BTreeSet<BindingSet> {
    let mut out = BTreeSet::new();
    for left_row in left {
        let mut matched = false;
        for right_row in right {
            if let Some(joined) = join_compatible(left_row, right_row) {
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

pub fn filter_eq(
    rows: &BTreeSet<BindingSet>,
    variable: &str,
    value: &RyaType,
) -> BTreeSet<BindingSet> {
    rows.iter()
        .filter(|row| {
            row.get(variable)
                .is_some_and(|candidate| candidate == value)
        })
        .cloned()
        .collect()
}

pub fn natural_join_bag(left: &[BindingSet], right: &[BindingSet]) -> Vec<BindingSet> {
    let mut out = Vec::new();
    for left_row in left {
        for right_row in right {
            if let Some(joined) = join_compatible(left_row, right_row) {
                out.push(joined);
            }
        }
    }
    out
}

pub fn union_binding_bag(left: &[BindingSet], right: &[BindingSet]) -> Vec<BindingSet> {
    left.iter().chain(right).cloned().collect()
}

pub fn left_join_bag(left: &[BindingSet], right: &[BindingSet]) -> Vec<BindingSet> {
    let mut out = Vec::new();
    for left_row in left {
        let mut matched = false;
        for right_row in right {
            if let Some(joined) = join_compatible(left_row, right_row) {
                out.push(joined);
                matched = true;
            }
        }
        if !matched {
            out.push(left_row.clone());
        }
    }
    out
}

pub fn filter_eq_bag(rows: &[BindingSet], variable: &str, value: &RyaType) -> Vec<BindingSet> {
    rows.iter()
        .filter(|row| {
            row.get(variable)
                .is_some_and(|candidate| candidate == value)
        })
        .cloned()
        .collect()
}

fn join_compatible(left: &BindingSet, right: &BindingSet) -> Option<BindingSet> {
    for (name, value) in left {
        if right
            .get(name)
            .is_some_and(|right_value| right_value != value)
        {
            return None;
        }
    }
    let mut joined = left.clone();
    for (name, value) in right {
        joined.entry(name.clone()).or_insert_with(|| value.clone());
    }
    Some(joined)
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum PatternTerm {
    Var(String),
    Constant(RyaType),
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct PcjStatementPattern {
    pub subject: PatternTerm,
    pub predicate: PatternTerm,
    pub object: PatternTerm,
}

impl PcjStatementPattern {
    pub fn new(subject: PatternTerm, predicate: PatternTerm, object: PatternTerm) -> Self {
        Self {
            subject,
            predicate,
            object,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalTupleSetPlan {
    pub patterns: BTreeSet<PcjStatementPattern>,
}

pub fn choose_precomputed_join_sets(
    query: &BTreeSet<PcjStatementPattern>,
    indexes: &[ExternalTupleSetPlan],
) -> Vec<ExternalTupleSetPlan> {
    let mut remaining = query.clone();
    let mut selected = Vec::new();
    for index in indexes {
        if index.patterns.iter().all(|pattern| {
            remaining.contains(pattern) || matches_pattern_by_renaming(query, pattern)
        }) {
            for pattern in &index.patterns {
                remaining.remove(pattern);
            }
            selected.push(index.clone());
        }
    }
    selected
}

fn matches_pattern_by_renaming(
    query: &BTreeSet<PcjStatementPattern>,
    pattern: &PcjStatementPattern,
) -> bool {
    query.iter().any(|candidate| {
        let mut mapping = BTreeMap::new();
        terms_match(&candidate.subject, &pattern.subject, &mut mapping)
            && terms_match(&candidate.predicate, &pattern.predicate, &mut mapping)
            && terms_match(&candidate.object, &pattern.object, &mut mapping)
    })
}

fn terms_match(
    query_term: &PatternTerm,
    index_term: &PatternTerm,
    mapping: &mut BTreeMap<String, PatternTerm>,
) -> bool {
    match (query_term, index_term) {
        (PatternTerm::Constant(a), PatternTerm::Constant(b)) => a == b,
        (_, PatternTerm::Var(name)) => match mapping.get(name) {
            Some(mapped) => mapped == query_term,
            None => {
                mapping.insert(name.clone(), query_term.clone());
                true
            }
        },
        _ => false,
    }
}

pub fn iri(value: &str) -> RyaType {
    RyaIri::new(value).expect("test IRI").into_type()
}

#[cfg(test)]
#[path = "../tests/pcj_tests.rs"]
mod tests;
