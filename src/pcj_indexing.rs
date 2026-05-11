use std::cell::RefCell;
use std::rc::Rc;

use crate::domain::{RyaIri, RyaStatement};
use crate::fjall::{
    CONF_TBL_PREFIX, FjallConnector, FjallIndexer, FjallRdfConfiguration, MultiTableBatchWriter,
};
use crate::fluo_pcj::InMemoryFluoPcjApp;
use crate::pcj::{
    InMemoryPcjTables, PcjCardinalityUpdateStrategy, PcjMetadata, VariableOrder,
    VisibilityBindingSet, make_pcj_table_name, pcj_id_from_table_name,
};

pub const PCJ_STORAGE_TYPE: &str = "rya.indexing.pcj.storageType";
pub const PCJ_UPDATER_TYPE: &str = "rya.indexing.pcj.updaterType";
pub const USE_PCJ_FLUO_UPDATER: &str = "rya.indexing.pcj.updater.fluo";
pub const FLUO_APP_NAME: &str = "rya.indexing.pcj.fluo.fluoAppName";
pub const FJALL_COORDINATORS: &str = "sc.fjall.coordinators";
pub const FJALL_INSTANCE: &str = "sc.fjall.instancename";
pub const FJALL_USERNAME: &str = "sc.fjall.username";
pub const FJALL_PASSWORD: &str = "sc.fjall.password";
pub const STATEMENT_VISIBILITY: &str = "sc.fjall.authorizations";
pub const PRECOMPUTED_JOIN_INDEXER_ID: &str = "omrya::pcj::precomputed_join_indexer";
pub const RYA_INDEXING_PCJ_ARTIFACT_ID: &str = "rya.indexing.pcj";

pub type SharedPcjTables = Rc<RefCell<InMemoryPcjTables>>;
pub type SharedFluoPcjApp = Rc<RefCell<InMemoryFluoPcjApp>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrecomputedJoinStorageType {
    Fjall,
}

impl PrecomputedJoinStorageType {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "FJALL" => Ok(Self::Fjall),
            other => Err(format!("Unsupported PrecomputedJoinStorageType: {other}")),
        }
    }
}

impl std::fmt::Display for PrecomputedJoinStorageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fjall => f.write_str("FJALL"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PrecomputedJoinUpdaterType {
    Fluo,
    NoUpdate,
}

impl PrecomputedJoinUpdaterType {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "FLUO" => Ok(Self::Fluo),
            "NO_UPDATE" => Ok(Self::NoUpdate),
            other => Err(format!("Unsupported PrecomputedJoinUpdaterType: {other}")),
        }
    }
}

