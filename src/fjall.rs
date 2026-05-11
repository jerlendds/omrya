use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::{RyaStatement, RyaType, XSD_DATE, XSD_DATETIME};
use crate::query::{InMemoryRyaDao, QueryOptions, StatementPattern};
use crate::resolver::datetime::normalize_xsd_datetime_to_utc;
use crate::resolver::serialize_type;
use crate::resolver::triple::{
    TableLayout, TripleContext, TriplePatternStrategyKind, TripleRowRegex,
};
use fjall_kv::secure::{
    AllowAllPermissions, Authorizations, Cell, CompositeKey, CompositePrefix, Credentials,
    SecureDatabase, SecureKeyspace, Session, StaticAuthenticator, StaticAuthorizor, VersionScan,
    VisibilityExpr,
};

pub const CONF_ADDITIONAL_INDEXERS: &str = "ac.additional.indexers";
pub const CONF_FLUSH_EACH_UPDATE: &str = "ac.dao.flush";
pub const CONF_FJALL_DATA_DIR: &str = "fjall.data.dir";
pub const ITERATOR_SETTINGS_SIZE: &str = "ac.iterators.size";
pub const ITERATOR_SETTINGS_BASE: &str = "ac.iterators.%d.";
pub const ITERATOR_SETTINGS_NAME: &str = "ac.iterators.%d.name";
pub const ITERATOR_SETTINGS_KIND: &str = "ac.iterators.%d.iterator";
pub const ITERATOR_SETTINGS_PRIORITY: &str = "ac.iterators.%d.priority";
pub const ITERATOR_SETTINGS_OPTIONS_SIZE: &str = "ac.iterators.%d.optionsSize";
pub const ITERATOR_SETTINGS_OPTIONS_KEY: &str = "ac.iterators.%d.option.%d.name";
pub const ITERATOR_SETTINGS_OPTIONS_VALUE: &str = "ac.iterators.%d.option.%d.value";
pub const CONF_TBL_PREFIX: &str = "rdf.tablePrefix";
pub const CONF_QUERY_AUTH: &str = "query.auth";
pub const CONF_CV: &str = "conf.cv";
pub const CONF_INFER: &str = "query.infer";
pub const VERSION_SUBJECT_RYA: &str = "urn:omrya/version";
pub const VERSION_PREDICATE_RYA: &str = "urn:omrya/versionPredicate";
pub const VERSION_RYA: &str = "3.2.10-SNAPSHOT";

pub const FIRST_ENTRY_IN_ROW_ITERATOR_ID: &str = "omrya::fjall::iterators::first_entry_in_row";
pub const DEFAULT_CONNECTOR_ID: &str = "in-memory-fjall-connector";
pub const DEFAULT_MULTI_TABLE_BATCH_WRITER_ID: &str = "in-memory-multi-table-batch-writer";
pub const FJALL_DATETIME_DEFAULT_OFFSET_MINUTES: i32 = 0;
pub const TBL_PRFX_DEF: &str = "rya_";
pub const TBL_SPO_SUFFIX: &str = "spo";
pub const TBL_PO_SUFFIX: &str = "po";
pub const TBL_OSP_SUFFIX: &str = "osp";
pub const TBL_NS_SUFFIX: &str = "ns";
pub const TBL_EVAL_SUFFIX: &str = "eval";
pub const TBL_STATS_SUFFIX: &str = "prospects";
pub const TBL_SEL_SUFFIX: &str = "selectivity";
pub const ENTITY_INDEX_SUFFIX: &str = "entity";
pub const FREETEXT_DOC_INDEX_SUFFIX: &str = "freetext";
pub const FREETEXT_TERM_INDEX_SUFFIX: &str = "freetext_term";
pub const GEO_INDEX_SUFFIX: &str = "geo";
pub const TEMPORAL_INDEX_SUFFIX: &str = "temporal";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TablePrefixLayoutStrategy {
    table_prefix: String,
}

impl Default for TablePrefixLayoutStrategy {
    fn default() -> Self {
        Self::new(TBL_PRFX_DEF)
    }
}

impl TablePrefixLayoutStrategy {
    pub fn new(table_prefix: impl Into<String>) -> Self {
        Self {
            table_prefix: table_prefix.into(),
        }
    }

    pub fn table_prefix(&self) -> &str {
        &self.table_prefix
    }

    pub fn spo(&self) -> String {
        format!("{}{}", self.table_prefix, TBL_SPO_SUFFIX)
    }

    pub fn po(&self) -> String {
        format!("{}{}", self.table_prefix, TBL_PO_SUFFIX)
    }

    pub fn osp(&self) -> String {
        format!("{}{}", self.table_prefix, TBL_OSP_SUFFIX)
    }

    pub fn ns(&self) -> String {
        format!("{}{}", self.table_prefix, TBL_NS_SUFFIX)
    }

    pub fn eval(&self) -> String {
        format!("{}{}", self.table_prefix, TBL_EVAL_SUFFIX)
    }

    pub fn prospects(&self) -> String {
        format!("{}{}", self.table_prefix, TBL_STATS_SUFFIX)
    }

