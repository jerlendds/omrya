use std::collections::HashMap;

use crate::domain::{RyaIri, RyaStatement, RyaType};
use crate::indexing::{
    EntityCentricIndex, EntityIndexMutation, InMemoryFreeTextIndexer, InMemoryGeoIndexer,
    InMemorySecondaryIndexer, InMemoryTemporalIndexer,
};
use crate::resolver::triple::{TableLayout, TripleContext, TripleRow};

pub const JOB_NAME_PROP: &str = "mapred.job.name";
pub const AC_USERNAME_PROP: &str = "ac.username";
pub const AC_PWD_PROP: &str = "ac.pwd";
pub const AC_ZK_PROP: &str = "ac.zk";
pub const AC_INSTANCE_PROP: &str = "ac.instance";
pub const AC_TTL_PROP: &str = "ac.ttl";
pub const AC_TABLE_PROP: &str = "ac.table";
pub const AC_AUTH_PROP: &str = "ac.auth";
pub const AC_CV_PROP: &str = "ac.cv";
pub const AC_MOCK_PROP: &str = "ac.mock";
pub const AC_HDFS_INPUT_PROP: &str = "ac.hdfsinput";
pub const HADOOP_IO_SORT_MB: &str = "io.sort.mb";
pub const FORMAT_PROP: &str = "rdf.format";
pub const INPUT_PATH: &str = "input";
pub const NAMED_GRAPH_PROP: &str = "rdf.graph";
pub const TABLE_LAYOUT_PROP: &str = "rdf.tablelayout";
pub const TABLE_PREFIX_PROPERTY: &str = "rdf.tablePrefix";
pub const TBL_PRFX_DEF: &str = "rya_";
pub const TBL_SPO_SUFFIX: &str = "spo";
pub const TBL_PO_SUFFIX: &str = "po";
pub const TBL_OSP_SUFFIX: &str = "osp";

pub const MRUNIT_GROUP_ID: &str = "org.apache.mrunit";
pub const MRUNIT_ARTIFACT_ID: &str = "mrunit";
pub const MRUNIT_CLASSIFIER: &str = "hadoop2";
pub const MRUNIT_VERSION: &str = "1.1.0";
pub const MRUNIT_SCOPE: &str = "test";
pub const RYA_MAPREDUCE_PACKAGE: &str = "mvm.rya.fjall.mr";
pub const LEGACY_MR_UTILS_PACKAGE: &str = "mvm.rya.fjall.mr.utils";
pub const RYA_INPUT_FORMAT_CLASS: &str = "mvm.rya.fjall.mr.RyaInputFormat";
pub const RYA_OUTPUT_FORMAT_CLASS: &str = "mvm.rya.fjall.mr.RyaOutputFormat";
pub const RDF_FILE_INPUT_FORMAT_CLASS: &str = "mvm.rya.fjall.mr.RdfFileInputFormat";
pub const RDF_FILE_INPUT_TOOL_CLASS: &str = "mvm.rya.fjall.mr.tools.RdfFileInputTool";
pub const FJALL_RDF_COUNT_TOOL_CLASS: &str = "mvm.rya.fjall.mr.tools.FjallRdfCountTool";
pub const UPGRADE_322_TOOL_CLASS: &str = "mvm.rya.fjall.mr.tools.Upgrade322Tool";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MrConfiguration {
    values: HashMap<String, String>,
}

impl MrConfiguration {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn set_table_layout(&mut self, layout: TableLayout) {
        self.set(TABLE_LAYOUT_PROP, layout_name(layout));
    }

    pub fn table_layout_or_osp(&self) -> Result<TableLayout, String> {
        match self.get(TABLE_LAYOUT_PROP).unwrap_or("OSP") {
            "SPO" => Ok(TableLayout::Spo),
            "PO" => Ok(TableLayout::Po),
            "OSP" => Ok(TableLayout::Osp),
            other => Err(format!("Unknown Rya table layout: {other}")),
        }
    }

    pub fn set_table_prefix(&mut self, prefix: impl Into<String>) {
        self.set(TABLE_PREFIX_PROPERTY, prefix);
    }

    pub fn table_prefix_or_default(&self) -> &str {
        self.get(TABLE_PREFIX_PROPERTY).unwrap_or(TBL_PRFX_DEF)
    }