impl std::fmt::Display for PrecomputedJoinUpdaterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Fluo => f.write_str("FLUO"),
            Self::NoUpdate => f.write_str("NO_UPDATE"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PrecomputedJoinIndexerConfig {
    config: FjallRdfConfiguration,
}

impl PrecomputedJoinIndexerConfig {
    pub fn new(config: &FjallRdfConfiguration) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub fn pcj_storage_type(&self) -> Result<Option<PrecomputedJoinStorageType>, String> {
        self.config
            .get(PCJ_STORAGE_TYPE)
            .map(PrecomputedJoinStorageType::parse)
            .transpose()
    }

    pub fn pcj_updater_type(&self) -> Result<Option<PrecomputedJoinUpdaterType>, String> {
        self.config
            .get(PCJ_UPDATER_TYPE)
            .map(PrecomputedJoinUpdaterType::parse)
            .transpose()
    }

    pub fn use_fluo_updater(&self) -> bool {
        self.config
            .get(USE_PCJ_FLUO_UPDATER)
            .is_some_and(|value| value.eq_ignore_ascii_case("true"))
    }

    pub fn config(&self) -> &FjallRdfConfiguration {
        &self.config
    }
}

#[derive(Clone, Debug)]
pub struct FjallPcjStorageConfig {
    config: FjallRdfConfiguration,
}

impl FjallPcjStorageConfig {
    pub fn new(config: &FjallRdfConfiguration) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub fn rya_instance_name(&self) -> Result<String, String> {
        self.config
            .get(CONF_TBL_PREFIX)
            .or_else(|| self.config.get("sc.tablePrefix"))
            .map(str::to_string)
            .ok_or_else(|| format!("Missing configuration: {CONF_TBL_PREFIX}"))
    }
}

#[derive(Clone, Debug)]
pub struct FjallPcjStorage {
    tables: SharedPcjTables,
    pub connector: FjallConnector,
    pub rya_instance_name: String,
}

impl FjallPcjStorage {
    pub fn new(
        tables: SharedPcjTables,
        connector: FjallConnector,
        rya_instance_name: impl Into<String>,
    ) -> Self {
        Self {
            tables,
            connector,
            rya_instance_name: rya_instance_name.into(),
        }
    }

    pub fn list_pcjs(&self) -> Vec<String> {
        self.tables.borrow().list_pcj_ids(&self.rya_instance_name)
    }

    pub fn list_pcj_tables(&self) -> Vec<String> {
        self.tables
            .borrow()
            .list_pcj_tables(&self.rya_instance_name)
    }

    pub fn create_pcj(
        &self,
        sparql: &str,
        var_orders: impl IntoIterator<Item = VariableOrder>,
    ) -> String {
        let pcj_id = "generated".to_string();
        let table_name = self.table_name_for_pcj_id(&pcj_id);
        self.tables
            .borrow_mut()
            .create_pcj_table(table_name, var_orders, sparql);
        pcj_id
    }

    pub fn get_pcj_metadata(&self, pcj_id: &str) -> Result<PcjMetadata, String> {
        self.tables
            .borrow()
            .get_pcj_metadata(&self.table_name_for_pcj_id(pcj_id))
    }

    pub fn get_pcj_metadata_by_table(&self, table_name: &str) -> Result<PcjMetadata, String> {
        self.tables.borrow().get_pcj_metadata(table_name)
    }

    pub fn add_results(
        &self,
        pcj_id: &str,
        results: impl IntoIterator<Item = VisibilityBindingSet>,
    ) -> Result<(), String> {
        let strategy = if self.connector.is_mock_instance() {
            PcjCardinalityUpdateStrategy::MockBatchWriter
        } else {
            PcjCardinalityUpdateStrategy::ConditionalWriter
        };
        self.tables
            .borrow_mut()
            .add_visibility_results_with_strategy(
                &self.table_name_for_pcj_id(pcj_id),
                results,
                strategy,
            )
    }

    pub fn purge(&self, pcj_id: &str) -> Result<(), String> {
        self.tables
            .borrow_mut()
            .purge_pcj_table(&self.table_name_for_pcj_id(pcj_id))
    }

    pub fn drop_pcj(&self, pcj_id: &str) -> bool {
        self.tables
            .borrow_mut()
            .drop_pcj_table(&self.table_name_for_pcj_id(pcj_id))
    }

    pub fn close(&self) {}

    pub fn table_name_for_pcj_id(&self, pcj_id: &str) -> String {
        make_pcj_table_name(&self.rya_instance_name, pcj_id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FjallPcjIndexDescriptor {
    pub pcj_id: String,
    pub table_name: String,
    pub sparql: String,
}

pub fn optimizer_pcj_indices(
    storage: &FjallPcjStorage,
    configured_tables: &[String],
) -> Result<Vec<FjallPcjIndexDescriptor>, String> {
    let mut indices = Vec::new();

    if configured_tables.is_empty() {
        for table_name in storage.list_pcj_tables() {
            let pcj_id = pcj_id_from_table_name(&table_name)
                .ok_or_else(|| {
                    format!("PCJ table name does not contain an INDEX_ id: {table_name}")
                })?
                .to_string();
            let metadata = storage.get_pcj_metadata_by_table(&table_name)?;
            indices.push(FjallPcjIndexDescriptor {
                table_name,
                pcj_id,
                sparql: metadata.sparql,
            });
        }
    } else {
        for table_name in configured_tables {
            let pcj_id = pcj_id_from_table_name(table_name)
                .ok_or_else(|| {
                    format!("PCJ table name does not contain an INDEX_ id: {table_name}")
                })?
                .to_string();
            let metadata = storage.get_pcj_metadata(&pcj_id)?;
            indices.push(FjallPcjIndexDescriptor {
                pcj_id,
                table_name: table_name.clone(),
                sparql: metadata.sparql,
            });
        }
    }

    Ok(indices)
}

#[derive(Clone, Debug)]
pub struct FjallPcjStorageSupplier {
    config: Option<FjallRdfConfiguration>,
    connector: Option<FjallConnector>,
    tables: SharedPcjTables,
}

impl FjallPcjStorageSupplier {
    pub fn new(
        config: Option<FjallRdfConfiguration>,
        connector: Option<FjallConnector>,
        tables: SharedPcjTables,
    ) -> Self {
        Self {
            config,
            connector,
            tables,
        }
    }

    pub fn get(&self) -> Result<FjallPcjStorage, String> {
        let config = self.config.as_ref().ok_or_else(|| {
            "Could not create a FjallPcjStorage because the application's configuration has not been provided yet.".to_string()
        })?;
        let indexer_config = PrecomputedJoinIndexerConfig::new(config);
        let storage_type = indexer_config.pcj_storage_type()?.ok_or_else(|| {
            format!("This supplier requires the '{PCJ_STORAGE_TYPE}' value be set to 'FJALL'.")
        })?;
        if storage_type != PrecomputedJoinStorageType::Fjall {
            return Err(format!(
                "This supplier requires the '{PCJ_STORAGE_TYPE}' value be set to 'FJALL'."
            ));
        }
        let connector = self.connector.clone().ok_or_else(|| {
            "The Fjall Connector must be set before initializing the FjallPcjStorage.".to_string()
        })?;
        let rya_instance_name = FjallPcjStorageConfig::new(config).rya_instance_name()?;
        Ok(FjallPcjStorage::new(
            Rc::clone(&self.tables),
            connector,
            rya_instance_name,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct PrecomputedJoinStorageSupplier {
    config: Option<FjallRdfConfiguration>,
    fjall_supplier: FjallPcjStorageSupplier,
}

impl PrecomputedJoinStorageSupplier {
    pub fn new(
        config: Option<FjallRdfConfiguration>,
        fjall_supplier: FjallPcjStorageSupplier,
    ) -> Self {
        Self {
            config,
            fjall_supplier,
        }
    }

    pub fn get(&self) -> Result<FjallPcjStorage, String> {
        let config = self.config.as_ref().ok_or_else(|| {
            "Could not build the PrecomputedJoinStorage until the PrecomputedJoinIndexer has been configured.".to_string()
        })?;
        let storage_type = PrecomputedJoinIndexerConfig::new(config)
            .pcj_storage_type()?
            .ok_or_else(|| {
                format!(
                    "The '{PCJ_STORAGE_TYPE}' property must have one of the following values: [FJALL]"
                )
            })?;
        match storage_type {
            PrecomputedJoinStorageType::Fjall => self.fjall_supplier.get(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FluoPcjUpdaterConfig {
    config: FjallRdfConfiguration,
}

impl FluoPcjUpdaterConfig {
    pub fn new(config: &FjallRdfConfiguration) -> Self {
        Self {
            config: config.clone(),
        }
    }

    pub fn fluo_app_name(&self) -> Option<&str> {
        self.config.get(FLUO_APP_NAME)
    }

    pub fn fjall_coordinators(&self) -> Option<&str> {
        self.config.get(FJALL_COORDINATORS)
    }

    pub fn fluo_coordinators(&self) -> Option<String> {
        self.fjall_coordinators()
            .map(|coordinators| format!("{coordinators}/fluo"))
    }

    pub fn fjall_instance(&self) -> Option<&str> {
        self.config.get(FJALL_INSTANCE)
    }

    pub fn fjall_username(&self) -> Option<&str> {
        self.config.get(FJALL_USERNAME)
    }

    pub fn fjall_password(&self) -> Option<&str> {
        self.config.get(FJALL_PASSWORD)
    }

    pub fn statement_visibility(&self) -> Option<&str> {
        self.config.get(STATEMENT_VISIBILITY)
    }
}

#[derive(Clone, Debug)]
pub struct FluoPcjUpdater {
    app: SharedFluoPcjApp,
    statement_visibility: String,
    delete_warning_printed: bool,
    closed: bool,
}

impl FluoPcjUpdater {
    pub fn new(app: SharedFluoPcjApp, statement_visibility: impl Into<String>) -> Self {
        Self {
            app,
            statement_visibility: statement_visibility.into(),
            delete_warning_printed: false,
            closed: false,
        }
    }

    pub fn add_statements(&mut self, statements: &[RyaStatement]) {
        let visibility = self.statement_visibility.clone();
        self.app.borrow_mut().insert_triples_with_visibility(
            statements
                .iter()
                .cloned()
                .map(|st| (st, visibility.clone())),
        );
    }

    pub fn delete_statements(&mut self, _statements: &[RyaStatement]) {
        self.delete_warning_printed = true;
    }

    pub fn flush(&mut self) {}

    pub fn close(&mut self) {
        self.closed = true;
    }

    pub fn delete_warning_printed(&self) -> bool {
        self.delete_warning_printed
    }

    pub fn closed(&self) -> bool {
        self.closed
    }
}

#[derive(Clone, Debug, Default)]
pub struct NoOpUpdater {
    add_count: usize,
    delete_count: usize,
    flush_count: usize,
    closed: bool,
}

impl NoOpUpdater {
    pub fn add_statements(&mut self, statements: &[RyaStatement]) {
        self.add_count += statements.len();
    }

    pub fn delete_statements(&mut self, statements: &[RyaStatement]) {
        self.delete_count += statements.len();
    }

    pub fn flush(&mut self) {
        self.flush_count += 1;
    }

    pub fn close(&mut self) {
        self.closed = true;
    }

    pub fn add_count(&self) -> usize {
        self.add_count
    }

    pub fn delete_count(&self) -> usize {
        self.delete_count
    }

    pub fn flush_count(&self) -> usize {
        self.flush_count
    }

    pub fn closed(&self) -> bool {
        self.closed
    }
}

#[derive(Clone, Debug, Default)]
pub struct NoOpUpdaterSupplier;

impl NoOpUpdaterSupplier {
    pub fn get(&self) -> NoOpUpdater {
        NoOpUpdater::default()
    }
}

#[derive(Clone, Debug)]
pub enum PcjUpdater {
    Fluo(FluoPcjUpdater),
    NoOp(NoOpUpdater),
}

impl PcjUpdater {
    pub fn add_statements(&mut self, statements: &[RyaStatement]) {
        match self {
            Self::Fluo(updater) => updater.add_statements(statements),
            Self::NoOp(updater) => updater.add_statements(statements),
        }
    }

    pub fn delete_statements(&mut self, statements: &[RyaStatement]) {
        match self {
            Self::Fluo(updater) => updater.delete_statements(statements),
            Self::NoOp(updater) => updater.delete_statements(statements),
        }
    }

    pub fn flush(&mut self) {
        match self {
            Self::Fluo(updater) => updater.flush(),
            Self::NoOp(updater) => updater.flush(),
        }
    }

    pub fn close(&mut self) {
        match self {
            Self::Fluo(updater) => updater.close(),
            Self::NoOp(updater) => updater.close(),
        }
    }

    pub fn is_fluo(&self) -> bool {
        matches!(self, Self::Fluo(_))
    }

    pub fn is_no_op(&self) -> bool {
        matches!(self, Self::NoOp(_))
    }
}

#[derive(Clone, Debug)]
pub struct FluoPcjUpdaterSupplier {
    config: Option<FjallRdfConfiguration>,
    app: SharedFluoPcjApp,
}

impl FluoPcjUpdaterSupplier {
    pub fn new(config: Option<FjallRdfConfiguration>, app: SharedFluoPcjApp) -> Self {
        Self { config, app }
    }

    pub fn get(&self) -> Result<FluoPcjUpdater, String> {
        let config = self.config.as_ref().ok_or_else(|| {
            "Could not create a FluoPcjUpdater because the application's configuration has not been provided yet.".to_string()
        })?;
        let indexer_config = PrecomputedJoinIndexerConfig::new(config);
        let updater_type = indexer_config.pcj_updater_type()?.ok_or_else(|| {
            format!("This supplier requires the '{PCJ_UPDATER_TYPE}' value be set to 'FLUO'.")
        })?;
        if updater_type != PrecomputedJoinUpdaterType::Fluo {
            return Err(format!(
                "This supplier requires the '{PCJ_UPDATER_TYPE}' value be set to 'FLUO'."
            ));
        }

        let fluo_config = FluoPcjUpdaterConfig::new(config);
        required(fluo_config.fluo_app_name(), FLUO_APP_NAME)?;
        required(fluo_config.fluo_coordinators().as_deref(), FJALL_COORDINATORS)?;
        required(fluo_config.fjall_coordinators(), FJALL_COORDINATORS)?;
        required(fluo_config.fjall_instance(), FJALL_INSTANCE)?;
        required(fluo_config.fjall_username(), FJALL_USERNAME)?;
        required(fluo_config.fjall_password(), FJALL_PASSWORD)?;
        let statement_visibility =
            required(fluo_config.statement_visibility(), STATEMENT_VISIBILITY)?.to_string();

        Ok(FluoPcjUpdater::new(
            Rc::clone(&self.app),
            statement_visibility,
        ))
    }
}

#[derive(Clone, Debug)]
pub struct PcjUpdaterSupplierFactory {
    config: Option<FjallRdfConfiguration>,
    fluo_supplier: FluoPcjUpdaterSupplier,
    no_op_supplier: NoOpUpdaterSupplier,
}

impl PcjUpdaterSupplierFactory {
    pub fn new(
        config: Option<FjallRdfConfiguration>,
        fluo_supplier: FluoPcjUpdaterSupplier,
    ) -> Self {
        Self {
            config,
            fluo_supplier,
            no_op_supplier: NoOpUpdaterSupplier,
        }
    }

    pub fn get(&self) -> Result<PcjUpdater, String> {
        let config = self.config.as_ref().ok_or_else(|| {
            "Can not build the PrecomputedJoinUpdater until the PrecomputedJoinIndexer has been configured.".to_string()
        })?;
        let indexer_config = PrecomputedJoinIndexerConfig::new(config);
        if indexer_config.use_fluo_updater() {
            self.fluo_supplier.get().map(PcjUpdater::Fluo)
        } else {
            Ok(PcjUpdater::NoOp(self.no_op_supplier.get()))
        }
    }
}

#[derive(Clone, Debug)]
pub struct PrecomputedJoinUpdaterSupplier {
    config: Option<FjallRdfConfiguration>,
    fluo_supplier: FluoPcjUpdaterSupplier,
}

impl PrecomputedJoinUpdaterSupplier {
    pub fn new(
        config: Option<FjallRdfConfiguration>,
        fluo_supplier: FluoPcjUpdaterSupplier,
    ) -> Self {
        Self {
            config,
            fluo_supplier,
        }
    }

    pub fn get(&self) -> Result<PcjUpdater, String> {
        let config = self.config.as_ref().ok_or_else(|| {
            "Can not build the PrecomputedJoinUpdater until the PrecomputedJoinIndexer has been configured.".to_string()
        })?;
        let updater_type = PrecomputedJoinIndexerConfig::new(config)
            .pcj_updater_type()?
            .ok_or_else(|| {
                format!(
                    "The '{PCJ_UPDATER_TYPE}' property must have one of the following values: [FLUO, NO_UPDATE]"
                )
            })?;
        match updater_type {
            PrecomputedJoinUpdaterType::Fluo => self.fluo_supplier.get().map(PcjUpdater::Fluo),
            PrecomputedJoinUpdaterType::NoUpdate => Ok(PcjUpdater::NoOp(NoOpUpdater::default())),
        }
    }
}

#[derive(Clone, Debug)]
pub struct PrecomputedJoinIndexer {
    conf: Option<FjallRdfConfiguration>,
    connector: Option<FjallConnector>,
    tables: SharedPcjTables,
    fluo_app: SharedFluoPcjApp,
    closed: bool,
    logged_errors: Vec<String>,
}

impl Default for PrecomputedJoinIndexer {
    fn default() -> Self {
        Self::new(
            Rc::new(RefCell::new(InMemoryPcjTables::default())),
            Rc::new(RefCell::new(InMemoryFluoPcjApp::default())),
        )
    }
}

impl PrecomputedJoinIndexer {
    pub fn new(tables: SharedPcjTables, fluo_app: SharedFluoPcjApp) -> Self {
        Self {
            conf: None,
            connector: None,
            tables,
            fluo_app,
            closed: false,
            logged_errors: Vec::new(),
        }
    }

    pub fn logged_errors(&self) -> &[String] {
        &self.logged_errors
    }

    pub fn get_table_name(&mut self) -> Option<String> {
        self.logged_errors.push(
            "PCJ indicies are not stored within a single table, so this method can not be implemented."
                .to_string(),
        );
        None
    }

    pub fn drop_graph(&mut self, _graphs: &[RyaIri]) {
        self.logged_errors.push(
            "PCJ indices do not store Graph metadata, so graph results can not be dropped."
                .to_string(),
        );
    }

    pub fn closed(&self) -> bool {
        self.closed
    }

    fn storage_supplier(&self) -> PrecomputedJoinStorageSupplier {
        PrecomputedJoinStorageSupplier::new(
            self.conf.clone(),
            FjallPcjStorageSupplier::new(
                self.conf.clone(),
                self.connector.clone(),
                Rc::clone(&self.tables),
            ),
        )
    }

    fn updater_factory(&self) -> PcjUpdaterSupplierFactory {
        PcjUpdaterSupplierFactory::new(
            self.conf.clone(),
            FluoPcjUpdaterSupplier::new(self.conf.clone(), Rc::clone(&self.fluo_app)),
        )
    }

    fn storage(&self) -> Result<FjallPcjStorage, String> {
        self.storage_supplier().get()
    }

    fn updater(&self) -> Result<PcjUpdater, String> {
        self.updater_factory().get()
    }
}

impl FjallIndexer for PrecomputedJoinIndexer {
    fn set_conf(&mut self, conf: &FjallRdfConfiguration) -> Result<(), String> {
        self.conf = Some(conf.clone());
        Ok(())
    }

    fn set_connector(&mut self, connector: FjallConnector) -> Result<(), String> {
        self.connector = Some(connector);
        Ok(())
    }

    fn set_multi_table_batch_writer(
        &mut self,
        _writer: MultiTableBatchWriter,
    ) -> Result<(), String> {
        Ok(())
    }

    fn init(&mut self) -> Result<(), String> {
        self.storage()?;
        self.updater()?;
        Ok(())
    }

    fn store_statement(&mut self, statement: &RyaStatement) -> Result<(), String> {
        self.store_statements(std::slice::from_ref(statement))
    }

    fn store_statements(&mut self, statements: &[RyaStatement]) -> Result<(), String> {
        let mut updater = self.updater().map_err(|err| {
            format!("Could not update the PCJs by adding the provided statements: {err}")
        })?;
        updater.add_statements(statements);
        Ok(())
    }

    fn delete_statement(&mut self, statement: &RyaStatement) -> Result<(), String> {
        let mut updater = self.updater().map_err(|err| {
            format!("Could not update the PCJs by removing the provided statement: {err}")
        })?;
        updater.delete_statements(std::slice::from_ref(statement));
        Ok(())
    }

    fn flush(&mut self) -> Result<(), String> {
        let mut updater = self
            .updater()
            .map_err(|err| format!("Could not flush the PCJ Updater: {err}"))?;
        updater.flush();
        Ok(())
    }

    fn destroy(&mut self) -> Result<(), String> {
        if let Ok(storage) = self.storage() {
            storage.close();
        }
        if let Ok(mut updater) = self.updater() {
            updater.close();
        }
        self.closed = true;
        Ok(())
    }

    fn purge(&mut self, _configuration: &FjallRdfConfiguration) -> Result<(), String> {
        match self.storage() {
            Ok(storage) => {
                for pcj_id in storage.list_pcjs() {
                    if let Err(err) = storage.purge(&pcj_id) {
                        self.logged_errors.push(format!(
                            "Could not purge the PCJ index with id: {pcj_id}: {err}"
                        ));
                    }
                }
            }
            Err(err) => self.logged_errors.push(format!(
                "Could not purge the PCJ indicies because they could not be listed: {err}"
            )),
        }
        Ok(())
    }

    fn drop_and_destroy(&mut self) -> Result<(), String> {
        match self.storage() {
            Ok(storage) => {
                for pcj_id in storage.list_pcjs() {
                    if !storage.drop_pcj(&pcj_id) {
                        self.logged_errors
                            .push(format!("Could not delete the PCJ index with id: {pcj_id}"));
                    }
                }
            }
            Err(err) => self.logged_errors.push(format!(
                "Could not delete the PCJ indicies because they could not be listed: {err}"
            )),
        }
        Ok(())
    }
}

fn required<'a>(value: Option<&'a str>, key: &str) -> Result<&'a str, String> {
    value.ok_or_else(|| format!("Missing configuration: {key}"))
}

#[cfg(test)]
#[path = "tests/pcj_indexing_tests.rs"]
mod tests;