    pub fn selectivity(&self) -> String {
        format!("{}{}", self.table_prefix, TBL_SEL_SUFFIX)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FjallQueryScanPlan {
    pub layout: TableLayout,
    pub strategy: TriplePatternStrategyKind,
    pub full_table_scan: bool,
    pub triple_row_regex: Option<TripleRowRegex>,
}

pub fn plan_single_query_scan(
    context: &TripleContext,
    selected_strategy: Option<TriplePatternStrategyKind>,
    pattern: &StatementPattern,
    options: &QueryOptions,
) -> Result<FjallQueryScanPlan, String> {
    let (strategy, full_table_scan) = match selected_strategy {
        Some(strategy) => (strategy, false),
        None => (
            context.retrieve_strategy(TableLayout::Spo).ok_or_else(|| {
                "No SPO strategy available for default full-table scan".to_string()
            })?,
            true,
        ),
    };
    let object_type_info = pattern
        .object
        .as_ref()
        .map(|object| serialize_type(object).map(|(_, type_info)| type_info))
        .transpose()?;
    let triple_row_regex = strategy.build_regex(
        options.regex_subject.as_deref(),
        options.regex_predicate.as_deref(),
        options.regex_object.as_deref(),
        None,
        object_type_info.as_deref(),
    );

    Ok(FjallQueryScanPlan {
        layout: strategy.layout(),
        strategy,
        full_table_scan,
        triple_row_regex,
    })
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum FjallInstanceKind {
    #[default]
    Real,
    Mock,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FjallConnector {
    pub id: String,
    pub instance_kind: FjallInstanceKind,
}

impl Default for FjallConnector {
    fn default() -> Self {
        Self {
            id: DEFAULT_CONNECTOR_ID.to_string(),
            instance_kind: FjallInstanceKind::Real,
        }
    }
}

impl FjallConnector {
    pub fn mock(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            instance_kind: FjallInstanceKind::Mock,
        }
    }

    pub fn is_mock_instance(&self) -> bool {
        self.instance_kind == FjallInstanceKind::Mock
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MultiTableBatchWriter {
    pub id: String,
}

impl Default for MultiTableBatchWriter {
    fn default() -> Self {
        Self {
            id: DEFAULT_MULTI_TABLE_BATCH_WRITER_ID.to_string(),
        }
    }
}

pub trait FjallIndexer {
    fn get_table_name(&self) -> Option<String> {
        None
    }

    fn set_conf(&mut self, _conf: &FjallRdfConfiguration) -> Result<(), String> {
        Ok(())
    }

    fn set_connector(&mut self, connector: FjallConnector) -> Result<(), String>;

    fn set_multi_table_batch_writer(&mut self, writer: MultiTableBatchWriter)
    -> Result<(), String>;

    fn init(&mut self) -> Result<(), String>;

    fn store_statement(&mut self, _statement: &RyaStatement) -> Result<(), String> {
        Ok(())
    }

    fn store_statements(&mut self, statements: &[RyaStatement]) -> Result<(), String> {
        for statement in statements {
            self.store_statement(statement)?;
        }
        Ok(())
    }

    fn delete_statement(&mut self, _statement: &RyaStatement) -> Result<(), String> {
        Ok(())
    }

    fn flush(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn destroy(&mut self) -> Result<(), String>;

    fn purge(&mut self, configuration: &FjallRdfConfiguration) -> Result<(), String>;

    fn drop_and_destroy(&mut self) -> Result<(), String>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildFixIndexerKind {
    NullFreeText,
    NullGeo,
    NullTemporal,
    EntityCentric,
    FreeText,
    GeoMesaGeo,
    Temporal,
}

impl BuildFixIndexerKind {
    pub fn registry_name(&self) -> &'static str {
        match self {
            Self::NullFreeText => "omrya::fjall::indexers::null_free_text",
            Self::NullGeo => "omrya::fjall::indexers::null_geo",
            Self::NullTemporal => "omrya::fjall::indexers::null_temporal",
            Self::EntityCentric => "omrya::fjall::indexers::entity_centric",
            Self::FreeText => "omrya::fjall::indexers::free_text",
            Self::GeoMesaGeo => "omrya::fjall::indexers::geomesa_geo",
            Self::Temporal => "omrya::fjall::indexers::temporal",
        }
    }

    pub fn all_indexer_kinds() -> [Self; 7] {
        [
            Self::NullFreeText,
            Self::NullGeo,
            Self::NullTemporal,
            Self::EntityCentric,
            Self::FreeText,
            Self::GeoMesaGeo,
            Self::Temporal,
        ]
    }

    fn initializes_from_conf(&self) -> bool {
        !matches!(
            self,
            Self::NullFreeText | Self::NullGeo | Self::NullTemporal
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildFixFjallIndexer {
    kind: BuildFixIndexerKind,
    conf_seen: bool,
    connector_seen: Option<FjallConnector>,
    writer_seen: Option<MultiTableBatchWriter>,
    internal_init_count: usize,
}

impl BuildFixFjallIndexer {
    pub fn new(kind: BuildFixIndexerKind) -> Self {
        Self {
            kind,
            conf_seen: false,
            connector_seen: None,
            writer_seen: None,
            internal_init_count: 0,
        }
    }

    pub fn kind(&self) -> BuildFixIndexerKind {
        self.kind
    }

    pub fn conf_seen(&self) -> bool {
        self.conf_seen
    }

    pub fn connector_seen(&self) -> Option<&FjallConnector> {
        self.connector_seen.as_ref()
    }

    pub fn writer_seen(&self) -> Option<&MultiTableBatchWriter> {
        self.writer_seen.as_ref()
    }

    pub fn internal_init_count(&self) -> usize {
        self.internal_init_count
    }

    fn init_internal_once(&mut self) {
        if self.internal_init_count == 0 {
            self.internal_init_count = 1;
        }
    }
}

impl FjallIndexer for BuildFixFjallIndexer {
    fn set_conf(&mut self, _conf: &FjallRdfConfiguration) -> Result<(), String> {
        self.conf_seen = true;
        if self.kind.initializes_from_conf() {
            self.init_internal_once();
        }
        Ok(())
    }

    fn set_connector(&mut self, connector: FjallConnector) -> Result<(), String> {
        self.connector_seen = Some(connector);
        Ok(())
    }

    fn set_multi_table_batch_writer(
        &mut self,
        writer: MultiTableBatchWriter,
    ) -> Result<(), String> {
        self.writer_seen = Some(writer);
        Ok(())
    }

    fn init(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn destroy(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn purge(&mut self, _configuration: &FjallRdfConfiguration) -> Result<(), String> {
        Ok(())
    }

    fn drop_and_destroy(&mut self) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IteratorSetting {
    pub priority: i32,
    pub name: String,
    pub iterator_id: String,
    pub options: BTreeMap<String, String>,
}

impl IteratorSetting {
    pub fn new(
        priority: i32,
        name: impl Into<String>,
        iterator_id: impl Into<String>,
        options: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        Self {
            priority,
            name: name.into(),
            iterator_id: iterator_id.into(),
            options: options
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        }
    }

    pub fn first_entry_in_row(priority: i32) -> Self {
        Self::new(
            priority,
            "FirstEntryInRowIterator",
            FIRST_ENTRY_IN_ROW_ITERATOR_ID,
            std::iter::empty::<(String, String)>(),
        )
    }

    fn is_first_entry_in_row(&self) -> bool {
        self.iterator_id == FIRST_ENTRY_IN_ROW_ITERATOR_ID || self.iterator_id == "first_entry_in_row"
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FjallRdfConfiguration {
    values: BTreeMap<String, String>,
}

impl FjallRdfConfiguration {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_parent(parent: &Self) -> Self {
        parent.clone()
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    pub fn set_cv(&mut self, visibility: impl Into<String>) {
        self.set(CONF_CV, visibility);
    }

    pub fn cv(&self) -> Option<&str> {
        self.get(CONF_CV)
    }

    pub fn is_infer(&self) -> bool {
        self.get(CONF_INFER)
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    }

    pub fn set_infer(&mut self, infer: bool) {
        self.set(CONF_INFER, infer.to_string());
    }

    pub fn flush_each_update(&self) -> bool {
        self.get(CONF_FLUSH_EACH_UPDATE)
            .map(|value| value.eq_ignore_ascii_case("true"))
            .unwrap_or(true)
    }

    pub fn set_flush(&mut self, flush: bool) {
        self.set(CONF_FLUSH_EACH_UPDATE, flush.to_string());
    }

    pub fn set_table_prefix(&mut self, prefix: impl Into<String>) {
        self.set(CONF_TBL_PREFIX, prefix);
    }

    pub fn table_prefix(&self) -> Option<&str> {
        self.get(CONF_TBL_PREFIX)
    }

    pub fn set_fjall_data_dir(&mut self, path: impl Into<String>) {
        self.set(CONF_FJALL_DATA_DIR, path);
    }

    pub fn fjall_data_dir(&self) -> Option<&str> {
        self.get(CONF_FJALL_DATA_DIR)
    }

    pub fn required_table_prefix(&self) -> Result<&str, String> {
        self.table_prefix().ok_or_else(|| {
            format!("Configuration key: {CONF_TBL_PREFIX} not set.  Cannot generate table name.")
        })
    }

    pub fn table_layout_strategy(&self) -> TablePrefixLayoutStrategy {
        TablePrefixLayoutStrategy::new(self.table_prefix().unwrap_or(TBL_PRFX_DEF))
    }

    pub fn entity_index_table_name(&self) -> Result<String, String> {
        self.secondary_index_table_name(ENTITY_INDEX_SUFFIX)
    }

    pub fn freetext_doc_table_name(&self) -> Result<String, String> {
        self.secondary_index_table_name(FREETEXT_DOC_INDEX_SUFFIX)
    }

    pub fn freetext_term_table_name(&self) -> Result<String, String> {
        self.secondary_index_table_name(FREETEXT_TERM_INDEX_SUFFIX)
    }

    pub fn freetext_table_names(&self) -> Result<Vec<String>, String> {
        Ok(vec![
            self.freetext_doc_table_name()?,
            self.freetext_term_table_name()?,
        ])
    }

    pub fn geo_index_table_name(&self) -> Result<String, String> {
        self.secondary_index_table_name(GEO_INDEX_SUFFIX)
    }

    pub fn temporal_index_table_name(&self) -> Result<String, String> {
        self.secondary_index_table_name(TEMPORAL_INDEX_SUFFIX)
    }

    fn secondary_index_table_name(&self, suffix: &str) -> Result<String, String> {
        Ok(format!("{}{}", self.required_table_prefix()?, suffix))
    }

    pub fn set_additional_iterators(&mut self, iterators: &[IteratorSetting]) {
        self.set(ITERATOR_SETTINGS_SIZE, iterators.len().to_string());
        for (i, iterator) in iterators.iter().enumerate() {
            self.set(
                iterator_key(ITERATOR_SETTINGS_NAME, i, None),
                &iterator.name,
            );
            self.set(
                iterator_key(ITERATOR_SETTINGS_KIND, i, None),
                &iterator.iterator_id,
            );
            self.set(
                iterator_key(ITERATOR_SETTINGS_PRIORITY, i, None),
                iterator.priority.to_string(),
            );
            self.set(
                iterator_key(ITERATOR_SETTINGS_OPTIONS_SIZE, i, None),
                iterator.options.len().to_string(),
            );
            for (j, (key, value)) in iterator.options.iter().enumerate() {
                self.set(iterator_key(ITERATOR_SETTINGS_OPTIONS_KEY, i, Some(j)), key);
                self.set(
                    iterator_key(ITERATOR_SETTINGS_OPTIONS_VALUE, i, Some(j)),
                    value,
                );
            }
        }
    }

    pub fn get_additional_iterators(&self) -> Result<Vec<IteratorSetting>, String> {
        let size = self
            .get(ITERATOR_SETTINGS_SIZE)
            .unwrap_or("0")
            .parse::<usize>()
            .map_err(|e| format!("Invalid {ITERATOR_SETTINGS_SIZE}: {e}"))?;
        let mut settings = Vec::with_capacity(size);
        for i in 0..size {
            let name = self
                .get(&iterator_key(ITERATOR_SETTINGS_NAME, i, None))
                .ok_or_else(|| format!("Missing iterator name at index {i}"))?
                .to_string();
            let iterator_id = self
                .get(&iterator_key(ITERATOR_SETTINGS_KIND, i, None))
                .ok_or_else(|| format!("Missing iterator class at index {i}"))?
                .to_string();
            let priority = self
                .get(&iterator_key(ITERATOR_SETTINGS_PRIORITY, i, None))
                .ok_or_else(|| format!("Missing iterator priority at index {i}"))?
                .parse::<i32>()
                .map_err(|e| format!("Invalid iterator priority at index {i}: {e}"))?;
            let options_size = self
                .get(&iterator_key(ITERATOR_SETTINGS_OPTIONS_SIZE, i, None))
                .ok_or_else(|| format!("Missing iterator option count at index {i}"))?
                .parse::<usize>()
                .map_err(|e| format!("Invalid iterator option count at index {i}: {e}"))?;
            let mut options = BTreeMap::new();
            for j in 0..options_size {
                let key = self
                    .get(&iterator_key(ITERATOR_SETTINGS_OPTIONS_KEY, i, Some(j)))
                    .ok_or_else(|| format!("Missing iterator option key at index {i}.{j}"))?
                    .to_string();
                let value = self
                    .get(&iterator_key(ITERATOR_SETTINGS_OPTIONS_VALUE, i, Some(j)))
                    .ok_or_else(|| format!("Missing iterator option value at index {i}.{j}"))?
                    .to_string();
                options.insert(key, value);
            }
            settings.push(IteratorSetting {
                priority,
                name,
                iterator_id,
                options,
            });
        }
        Ok(settings)
    }
}

pub struct FjallRyaDao {
    committed: FjallRyaStore,
    pending: Vec<PendingMutation>,
    flush_each_update: bool,
    flush_count: usize,
    connector: FjallConnector,
    multi_table_batch_writer: MultiTableBatchWriter,
    table_layout_strategy: TablePrefixLayoutStrategy,
    secondary_indexers: Vec<Box<dyn FjallIndexer>>,
    lifecycle_errors: Vec<String>,
    initialized: bool,
    core_version_mutation_count: usize,
}

impl Default for FjallRyaDao {
    fn default() -> Self {
        Self::new(&FjallRdfConfiguration::default())
    }
}

impl FjallRyaDao {
    pub fn new(conf: &FjallRdfConfiguration) -> Self {
        Self::try_new_with_indexers(conf, Vec::new())
            .expect("empty Fjall-backed Rya indexer list cannot fail to initialize")
    }

    pub fn try_new_with_indexers(
        conf: &FjallRdfConfiguration,
        secondary_indexers: Vec<Box<dyn FjallIndexer>>,
    ) -> Result<Self, String> {
        let mut dao = Self {
            committed: FjallRyaStore::new(conf)?,
            pending: Vec::new(),
            flush_each_update: conf.flush_each_update(),
            flush_count: 0,
            connector: FjallConnector::default(),
            multi_table_batch_writer: MultiTableBatchWriter::default(),
            table_layout_strategy: conf.table_layout_strategy(),
            secondary_indexers,
            lifecycle_errors: Vec::new(),
            initialized: false,
            core_version_mutation_count: 0,
        };

        for indexer in &mut dao.secondary_indexers {
            indexer.set_conf(conf)?;
        }
        for indexer in &mut dao.secondary_indexers {
            indexer.set_connector(dao.connector.clone())?;
            indexer.set_multi_table_batch_writer(dao.multi_table_batch_writer.clone())?;
            indexer.init()?;
        }

        dao.initialized = true;
        dao.check_version_without_indexers();
        Ok(dao)
    }

    pub fn add(&mut self, statement: RyaStatement) {
        let statement = normalize_statement_for_fjall(statement).expect("valid Fjall RyaStatement");
        for indexer in &mut self.secondary_indexers {
            if let Err(error) = indexer.store_statement(&statement) {
                self.lifecycle_errors.push(format!(
                    "Failed to update indexer with added statement: {error}"
                ));
            }
        }
        self.pending.push(PendingMutation::Add(statement));
        self.flush_if_configured();
    }

    pub fn delete_exact(&mut self, statement: RyaStatement) {
        let statement = normalize_statement_for_fjall(statement).expect("valid Fjall RyaStatement");
        for indexer in &mut self.secondary_indexers {
            if let Err(error) = indexer.delete_statement(&statement) {
                self.lifecycle_errors.push(format!(
                    "Failed to update indexer with deleted statement: {error}"
                ));
            }
        }
        self.pending.push(PendingMutation::Delete(statement));
        self.flush_if_configured();
    }

    pub fn add_namespace(&mut self, prefix: impl Into<String>, namespace: impl Into<String>) {
        self.pending.push(PendingMutation::AddNamespace {
            prefix: prefix.into(),
            namespace: namespace.into(),
        });
        self.flush_if_configured();
    }

    pub fn remove_namespace(&mut self, prefix: impl Into<String>) {
        self.pending
            .push(PendingMutation::RemoveNamespace(prefix.into()));
        self.flush_if_configured();
    }

    pub fn query(
        &self,
        pattern: &StatementPattern,
        options: &QueryOptions,
        conf: &FjallRdfConfiguration,
    ) -> Result<Vec<RyaStatement>, String> {
        let pattern = normalize_pattern_for_fjall(pattern)?;
        let results = self.committed.query(&pattern, options)?;
        apply_additional_iterators(results, &pattern, &conf.get_additional_iterators()?)
    }

    pub fn batch_query(
        &self,
        patterns: &[StatementPattern],
        options: &QueryOptions,
        conf: &FjallRdfConfiguration,
    ) -> Result<Vec<RyaStatement>, String> {
        let mut seen_patterns = std::collections::BTreeSet::new();
        let mut seen_results = std::collections::BTreeSet::new();
        let mut results = Vec::new();

        for pattern in patterns {
            let normalized = normalize_pattern_for_fjall(pattern)?;
            if !seen_patterns.insert(pattern_storage_key(&normalized)) {
                continue;
            }
            for statement in self.query(&normalized, options, conf)? {
                if seen_results.insert(fjall_statement_key(&statement)) {
                    results.push(statement);
                }
            }
        }

        Ok(results)
    }

    pub fn get_namespace(&self, prefix: &str) -> Option<&str> {
        self.committed.get_namespace(prefix)
    }

    pub fn flush(&mut self) {
        for indexer in &mut self.secondary_indexers {
            if let Err(error) = indexer.flush() {
                self.lifecycle_errors
                    .push(format!("Failed to flush indexer: {error}"));
            }
        }
        let pending = std::mem::take(&mut self.pending);
        for mutation in pending {
            match mutation {
                PendingMutation::Add(statement) => {
                    if let Err(error) = self.committed.upsert(statement) {
                        self.lifecycle_errors
                            .push(format!("Failed to persist statement to Fjall: {error}"));
                    }
                }
                PendingMutation::Delete(statement) => {
                    if let Err(error) = self.committed.delete_exact(&statement) {
                        self.lifecycle_errors
                            .push(format!("Failed to delete statement from Fjall: {error}"));
                    }
                }
                PendingMutation::AddNamespace { prefix, namespace } => {
                    self.committed.add_namespace(prefix, namespace);
                }
                PendingMutation::RemoveNamespace(prefix) => {
                    self.committed.remove_namespace(&prefix);
                }
            }
        }
        self.flush_count += 1;
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    pub fn flush_count(&self) -> usize {
        self.flush_count
    }

    pub fn secondary_indexer_count(&self) -> usize {
        self.secondary_indexers.len()
    }

    pub fn lifecycle_errors(&self) -> &[String] {
        &self.lifecycle_errors
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn core_version_mutation_count(&self) -> usize {
        self.core_version_mutation_count
    }

    pub fn table_layout_strategy(&self) -> &TablePrefixLayoutStrategy {
        &self.table_layout_strategy
    }

    pub fn table_prefix(&self) -> &str {
        self.table_layout_strategy.table_prefix()
    }

    pub fn destroy(&mut self) {
        if !self.initialized {
            return;
        }
        self.flush();
        self.initialized = false;

        let mut errors = Vec::new();
        for indexer in &mut self.secondary_indexers {
            if let Err(error) = indexer.destroy() {
                errors.push(format!("Failed to destroy indexer: {error}"));
            }
        }
        self.lifecycle_errors.extend(errors);
    }

    pub fn purge(&mut self, configuration: &FjallRdfConfiguration) {
        if let Err(error) = self.committed.clear() {
            self.lifecycle_errors
                .push(format!("Failed to purge Fjall keyspace: {error}"));
        }
        self.pending.clear();

        let mut errors = Vec::new();
        for indexer in &mut self.secondary_indexers {
            if let Err(error) = indexer.purge(configuration) {
                errors.push(format!("Failed to purge indexer: {error}"));
            }
        }
        self.lifecycle_errors.extend(errors);
    }

    pub fn drop_and_destroy(&mut self) {
        if let Err(error) = self.committed.clear() {
            self.lifecycle_errors
                .push(format!("Failed to drop Fjall keyspace: {error}"));
        }
        self.pending.clear();
        self.destroy();

        let mut errors = Vec::new();
        for indexer in &mut self.secondary_indexers {
            if let Err(error) = indexer.drop_and_destroy() {
                errors.push(format!("Failed to drop and destroy indexer: {error}"));
            }
        }
        self.lifecycle_errors.extend(errors);
    }

    fn flush_if_configured(&mut self) {
        if self.flush_each_update {
            self.flush();
        }
    }

    fn check_version_without_indexers(&mut self) {
        self.core_version_mutation_count += 1;
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum PendingMutation {
    Add(RyaStatement),
    Delete(RyaStatement),
    AddNamespace { prefix: String, namespace: String },
    RemoveNamespace(String),
}

const FJALL_STATEMENT_ROW: &str = "statements";
const FJALL_STATEMENT_QUALIFIER: &str = "triple";
const FJALL_DEFAULT_PRINCIPAL: &str = "omrya";
const FJALL_DEFAULT_SECRET: &str = "local-fjall";
const STATEMENT_VALUE_VERSION: u8 = 1;
const NONE_FIELD_LEN: u32 = u32::MAX;

struct FjallRyaStore {
    _data_dir: PathBuf,
    _database: SecureDatabase<String>,
    keyspace: SecureKeyspace<String>,
    write_session: Session<String>,
    namespaces: BTreeMap<String, String>,
}

impl FjallRyaStore {
    fn new(conf: &FjallRdfConfiguration) -> Result<Self, String> {
        let data_dir = conf
            .fjall_data_dir()
            .map(PathBuf::from)
            .unwrap_or_else(default_fjall_data_dir);
        let keyspace_name = format!("{}triples", conf.table_prefix().unwrap_or(TBL_PRFX_DEF));
        let auths = Authorizations::empty();

        let database = SecureDatabase::builder(&data_dir)
            .authenticator(StaticAuthenticator::new().with_principal(
                FJALL_DEFAULT_PRINCIPAL,
                FJALL_DEFAULT_SECRET,
                FJALL_DEFAULT_PRINCIPAL.to_string(),
            ))
            .permission_handler(AllowAllPermissions::<String>::new())
            .authorizor(StaticAuthorizor::new(auths))
            .open()
            .map_err(|e| format!("Failed to open Fjall store at {}: {e}", data_dir.display()))?;
        let write_session = database
            .authenticate(&Credentials::new(
                FJALL_DEFAULT_PRINCIPAL,
                FJALL_DEFAULT_SECRET,
            ))
            .map_err(|e| format!("Failed to authenticate Fjall compatibility session: {e}"))?;
        let keyspace = database
            .secure_keyspace(&keyspace_name, Default::default)
            .map_err(|e| format!("Failed to open Fjall secure keyspace {keyspace_name}: {e}"))?;

        Ok(Self {
            _data_dir: data_dir,
            _database: database,
            keyspace,
            write_session,
            namespaces: BTreeMap::new(),
        })
    }

    fn upsert(&self, statement: RyaStatement) -> Result<(), String> {
        self.delete_storage_key(&statement)?;
        let cell = statement_cell(&statement)?;
        self.keyspace
            .insert_version(
                &self.write_session,
                &cell.version(statement_timestamp(&statement)),
                serialize_statement(&statement)?,
            )
            .map_err(|e| format!("Fjall insert failed: {e}"))
    }

    fn delete_exact(&self, statement: &RyaStatement) -> Result<(), String> {
        let cell = statement_cell(statement)?;
        self.keyspace
            .delete_version(
                &self.write_session,
                &cell.version(statement_timestamp(statement)),
            )
            .map_err(|e| format!("Fjall delete failed: {e}"))
    }

    fn query(
        &self,
        pattern: &StatementPattern,
        options: &QueryOptions,
    ) -> Result<Vec<RyaStatement>, String> {
        let session = Session {
            identity: FJALL_DEFAULT_PRINCIPAL.to_string(),
            auths: authorizations_from_options(options)?,
        };
        let entries = self
            .keyspace
            .scan_versioned_prefix(
                &session,
                &CompositePrefix {
                    row: Some(FJALL_STATEMENT_ROW.into()),
                    family: None,
                    qualifier: None,
                    visibility: None,
                },
                &VersionScan::default(),
            )
            .map_err(|e| format!("Fjall scan failed: {e}"))?;
        let mut dao = InMemoryRyaDao::new();

        for entry in entries {
            dao.add(deserialize_statement(entry.value.as_ref())?);
        }

        Ok(dao.query(pattern, options))
    }

    fn add_namespace(&mut self, prefix: impl Into<String>, namespace: impl Into<String>) {
        self.namespaces.insert(prefix.into(), namespace.into());
    }

    fn get_namespace(&self, prefix: &str) -> Option<&str> {
        self.namespaces.get(prefix).map(String::as_str)
    }

    fn remove_namespace(&mut self, prefix: &str) {
        self.namespaces.remove(prefix);
    }

    fn clear(&mut self) -> Result<(), String> {
        let range = CompositePrefix {
            row: Some(FJALL_STATEMENT_ROW.into()),
            family: None,
            qualifier: None,
            visibility: None,
        }
        .range()
        .map_err(|e| format!("Failed to build Fjall clear range: {e}"))?;
        let mut keys = Vec::new();

        for guard in self.keyspace.inner().range(range) {
            keys.push(
                guard
                    .key()
                    .map_err(|e| format!("Failed to read Fjall key during clear: {e}"))?,
            );
        }

        for key in keys {
            self.keyspace
                .inner()
                .remove(key)
                .map_err(|e| format!("Failed to remove Fjall key during clear: {e}"))?;
        }

        self.namespaces.clear();
        Ok(())
    }

    fn delete_storage_key(&self, statement: &RyaStatement) -> Result<(), String> {
        let range = CompositePrefix {
            row: Some(FJALL_STATEMENT_ROW.into()),
            family: Some(fjall_statement_key(statement).into()),
            qualifier: Some(FJALL_STATEMENT_QUALIFIER.into()),
            visibility: None,
        }
        .range()
        .map_err(|e| format!("Failed to build Fjall upsert range: {e}"))?;
        let mut keys = Vec::new();

        for guard in self.keyspace.inner().range(range) {
            let key = guard
                .key()
                .map_err(|e| format!("Failed to read Fjall key during upsert: {e}"))?;
            let _ = CompositeKey::decode(&key)
                .map_err(|e| format!("Fjall stored an invalid composite key: {e}"))?;
            keys.push(key);
        }

        for key in keys {
            self.keyspace
                .inner()
                .remove(key)
                .map_err(|e| format!("Failed to remove old Fjall version during upsert: {e}"))?;
        }

        Ok(())
    }
}

fn default_fjall_data_dir() -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos());

    std::env::temp_dir().join(format!("omrya-fjall-{}-{nanos}", std::process::id()))
}

fn statement_cell(statement: &RyaStatement) -> Result<Cell, String> {
    Ok(Cell {
        row: FJALL_STATEMENT_ROW.into(),
        family: fjall_statement_key(statement).into(),
        qualifier: FJALL_STATEMENT_QUALIFIER.into(),
        visibility: canonical_visibility(statement.column_visibility.as_deref())?.into(),
    })
}

fn canonical_visibility(visibility: Option<&[u8]>) -> Result<String, String> {
    let Some(visibility) = visibility else {
        return Ok(String::new());
    };
    let expression = std::str::from_utf8(visibility)
        .map_err(|e| format!("Fjall secure visibility must be valid UTF-8: {e}"))?;

    VisibilityExpr::parse(expression)
        .map(VisibilityExpr::into_string)
        .map_err(|e| format!("Invalid Fjall secure visibility expression: {e}"))
}

fn authorizations_from_options(options: &QueryOptions) -> Result<Authorizations, String> {
    Authorizations::from_labels(options.auths.iter().cloned())
        .map_err(|e| format!("Invalid Fjall authorization label: {e}"))
}

fn statement_timestamp(statement: &RyaStatement) -> i64 {
    i64::try_from(statement.timestamp).unwrap_or(i64::MAX)
}

fn serialize_statement(statement: &RyaStatement) -> Result<Vec<u8>, String> {
    let mut bytes = vec![STATEMENT_VALUE_VERSION];

    write_text(&mut bytes, statement.subject.data())?;
    write_text(&mut bytes, statement.predicate.data())?;
    write_optional_text(&mut bytes, statement.object.data_type())?;
    write_text(&mut bytes, statement.object.data())?;
    write_optional_text(&mut bytes, statement.object.language())?;
    write_optional_text(
        &mut bytes,
        statement.context.as_ref().map(|context| context.data()),
    )?;
    write_optional_text(&mut bytes, statement.qualifier.as_deref())?;
    write_optional_bytes(&mut bytes, statement.column_visibility.as_deref())?;
    write_optional_bytes(&mut bytes, statement.value.as_deref())?;
    bytes.extend_from_slice(&statement.timestamp.to_be_bytes());

    Ok(bytes)
}

fn deserialize_statement(bytes: &[u8]) -> Result<RyaStatement, String> {
    let mut cursor = 0;
    let version = read_u8(bytes, &mut cursor)?;
    if version != STATEMENT_VALUE_VERSION {
        return Err(format!(
            "Unsupported Fjall statement value version: {version}"
        ));
    }

    let subject = crate::domain::RyaIri::new(read_text(bytes, &mut cursor)?)
        .map_err(|e| format!("Invalid serialized subject IRI: {e}"))?;
    let predicate = crate::domain::RyaIri::new(read_text(bytes, &mut cursor)?)
        .map_err(|e| format!("Invalid serialized predicate IRI: {e}"))?;
    let object_data_type = read_optional_text(bytes, &mut cursor)?;
    let object_data = read_text(bytes, &mut cursor)?;
    let object_language = read_optional_text(bytes, &mut cursor)?;
    let context = read_optional_text(bytes, &mut cursor)?
        .map(crate::domain::RyaIri::new)
        .transpose()
        .map_err(|e| format!("Invalid serialized context IRI: {e}"))?;
    let qualifier = read_optional_text(bytes, &mut cursor)?;
    let column_visibility = read_optional_bytes(bytes, &mut cursor)?;
    let value = read_optional_bytes(bytes, &mut cursor)?;
    let timestamp = read_u64(bytes, &mut cursor)?;

    if cursor != bytes.len() {
        return Err("Serialized Fjall statement has trailing bytes".to_string());
    }

    let mut statement = RyaStatement::new(
        subject,
        predicate,
        RyaType::from_parts(object_data_type, object_data, object_language),
    )
    .with_timestamp(timestamp);
    statement.context = context;
    statement.qualifier = qualifier;
    statement.column_visibility = column_visibility;
    statement.value = value;

    Ok(statement)
}

fn write_text(bytes: &mut Vec<u8>, value: &str) -> Result<(), String> {
    write_bytes(bytes, value.as_bytes())
}

fn write_optional_text(bytes: &mut Vec<u8>, value: Option<&str>) -> Result<(), String> {
    write_optional_bytes(bytes, value.map(str::as_bytes))
}

fn write_optional_bytes(bytes: &mut Vec<u8>, value: Option<&[u8]>) -> Result<(), String> {
    match value {
        Some(value) => write_bytes(bytes, value),
        None => {
            bytes.extend_from_slice(&NONE_FIELD_LEN.to_be_bytes());
            Ok(())
        }
    }
}

fn write_bytes(bytes: &mut Vec<u8>, value: &[u8]) -> Result<(), String> {
    let len = u32::try_from(value.len())
        .map_err(|_| "Fjall statement field exceeds u32 length".to_string())?;
    if len == NONE_FIELD_LEN {
        return Err("Fjall statement field length collides with None sentinel".to_string());
    }
    bytes.extend_from_slice(&len.to_be_bytes());
    bytes.extend_from_slice(value);
    Ok(())
}

fn read_u8(bytes: &[u8], cursor: &mut usize) -> Result<u8, String> {
    let value = *bytes
        .get(*cursor)
        .ok_or_else(|| "Truncated Fjall statement version".to_string())?;
    *cursor += 1;
    Ok(value)
}

fn read_text(bytes: &[u8], cursor: &mut usize) -> Result<String, String> {
    String::from_utf8(read_bytes(bytes, cursor)?)
        .map_err(|e| format!("Serialized Fjall statement text is not UTF-8: {e}"))
}

fn read_optional_text(bytes: &[u8], cursor: &mut usize) -> Result<Option<String>, String> {
    read_optional_bytes(bytes, cursor)?
        .map(String::from_utf8)
        .transpose()
        .map_err(|e| format!("Serialized Fjall statement optional text is not UTF-8: {e}"))
}

fn read_optional_bytes(bytes: &[u8], cursor: &mut usize) -> Result<Option<Vec<u8>>, String> {
    let len = read_len(bytes, cursor)?;
    if len == NONE_FIELD_LEN {
        return Ok(None);
    }
    read_len_bytes(bytes, cursor, len).map(Some)
}

fn read_bytes(bytes: &[u8], cursor: &mut usize) -> Result<Vec<u8>, String> {
    let len = read_len(bytes, cursor)?;
    if len == NONE_FIELD_LEN {
        return Err("Required Fjall statement field is marked None".to_string());
    }
    read_len_bytes(bytes, cursor, len)
}

fn read_len(bytes: &[u8], cursor: &mut usize) -> Result<u32, String> {
    let end = cursor
        .checked_add(4)
        .ok_or_else(|| "Fjall statement cursor overflow".to_string())?;
    let chunk = bytes
        .get(*cursor..end)
        .ok_or_else(|| "Truncated Fjall statement field length".to_string())?;
    *cursor = end;

    Ok(u32::from_be_bytes(chunk.try_into().unwrap()))
}

fn read_len_bytes(bytes: &[u8], cursor: &mut usize, len: u32) -> Result<Vec<u8>, String> {
    let len = usize::try_from(len)
        .map_err(|_| "Fjall statement field length exceeds usize".to_string())?;
    let end = cursor
        .checked_add(len)
        .ok_or_else(|| "Fjall statement cursor overflow".to_string())?;
    let chunk = bytes
        .get(*cursor..end)
        .ok_or_else(|| "Truncated Fjall statement field".to_string())?;
    *cursor = end;

    Ok(chunk.to_vec())
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, String> {
    let end = cursor
        .checked_add(8)
        .ok_or_else(|| "Fjall statement timestamp cursor overflow".to_string())?;
    let chunk = bytes
        .get(*cursor..end)
        .ok_or_else(|| "Truncated Fjall statement timestamp".to_string())?;
    *cursor = end;

    Ok(u64::from_be_bytes(chunk.try_into().unwrap()))
}

pub fn apply_additional_iterators(
    statements: Vec<RyaStatement>,
    pattern: &StatementPattern,
    iterators: &[IteratorSetting],
) -> Result<Vec<RyaStatement>, String> {
    let mut ordered = iterators.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|iterator| iterator.priority);
    let mut current = statements;
    for iterator in ordered {
        if iterator.is_first_entry_in_row() {
            current = first_entry_in_row(current, pattern);
        } else {
            return Err(format!(
                "Unsupported in-memory Fjall iterator: {}",
                iterator.iterator_id
            ));
        }
    }
    Ok(current)
}

fn first_entry_in_row(
    statements: impl IntoIterator<Item = RyaStatement>,
    pattern: &StatementPattern,
) -> Vec<RyaStatement> {
    let mut rows = BTreeMap::<String, RyaStatement>::new();
    for statement in statements {
        rows.entry(core_table_row(&statement, pattern))
            .or_insert(statement);
    }
    rows.into_values().collect()
}

fn core_table_row(statement: &RyaStatement, pattern: &StatementPattern) -> String {
    match (&pattern.subject, &pattern.predicate, &pattern.object) {
        (None, Some(predicate), Some(object)) => {
            format!("po\0{}\0{}", predicate.data(), type_key(object))
        }
        (None, None, Some(object)) => format!("osp\0{}", type_key(object)),
        (Some(subject), _, _) => format!("spo\0{}", subject.data()),
        (None, Some(predicate), None) => format!("po\0{}", predicate.data()),
        (None, None, None) => format!("spo\0{}", statement.subject.data()),
    }
}

fn type_key(value: &RyaType) -> String {
    format!(
        "{}\0{}\0{}",
        value.data_type().unwrap_or_default(),
        value.language().unwrap_or_default(),
        value.data()
    )
}

fn normalize_statement_for_fjall(mut statement: RyaStatement) -> Result<RyaStatement, String> {
    statement.object = normalize_type_for_fjall(&statement.object)?;
    Ok(statement)
}

fn normalize_pattern_for_fjall(pattern: &StatementPattern) -> Result<StatementPattern, String> {
    Ok(StatementPattern {
        subject: pattern.subject.clone(),
        predicate: pattern.predicate.clone(),
        object: pattern
            .object
            .as_ref()
            .map(normalize_type_for_fjall)
            .transpose()?,
        context: pattern.context.clone(),
        qualifier: pattern.qualifier.clone(),
    })
}

fn normalize_type_for_fjall(value: &RyaType) -> Result<RyaType, String> {
    match value.data_type() {
        Some(XSD_DATETIME | XSD_DATE) => Ok(RyaType::custom(
            XSD_DATETIME,
            normalize_xsd_datetime_to_utc(value.data(), FJALL_DATETIME_DEFAULT_OFFSET_MINUTES)?,
        )),
        _ => Ok(value.clone()),
    }
}

fn fjall_statement_key(statement: &RyaStatement) -> String {
    format!(
        "{}\0{}\0{}\0{}\0{}\0{}",
        statement.subject.data(),
        statement.predicate.data(),
        type_key(&statement.object),
        statement
            .context
            .as_ref()
            .map(|context| context.data())
            .unwrap_or_default(),
        statement.qualifier.as_deref().unwrap_or_default(),
        bytes_key(statement.column_visibility.as_deref())
    )
}

fn pattern_storage_key(pattern: &StatementPattern) -> String {
    format!(
        "{}\0{}\0{}\0{}\0{}",
        pattern
            .subject
            .as_ref()
            .map(|subject| subject.data())
            .unwrap_or("*"),
        pattern
            .predicate
            .as_ref()
            .map(|predicate| predicate.data())
            .unwrap_or("*"),
        pattern
            .object
            .as_ref()
            .map(type_key)
            .unwrap_or("*".to_string()),
        pattern
            .context
            .as_ref()
            .map(|context| context.data())
            .unwrap_or("*"),
        pattern.qualifier.as_deref().unwrap_or("*")
    )
}

fn bytes_key(bytes: Option<&[u8]>) -> String {
    bytes
        .map(|bytes| {
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        })
        .unwrap_or_default()
}

fn iterator_key(pattern: &str, i: usize, j: Option<usize>) -> String {
    let with_i = pattern.replacen("%d", &i.to_string(), 1);
    if let Some(j) = j {
        with_i.replacen("%d", &j.to_string(), 1)
    } else {
        with_i
    }
}

#[cfg(test)]
#[path = "tests/fjall_tests.rs"]
mod tests;