    pub fn set_bool(&mut self, key: impl Into<String>, value: bool) {
        self.set(key, if value { "true" } else { "false" });
    }

    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.get(key)
            .and_then(|value| value.parse().ok())
            .unwrap_or(default)
    }

    pub fn set_rdf_format(&mut self, format: RdfFormat) {
        self.set(FORMAT_PROP, format.name());
    }

    pub fn rdf_format(&self) -> Option<RdfFormat> {
        self.get(FORMAT_PROP).and_then(RdfFormat::from_name)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RdfFormat {
    NTriples,
    NQuads,
    Trig,
    RdfXml,
}

impl RdfFormat {
    pub fn name(self) -> &'static str {
        match self {
            Self::NTriples => "N-Triples",
            Self::NQuads => "N-Quads",
            Self::Trig => "TriG",
            Self::RdfXml => "RDF/XML",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "N-Triples" | "NTRIPLES" | "NTRIPLES_FORMAT" => Some(Self::NTriples),
            "N-Quads" | "NQUADS" => Some(Self::NQuads),
            "TriG" | "TRIG" => Some(Self::Trig),
            "RDF/XML" | "RDFXML" => Some(Self::RdfXml),
            _ => None,
        }
    }

    pub fn is_line_splittable(self) -> bool {
        matches!(self, Self::NTriples | Self::NQuads)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RdfFileInputFormat {
    format: RdfFormat,
}

impl RdfFileInputFormat {
    pub fn new(format: RdfFormat) -> Self {
        Self { format }
    }

    pub fn from_config(conf: &MrConfiguration, default_format: RdfFormat) -> Self {
        Self {
            format: conf.rdf_format().unwrap_or(default_format),
        }
    }

    pub fn is_splitable(&self) -> bool {
        self.format.is_line_splittable()
    }

    pub fn read_str(&self, input: &str) -> Result<Vec<RyaStatementWritable>, String> {
        let statements = match self.format {
            RdfFormat::NTriples => parse_ntriples(input)?,
            RdfFormat::Trig => parse_trig(input)?,
            RdfFormat::NQuads => parse_nquads(input)?,
            RdfFormat::RdfXml => {
                return Err("RDF/XML parsing is not available in this fixture".to_string());
            }
        };
        let context = TripleContext::new(false);
        Ok(statements
            .into_iter()
            .map(|statement| RyaStatementWritable::with_statement(statement, context))
            .collect())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RyaOutputFormatConfig {
    pub table_prefix: String,
    pub default_visibility: Option<Vec<u8>>,
    pub default_context: Option<RyaIri>,
    pub core_tables_enabled: bool,
    pub free_text_enabled: bool,
    pub geo_enabled: bool,
    pub temporal_enabled: bool,
    pub entity_enabled: bool,
}

impl Default for RyaOutputFormatConfig {
    fn default() -> Self {
        Self {
            table_prefix: TBL_PRFX_DEF.to_string(),
            default_visibility: None,
            default_context: None,
            core_tables_enabled: true,
            free_text_enabled: false,
            geo_enabled: false,
            temporal_enabled: false,
            entity_enabled: false,
        }
    }
}

impl RyaOutputFormatConfig {
    pub fn from_mr_config(conf: &MrConfiguration) -> Result<Self, String> {
        Ok(Self {
            table_prefix: conf.table_prefix_or_default().to_string(),
            default_visibility: conf.get(AC_CV_PROP).map(|value| value.as_bytes().to_vec()),
            default_context: conf.get(NAMED_GRAPH_PROP).map(RyaIri::new).transpose()?,
            core_tables_enabled: conf.get_bool("RyaOutputFormat.coretables.enable", true),
            free_text_enabled: conf.get_bool("RyaOutputFormat.freetext.enable", false),
            geo_enabled: conf.get_bool("RyaOutputFormat.geo.enable", false),
            temporal_enabled: conf.get_bool("RyaOutputFormat.temporal.enable", false),
            entity_enabled: conf.get_bool("RyaOutputFormat.entity.enable", false),
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct RyaOutputSink {
    pub core_mutations: Vec<(String, Mutation)>,
    pub free_text: InMemoryFreeTextIndexer,
    pub geo: InMemoryGeoIndexer,
    pub temporal: InMemoryTemporalIndexer,
    pub entity_mutations: Vec<EntityIndexMutation>,
}

#[derive(Clone, Debug)]
pub struct RyaOutputRecordWriter {
    config: RyaOutputFormatConfig,
    buffer: Vec<RyaStatement>,
    max_buffered_statements: usize,
    sink: RyaOutputSink,
}

impl RyaOutputRecordWriter {
    pub fn new(config: RyaOutputFormatConfig) -> Self {
        Self {
            config,
            buffer: Vec::new(),
            max_buffered_statements: 1024,
            sink: RyaOutputSink::default(),
        }
    }

    pub fn with_max_buffered_statements(mut self, max: usize) -> Self {
        self.max_buffered_statements = max.max(1);
        self
    }

    pub fn write(&mut self, value: &RyaStatementWritable) -> Result<(), String> {
        let mut statement = value
            .rya_statement()
            .ok_or_else(|| "Rya Statement is null".to_string())?
            .clone();
        if statement.column_visibility.is_none() {
            statement.column_visibility = self.config.default_visibility.clone();
        }
        if statement.context.is_none() {
            statement.context = self.config.default_context.clone();
        }
        self.buffer.push(statement);
        if self.buffer.len() >= self.max_buffered_statements {
            self.flush()?;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), String> {
        for statement in self.buffer.drain(..) {
            if self.config.core_tables_enabled {
                let mut conf = MrConfiguration::new();
                conf.set_table_prefix(self.config.table_prefix.clone());
                self.sink
                    .core_mutations
                    .extend(emit_table_mutations(&conf, &statement)?);
            }
            if self.config.free_text_enabled {
                self.sink.free_text.store_statement(&statement);
            }
            if self.config.geo_enabled {
                self.sink.geo.store_statement(&statement);
            }
            if self.config.temporal_enabled {
                self.sink.temporal.store_statement(&statement);
            }
            if self.config.entity_enabled {
                self.sink
                    .entity_mutations
                    .extend(EntityCentricIndex::create_mutations(&statement)?);
            }
        }
        Ok(())
    }

    pub fn close(mut self) -> Result<RyaOutputSink, String> {
        self.flush()?;
        Ok(self.sink)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Mutation {
    pub row: Vec<u8>,
    pub column_family: Vec<u8>,
    pub column_qualifier: Vec<u8>,
    pub column_visibility: Option<Vec<u8>>,
    pub value: Vec<u8>,
    pub timestamp: u64,
    pub delete: bool,
}

impl Mutation {
    fn from_triple_row(row: TripleRow) -> Self {
        Self {
            row: row.row,
            column_family: row.column_family,
            column_qualifier: row.column_qualifier,
            column_visibility: row.column_visibility,
            value: row.value.unwrap_or_default(),
            timestamp: row.timestamp,
            delete: false,
        }
    }

    fn delete_from_triple_row(row: TripleRow) -> Self {
        Self {
            row: row.row,
            column_family: row.column_family,
            column_qualifier: row.column_qualifier,
            column_visibility: row.column_visibility,
            value: Vec::new(),
            timestamp: row.timestamp,
            delete: true,
        }
    }

    pub fn as_triple_row(&self) -> TripleRow {
        TripleRow {
            row: self.row.clone(),
            column_family: self.column_family.clone(),
            column_qualifier: self.column_qualifier.clone(),
            column_visibility: self.column_visibility.clone(),
            value: Some(self.value.clone()),
            timestamp: self.timestamp,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RyaTableMutationsFactory {
    context: TripleContext,
}

impl RyaTableMutationsFactory {
    pub fn new(context: TripleContext) -> Self {
        Self { context }
    }

    pub fn serialize(
        &self,
        statement: &RyaStatement,
    ) -> Result<HashMap<TableLayout, Vec<Mutation>>, String> {
        let mut mutations = HashMap::new();
        for (layout, row) in self.context.serialize_triple(statement)? {
            mutations.insert(layout, vec![Mutation::from_triple_row(row)]);
        }
        Ok(mutations)
    }

    pub fn serialize_delete(
        &self,
        statement: &RyaStatement,
    ) -> Result<HashMap<TableLayout, Vec<Mutation>>, String> {
        let mut mutations = HashMap::new();
        for (layout, row) in self.context.serialize_triple(statement)? {
            mutations.insert(layout, vec![Mutation::delete_from_triple_row(row)]);
        }
        Ok(mutations)
    }
}

impl Default for RyaTableMutationsFactory {
    fn default() -> Self {
        Self::new(TripleContext::new(false))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RyaStatementWritable {
    statement: Option<RyaStatement>,
    context: TripleContext,
}

impl RyaStatementWritable {
    pub fn new() -> Self {
        Self::with_context(TripleContext::new(false))
    }

    pub fn with_context(context: TripleContext) -> Self {
        Self {
            statement: None,
            context,
        }
    }

    pub fn with_statement(statement: RyaStatement, context: TripleContext) -> Self {
        Self {
            statement: Some(statement),
            context,
        }
    }

    pub fn rya_statement(&self) -> Option<&RyaStatement> {
        self.statement.as_ref()
    }

    pub fn set_rya_statement(&mut self, statement: RyaStatement) {
        self.statement = Some(statement);
    }

    pub fn java_compare_to(&self, other: &Self) -> i32 {
        match (self.rya_statement(), other.rya_statement()) {
            (Some(left), Some(right)) if left == right => 0,
            _ => -1,
        }
    }

    pub fn write_to_vec(&self) -> Result<Vec<u8>, String> {
        let statement = self
            .statement
            .as_ref()
            .ok_or_else(|| "Rya Statement is null".to_string())?;
        let rows = self.context.serialize_triple(statement)?;
        let row = rows
            .get(&TableLayout::Spo)
            .ok_or_else(|| "Missing SPO triple row".to_string())?;

        let mut bytes = Vec::new();
        write_field(&mut bytes, Some(&row.row));
        write_field(&mut bytes, Some(&row.column_family));
        write_field(&mut bytes, Some(&row.column_qualifier));
        write_field(&mut bytes, statement.column_visibility.as_deref());
        write_field(&mut bytes, statement.value.as_deref());
        bytes.push(1);
        bytes.extend_from_slice(&statement.timestamp.to_be_bytes());
        Ok(bytes)
    }

    pub fn read_from_slice(&mut self, bytes: &[u8]) -> Result<(), String> {
        let mut cursor = 0;
        let row = read_field(bytes, &mut cursor)?;
        let column_family = read_field(bytes, &mut cursor)?;
        let column_qualifier = read_field(bytes, &mut cursor)?;
        let column_visibility = read_field(bytes, &mut cursor)?;
        let value = read_field(bytes, &mut cursor)?;
        let has_timestamp = read_bool(bytes, &mut cursor)?;
        let timestamp = if has_timestamp {
            read_u64(bytes, &mut cursor)?
        } else {
            0
        };
        if cursor != bytes.len() {
            return Err("Trailing bytes after RyaStatementWritable".to_string());
        }

        let triple_row = TripleRow {
            row: row.ok_or_else(|| "Writable is missing triple row".to_string())?,
            column_family: column_family
                .ok_or_else(|| "Writable is missing column family".to_string())?,
            column_qualifier: column_qualifier
                .ok_or_else(|| "Writable is missing column qualifier".to_string())?,
            column_visibility,
            value,
            timestamp,
        };
        self.statement = Some(
            self.context
                .deserialize_triple(TableLayout::Spo, &triple_row)?,
        );
        Ok(())
    }
}

impl Default for RyaStatementWritable {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FjallKeyValue {
    pub key: FjallKey,
    pub value: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FjallKey {
    pub row: Vec<u8>,
    pub column_family: Vec<u8>,
    pub column_qualifier: Vec<u8>,
    pub column_visibility: Option<Vec<u8>>,
    pub timestamp: u64,
}

impl FjallKeyValue {
    pub fn from_mutation(mutation: &Mutation) -> Self {
        Self {
            key: FjallKey {
                row: mutation.row.clone(),
                column_family: mutation.column_family.clone(),
                column_qualifier: mutation.column_qualifier.clone(),
                column_visibility: mutation.column_visibility.clone(),
                timestamp: mutation.timestamp,
            },
            value: mutation.value.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RyaStatementInputFormat {
    context: TripleContext,
}

impl RyaStatementInputFormat {
    pub fn new(context: TripleContext) -> Self {
        Self { context }
    }

    pub fn create_record_reader(
        &self,
        conf: &MrConfiguration,
        entries: Vec<FjallKeyValue>,
    ) -> Result<RyaStatementRecordReader, String> {
        Ok(RyaStatementRecordReader {
            entries,
            index: 0,
            table_layout: conf.table_layout_or_osp()?,
            context: self.context,
            current_key: None,
            current_value: None,
        })
    }
}

#[derive(Clone, Debug)]
pub struct RyaStatementRecordReader {
    entries: Vec<FjallKeyValue>,
    index: usize,
    table_layout: TableLayout,
    context: TripleContext,
    current_key: Option<Vec<u8>>,
    current_value: Option<RyaStatementWritable>,
}

impl RyaStatementRecordReader {
    pub fn next_key_value(&mut self) -> Result<bool, String> {
        let Some(entry) = self.entries.get(self.index).cloned() else {
            return Ok(false);
        };
        self.index += 1;
        self.current_key = Some(entry.key.row.clone());

        let triple_row = TripleRow {
            row: entry.key.row,
            column_family: entry.key.column_family,
            column_qualifier: entry.key.column_qualifier,
            column_visibility: entry.key.column_visibility,
            value: Some(entry.value),
            timestamp: entry.key.timestamp,
        };
        let statement = self
            .context
            .deserialize_triple(self.table_layout, &triple_row)?;
        self.current_value = Some(RyaStatementWritable::with_statement(
            statement,
            self.context,
        ));
        Ok(true)
    }

    pub fn current_key(&self) -> Option<&[u8]> {
        self.current_key.as_deref()
    }

    pub fn current_value(&self) -> Option<&RyaStatementWritable> {
        self.current_value.as_ref()
    }
}

pub fn map_statement(
    conf: &MrConfiguration,
    value: &RyaStatementWritable,
) -> Result<Vec<(String, Mutation)>, String> {
    let statement = value
        .rya_statement()
        .ok_or_else(|| "Rya Statement is null".to_string())?;
    emit_table_mutations(conf, statement)
}

pub fn reduce_statements(
    conf: &MrConfiguration,
    values: &[RyaStatementWritable],
) -> Result<Vec<(String, Mutation)>, String> {
    let mut outputs = Vec::new();
    for value in values {
        let statement = value
            .rya_statement()
            .ok_or_else(|| "Rya Statement is null".to_string())?;
        outputs.extend(emit_table_mutations(conf, statement)?);
    }
    Ok(outputs)
}

pub fn table_name(prefix: &str, layout: TableLayout) -> String {
    let suffix = match layout {
        TableLayout::Spo => TBL_SPO_SUFFIX,
        TableLayout::Po => TBL_PO_SUFFIX,
        TableLayout::Osp => TBL_OSP_SUFFIX,
    };
    format!("{prefix}{suffix}")
}

fn emit_table_mutations(
    conf: &MrConfiguration,
    statement: &RyaStatement,
) -> Result<Vec<(String, Mutation)>, String> {
    let prefix = conf.table_prefix_or_default();
    let factory = RyaTableMutationsFactory::default();
    let mut outputs = Vec::new();
    for (layout, mutations) in factory.serialize(statement)? {
        let table = table_name(prefix, layout);
        outputs.extend(
            mutations
                .into_iter()
                .map(|mutation| (table.clone(), mutation)),
        );
    }
    Ok(outputs)
}

pub fn upgraded_object_serialization(object: &str, type_marker: u8) -> Option<String> {
    match type_marker {
        10 => Some(if object == "true" { "1" } else { "0" }.to_string()),
        9 => object
            .parse::<i8>()
            .ok()
            .map(|value| format!("{:02x}", value as u8)),
        4 => object
            .parse::<i64>()
            .ok()
            .map(|value| format!("{:016x}", (value as u64) ^ 0x8000_0000_0000_0000)),
        5 => object
            .parse::<i32>()
            .ok()
            .map(|value| format!("{:08x}", (value as u32) ^ 0x8000_0000)),
        6 => upgrade_legacy_double(object),
        7 => object.parse::<u64>().ok().map(|encoded| {
            let millis = i64::MAX as u64 - encoded;
            format!("{:016x}", millis ^ 0x8000_0000_0000_0000)
        }),
        _ => None,
    }
}

pub fn upgrade_osp_row_object(row: &str) -> Option<String> {
    let first = row.find('\0')?;
    let second = row[first + 1..].find('\0')? + first + 1;
    let type_delim = row.rfind('\u{1}')?;
    let object = &row[..first];
    let type_marker = row.as_bytes().last().copied()?;
    let upgraded = upgraded_object_serialization(object, type_marker)?;
    Some(
        format!(
            "{upgraded}\0{}\0{}",
            &row[first + 1..second],
            &row[second + 1..type_delim + 1]
        ) + &(type_marker as char).to_string(),
    )
}

fn upgrade_legacy_double(object: &str) -> Option<String> {
    if object == "00001 1.0" {
        return Some("c024000000000000".to_string());
    }
    let exp = object.get(2..5)?.parse::<i32>().ok()?;
    let value_sign = object.chars().next()?;
    let exp_sign = object.chars().nth(1)?;
    let exp_int = if exp_sign == '-' { 999 - exp } else { exp };
    let mantissa = object.get(6..)?;
    let double = format!("{value_sign}{mantissa}E{exp_sign}{exp_int}")
        .parse::<f64>()
        .ok()?;
    let bits = double.to_bits();
    Some(format!(
        "{:016x}",
        bits ^ if bits >> 63 == 0 {
            0x8000_0000_0000_0000
        } else {
            u64::MAX
        }
    ))
}

fn parse_ntriples(input: &str) -> Result<Vec<RyaStatement>, String> {
    input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| parse_rdf_line(line, None))
        .collect()
}

fn parse_nquads(input: &str) -> Result<Vec<RyaStatement>, String> {
    input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let line = line.trim();
            let Some(graph_start) = line.rfind(" <") else {
                return parse_rdf_line(line, None);
            };
            let triple = format!("{} .", &line[..graph_start]);
            let graph = line[graph_start + 1..]
                .trim()
                .trim_end_matches('.')
                .trim()
                .trim_start_matches('<')
                .trim_end_matches('>');
            parse_rdf_line(&triple, Some(graph))
        })
        .collect()
}

fn parse_trig(input: &str) -> Result<Vec<RyaStatement>, String> {
    let mut prefixes: HashMap<String, String> = HashMap::new();
    let mut out = Vec::new();
    for raw in input.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("@prefix ") {
            let mut parts = rest.split_whitespace();
            let prefix = parts
                .next()
                .ok_or_else(|| "Missing TriG prefix".to_string())?
                .trim_end_matches(':')
                .to_string();
            let iri = parts
                .next()
                .ok_or_else(|| "Missing TriG prefix IRI".to_string())?
                .trim_start_matches('<')
                .trim_end_matches('>')
                .to_string();
            prefixes.insert(prefix, iri);
            continue;
        }
        let Some((graph, body)) = line.split_once('{') else {
            continue;
        };
        let graph = expand_prefixed(graph.trim(), &prefixes)?;
        let triple = body
            .trim()
            .trim_end_matches('}')
            .trim()
            .trim_end_matches('.')
            .trim();
        out.push(parse_trig_statement(triple, &graph, &prefixes)?);
    }
    Ok(out)
}

fn parse_trig_statement(
    triple: &str,
    graph: &str,
    prefixes: &HashMap<String, String>,
) -> Result<RyaStatement, String> {
    let mut parts = triple.split_whitespace();
    let subject = expand_prefixed(
        parts.next().ok_or_else(|| "Missing subject".to_string())?,
        prefixes,
    )?;
    let predicate = expand_prefixed(
        parts
            .next()
            .ok_or_else(|| "Missing predicate".to_string())?,
        prefixes,
    )?;
    let object_text = parts.collect::<Vec<_>>().join(" ");
    let object = if object_text.starts_with('"') {
        let end = object_text[1..]
            .find('"')
            .ok_or_else(|| "Unclosed literal".to_string())?
            + 1;
        RyaType::new(&object_text[1..end])
    } else {
        RyaIri::new(expand_prefixed(&object_text, prefixes)?)?.into_type()
    };
    let mut statement = RyaStatement::new(RyaIri::new(subject)?, RyaIri::new(predicate)?, object);
    statement.context = Some(RyaIri::new(graph)?);
    Ok(statement)
}

fn parse_rdf_line(line: &str, context: Option<&str>) -> Result<RyaStatement, String> {
    let line = line.trim().trim_end_matches('.').trim();
    let (subject, rest) = parse_angle_iri(line)?;
    let (predicate, rest) = parse_angle_iri(rest.trim_start())?;
    let rest = rest.trim_start();
    let object = if rest.starts_with('<') {
        let (object, _) = parse_angle_iri(rest)?;
        RyaIri::new(object)?.into_type()
    } else if let Some(literal) = rest.strip_prefix('"') {
        let end = literal
            .find('"')
            .ok_or_else(|| "Unclosed literal".to_string())?;
        RyaType::new(&literal[..end])
    } else {
        return Err(format!("Unsupported RDF object: {rest}"));
    };
    let mut statement = RyaStatement::new(RyaIri::new(subject)?, RyaIri::new(predicate)?, object);
    if let Some(context) = context {
        statement.context = Some(RyaIri::new(context)?);
        Ok(statement)
    } else {
        Ok(statement)
    }
}

fn parse_angle_iri(input: &str) -> Result<(&str, &str), String> {
    let input = input
        .strip_prefix('<')
        .ok_or_else(|| format!("Expected IRI: {input}"))?;
    let end = input
        .find('>')
        .ok_or_else(|| format!("Unclosed IRI: {input}"))?;
    Ok((&input[..end], &input[end + 1..]))
}

fn expand_prefixed(input: &str, prefixes: &HashMap<String, String>) -> Result<String, String> {
    let input = input.trim();
    if input.starts_with('<') && input.ends_with('>') {
        return Ok(input
            .trim_start_matches('<')
            .trim_end_matches('>')
            .to_string());
    }
    let (prefix, local) = input
        .split_once(':')
        .ok_or_else(|| format!("Expected prefixed name: {input}"))?;
    let namespace = prefixes
        .get(prefix)
        .ok_or_else(|| format!("Unknown prefix: {prefix}"))?;
    Ok(format!("{namespace}{local}"))
}

fn layout_name(layout: TableLayout) -> &'static str {
    match layout {
        TableLayout::Spo => "SPO",
        TableLayout::Po => "PO",
        TableLayout::Osp => "OSP",
    }
}

fn write_field(out: &mut Vec<u8>, field: Option<&[u8]>) {
    match field {
        Some(bytes) => {
            out.push(1);
            out.extend_from_slice(&(bytes.len() as i32).to_be_bytes());
            out.extend_from_slice(bytes);
        }
        None => out.push(0),
    }
}

fn read_field(bytes: &[u8], cursor: &mut usize) -> Result<Option<Vec<u8>>, String> {
    if !read_bool(bytes, cursor)? {
        return Ok(None);
    }
    if bytes.len().saturating_sub(*cursor) < 4 {
        return Err("Truncated field length".to_string());
    }
    let len = i32::from_be_bytes(bytes[*cursor..*cursor + 4].try_into().unwrap());
    *cursor += 4;
    if len < 0 {
        return Err("Negative field length".to_string());
    }
    let len = len as usize;
    if bytes.len().saturating_sub(*cursor) < len {
        return Err("Truncated field bytes".to_string());
    }
    let field = bytes[*cursor..*cursor + len].to_vec();
    *cursor += len;
    Ok(Some(field))
}

fn read_bool(bytes: &[u8], cursor: &mut usize) -> Result<bool, String> {
    let byte = *bytes
        .get(*cursor)
        .ok_or_else(|| "Truncated boolean".to_string())?;
    *cursor += 1;
    match byte {
        0 => Ok(false),
        1 => Ok(true),
        other => Err(format!("Invalid boolean byte: {other}")),
    }
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, String> {
    if bytes.len().saturating_sub(*cursor) < 8 {
        return Err("Truncated timestamp".to_string());
    }
    let timestamp = u64::from_be_bytes(bytes[*cursor..*cursor + 8].try_into().unwrap());
    *cursor += 8;
    Ok(timestamp)
}

#[cfg(test)]
#[path = "tests/fjall_mr_tests.rs"]
mod tests;
