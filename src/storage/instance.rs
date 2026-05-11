use std::cell::RefCell;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

pub const CONFIG_USE_GEO: &str = "sc.use_geo";
pub const CONFIG_USE_FREETEXT: &str = "sc.use_freetext";
pub const CONFIG_USE_TEMPORAL: &str = "sc.use_temporal";
pub const CONFIG_USE_ENTITY: &str = "sc.use_entity";
pub const CONFIG_USE_PCJ: &str = "sc.use_pcj";
pub const INSTANCE_DETAILS_TABLE_SUFFIX: &str = "instance_details";
pub const INSTANCE_DETAILS_ROW_ID: &str = "instance metadata";
pub const INSTANCE_DETAILS_COL_FAMILY: &str = "instance";
pub const INSTANCE_DETAILS_COL_QUALIFIER: &str = "details";

const SERIALIZATION_MAGIC: &[u8] = b"OMRYA_RYA_DETAILS_V1\0";

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RyaDetails {
    rya_instance_name: String,
    rya_version: String,
    entity_centric_index_details: EntityCentricIndexDetails,
    geo_index_details: GeoIndexDetails,
    pcj_index_details: PcjIndexDetails,
    temporal_index_details: TemporalIndexDetails,
    free_text_index_details: FreeTextIndexDetails,
    prospector_details: ProspectorDetails,
    join_selectivity_details: JoinSelectivityDetails,
}

impl RyaDetails {
    pub fn builder() -> RyaDetailsBuilder {
        RyaDetailsBuilder::default()
    }

    pub fn get_rya_instance_name(&self) -> &str {
        &self.rya_instance_name
    }

    pub fn get_rya_version(&self) -> &str {
        &self.rya_version
    }

    pub fn get_entity_centric_index_details(&self) -> &EntityCentricIndexDetails {
        &self.entity_centric_index_details
    }

    pub fn get_geo_index_details(&self) -> &GeoIndexDetails {
        &self.geo_index_details
    }

    pub fn get_pcj_index_details(&self) -> &PcjIndexDetails {
        &self.pcj_index_details
    }

    pub fn get_temporal_index_details(&self) -> &TemporalIndexDetails {
        &self.temporal_index_details
    }

    pub fn get_free_text_index_details(&self) -> &FreeTextIndexDetails {
        &self.free_text_index_details
    }

    pub fn get_prospector_details(&self) -> &ProspectorDetails {
        &self.prospector_details
    }

    pub fn get_join_selectivity_details(&self) -> &JoinSelectivityDetails {
        &self.join_selectivity_details
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RyaDetailsBuilder {
    rya_instance_name: Option<String>,
    rya_version: Option<String>,
    entity_centric_index_details: Option<EntityCentricIndexDetails>,
    geo_index_details: Option<GeoIndexDetails>,
    pcj_index_details: Option<PcjIndexDetails>,
    temporal_index_details: Option<TemporalIndexDetails>,
    free_text_index_details: Option<FreeTextIndexDetails>,
    prospector_details: Option<ProspectorDetails>,
    join_selectivity_details: Option<JoinSelectivityDetails>,
}

impl RyaDetailsBuilder {
    pub fn from_details(details: &RyaDetails) -> Self {
        Self {
            rya_instance_name: Some(details.rya_instance_name.clone()),
            rya_version: Some(details.rya_version.clone()),
            entity_centric_index_details: Some(details.entity_centric_index_details.clone()),
            geo_index_details: Some(details.geo_index_details.clone()),
            pcj_index_details: Some(details.pcj_index_details.clone()),
            temporal_index_details: Some(details.temporal_index_details.clone()),
            free_text_index_details: Some(details.free_text_index_details.clone()),
            prospector_details: Some(details.prospector_details.clone()),
            join_selectivity_details: Some(details.join_selectivity_details.clone()),
        }
    }

    pub fn set_rya_instance_name(mut self, instance_name: impl Into<String>) -> Self {
        self.rya_instance_name = Some(instance_name.into());
        self
    }

    pub fn set_rya_version(mut self, version: impl Into<String>) -> Self {
        self.rya_version = Some(version.into());
        self
    }

    pub fn set_entity_centric_index_details(mut self, details: EntityCentricIndexDetails) -> Self {
        self.entity_centric_index_details = Some(details);
        self
    }

    pub fn set_geo_index_details(mut self, details: GeoIndexDetails) -> Self {
        self.geo_index_details = Some(details);
        self
    }

    pub fn set_pcj_index_details(mut self, details: PcjIndexDetails) -> Self {
        self.pcj_index_details = Some(details);
        self
    }

    pub fn set_temporal_index_details(mut self, details: TemporalIndexDetails) -> Self {
        self.temporal_index_details = Some(details);
        self
    }

    pub fn set_free_text_index_details(mut self, details: FreeTextIndexDetails) -> Self {
        self.free_text_index_details = Some(details);
        self
    }

    pub fn set_prospector_details(mut self, details: ProspectorDetails) -> Self {
        self.prospector_details = Some(details);
        self
    }

    pub fn set_join_selectivity_details(mut self, details: JoinSelectivityDetails) -> Self {
        self.join_selectivity_details = Some(details);
        self
    }

    pub fn build(self) -> Result<RyaDetails, RyaDetailsBuildError> {
        Ok(RyaDetails {
            rya_instance_name: required(self.rya_instance_name, "rya_instance_name")?,
            rya_version: required(self.rya_version, "rya_version")?,
            entity_centric_index_details: required(
                self.entity_centric_index_details,
                "entity_centric_index_details",
            )?,
            geo_index_details: required(self.geo_index_details, "geo_index_details")?,
            pcj_index_details: required(self.pcj_index_details, "pcj_index_details")?,
            temporal_index_details: required(
                self.temporal_index_details,
                "temporal_index_details",
            )?,
            free_text_index_details: required(
                self.free_text_index_details,
                "free_text_index_details",
            )?,
            prospector_details: required(self.prospector_details, "prospector_details")?,
            join_selectivity_details: required(
                self.join_selectivity_details,
                "join_selectivity_details",
            )?,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RyaDetailsBuildError {
    pub missing_field: &'static str,
}

impl std::fmt::Display for RyaDetailsBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "missing RyaDetails field: {}", self.missing_field)
    }
}

impl std::error::Error for RyaDetailsBuildError {}

fn required<T>(value: Option<T>, field: &'static str) -> Result<T, RyaDetailsBuildError> {
    value.ok_or(RyaDetailsBuildError {
        missing_field: field,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigurationOverwrite {
    pub field: &'static str,
    pub configured_value: String,
    pub admin_value: bool,
}

pub struct RyaDetailsToConfiguration;

impl RyaDetailsToConfiguration {
    pub fn add_rya_details_to_configuration(
        details: &RyaDetails,
        conf: &mut BTreeMap<String, String>,
    ) -> Vec<ConfigurationOverwrite> {
        let mut overwrites = Vec::new();
        check_and_set(
            conf,
            CONFIG_USE_ENTITY,
            details.get_entity_centric_index_details().is_enabled(),
            &mut overwrites,
        );
        check_and_set(
            conf,
            CONFIG_USE_FREETEXT,
            details.get_free_text_index_details().is_enabled(),
            &mut overwrites,
        );
        check_and_set(
            conf,
            CONFIG_USE_GEO,
            details.get_geo_index_details().is_enabled(),
            &mut overwrites,
        );
        check_and_set(
            conf,
            CONFIG_USE_TEMPORAL,
            details.get_temporal_index_details().is_enabled(),
            &mut overwrites,
        );
        check_and_set(
            conf,
            CONFIG_USE_PCJ,
            details.get_pcj_index_details().is_enabled(),
            &mut overwrites,
        );
        overwrites
    }
}

fn check_and_set(
    conf: &mut BTreeMap<String, String>,
    field: &'static str,
    value: bool,
    overwrites: &mut Vec<ConfigurationOverwrite>,
) {
    match conf.get(field).cloned() {
        Some(configured_value) => {
            if parse_bool_flag(&configured_value) != value {
                overwrites.push(ConfigurationOverwrite {
                    field,
                    configured_value,
                    admin_value: value,
                });
                conf.insert(field.to_string(), value.to_string());
            }
        }
        None => {
            conf.insert(field.to_string(), value.to_string());
        }
    }
}

fn parse_bool_flag(value: &str) -> bool {
    value.eq_ignore_ascii_case("true")
}

macro_rules! bool_details {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, PartialEq, Hash)]
        pub struct $name {
            enabled: bool,
        }

        impl $name {
            pub fn new(enabled: bool) -> Self {
                Self { enabled }
            }

            pub fn is_enabled(&self) -> bool {
                self.enabled
            }
        }
    };
}

bool_details!(EntityCentricIndexDetails);
bool_details!(GeoIndexDetails);
bool_details!(TemporalIndexDetails);
bool_details!(FreeTextIndexDetails);

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct PcjIndexDetails {
    enabled: bool,
    fluo_details: Option<FluoDetails>,
    pcj_details: BTreeMap<String, PcjDetails>,
}

impl PcjIndexDetails {
    pub fn builder() -> PcjIndexDetailsBuilder {
        PcjIndexDetailsBuilder::default()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn get_fluo_details(&self) -> Option<&FluoDetails> {
        self.fluo_details.as_ref()
    }

    pub fn get_pcj_details(&self) -> &BTreeMap<String, PcjDetails> {
        &self.pcj_details
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PcjIndexDetailsBuilder {
    enabled: Option<bool>,
    fluo_details: Option<FluoDetails>,
    pcj_details: Vec<PcjDetails>,
}

impl PcjIndexDetailsBuilder {
    pub fn set_enabled(mut self, enabled: bool) -> Self {
        self.enabled = Some(enabled);
        self
    }

    pub fn set_fluo_details(mut self, details: FluoDetails) -> Self {
        self.fluo_details = Some(details);
        self
    }

    pub fn add_pcj_details(mut self, details: impl Into<Option<PcjDetails>>) -> Self {
        if let Some(details) = details.into() {
            self.pcj_details.push(details);
        }
        self
    }

    pub fn build(self) -> Result<PcjIndexDetails, RyaDetailsBuildError> {
        let mut pcj_details = BTreeMap::new();
        for details in self.pcj_details {
            if pcj_details
                .insert(details.get_id().to_string(), details)
                .is_some()
            {
                return Err(RyaDetailsBuildError {
                    missing_field: "pcj_index_details.pcj_details.duplicate_id",
                });
            }
        }

        Ok(PcjIndexDetails {
            enabled: required(self.enabled, "pcj_index_details.enabled")?,
            fluo_details: self.fluo_details,
            pcj_details,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct FluoDetails {
    update_app_name: String,
}

impl FluoDetails {
    pub fn new(update_app_name: impl Into<String>) -> Self {
        Self {
            update_app_name: update_app_name.into(),
        }
    }

    pub fn update_app_name(&self) -> &str {
        &self.update_app_name
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct PcjDetails {
    id: String,
    update_strategy: PcjUpdateStrategy,
    last_update_time: Option<i64>,
}

impl PcjDetails {
    pub fn builder() -> PcjDetailsBuilder {
        PcjDetailsBuilder::default()
    }

    pub fn get_id(&self) -> &str {
        &self.id
    }

    pub fn get_update_strategy(&self) -> PcjUpdateStrategy {
        self.update_strategy
    }

    pub fn get_last_update_time(&self) -> Option<i64> {
        self.last_update_time
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PcjDetailsBuilder {
    id: Option<String>,
    update_strategy: Option<PcjUpdateStrategy>,
    last_update_time: Option<i64>,
}

impl PcjDetailsBuilder {
    pub fn set_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    pub fn set_update_strategy(mut self, strategy: PcjUpdateStrategy) -> Self {
        self.update_strategy = Some(strategy);
        self
    }

    pub fn set_last_update_time(mut self, epoch_millis: i64) -> Self {
        self.last_update_time = Some(epoch_millis);
        self
    }

    pub fn build(self) -> Result<PcjDetails, RyaDetailsBuildError> {
        Ok(PcjDetails {
            id: required(self.id, "pcj_details.id")?,
            update_strategy: required(self.update_strategy, "pcj_details.update_strategy")?,
            last_update_time: self.last_update_time,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum PcjUpdateStrategy {
    Batch,
    Incremental,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ProspectorDetails {
    last_updated: Option<i64>,
}

impl ProspectorDetails {
    pub fn new(last_updated: Option<i64>) -> Self {
        Self { last_updated }
    }

    pub fn get_last_updated(&self) -> Option<i64> {
        self.last_updated
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct JoinSelectivityDetails {
    last_updated: Option<i64>,
}

impl JoinSelectivityDetails {
    pub fn new(last_updated: Option<i64>) -> Self {
        Self { last_updated }
    }

    pub fn get_last_updated(&self) -> Option<i64> {
        self.last_updated
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RyaDetailsSerializer;

impl RyaDetailsSerializer {
    pub fn serialize(&self, details: &RyaDetails) -> Vec<u8> {
        let mut bytes = SERIALIZATION_MAGIC.to_vec();
        write_string(&mut bytes, &details.rya_instance_name);
        write_string(&mut bytes, &details.rya_version);
        write_bool(&mut bytes, details.entity_centric_index_details.enabled);
        write_bool(&mut bytes, details.geo_index_details.enabled);
        write_bool(&mut bytes, details.temporal_index_details.enabled);
        write_bool(&mut bytes, details.free_text_index_details.enabled);
        write_bool(&mut bytes, details.pcj_index_details.enabled);
        write_option_string(
            &mut bytes,
            details
                .pcj_index_details
                .fluo_details
                .as_ref()
                .map(|details| details.update_app_name.as_str()),
        );
        write_u32(
            &mut bytes,
            details.pcj_index_details.pcj_details.len() as u32,
        );
        for pcj in details.pcj_index_details.pcj_details.values() {
            write_string(&mut bytes, &pcj.id);
            bytes.push(match pcj.update_strategy {
                PcjUpdateStrategy::Batch => 0,
                PcjUpdateStrategy::Incremental => 1,
            });
            write_option_i64(&mut bytes, pcj.last_update_time);
        }
        write_option_i64(&mut bytes, details.prospector_details.last_updated);
        write_option_i64(&mut bytes, details.join_selectivity_details.last_updated);
        bytes
    }

    pub fn deserialize(&self, bytes: &[u8]) -> Result<RyaDetails, RyaDetailsRepositoryError> {
        let mut reader = DetailsReader::new(bytes)?;
        let details = RyaDetails {
            rya_instance_name: reader.read_string()?,
            rya_version: reader.read_string()?,
            entity_centric_index_details: EntityCentricIndexDetails::new(reader.read_bool()?),
            geo_index_details: GeoIndexDetails::new(reader.read_bool()?),
            temporal_index_details: TemporalIndexDetails::new(reader.read_bool()?),
            free_text_index_details: FreeTextIndexDetails::new(reader.read_bool()?),
            pcj_index_details: {
                let enabled = reader.read_bool()?;
                let fluo_details = reader.read_option_string()?.map(FluoDetails::new);
                let pcj_count = reader.read_u32()? as usize;
                let mut pcj_details = BTreeMap::new();
                for _ in 0..pcj_count {
                    let id = reader.read_string()?;
                    let update_strategy = match reader.read_u8()? {
                        0 => PcjUpdateStrategy::Batch,
                        1 => PcjUpdateStrategy::Incremental,
                        other => {
                            return Err(RyaDetailsRepositoryError::Serialization(format!(
                                "unknown PCJ update strategy tag: {other}"
                            )));
                        }
                    };
                    let last_update_time = reader.read_option_i64()?;
                    let pcj = PcjDetails {
                        id: id.clone(),
                        update_strategy,
                        last_update_time,
                    };
                    if pcj_details.insert(id.clone(), pcj).is_some() {
                        return Err(RyaDetailsRepositoryError::Serialization(format!(
                            "duplicate PCJ details id: {id}"
                        )));
                    }
                }
                PcjIndexDetails {
                    enabled,
                    fluo_details,
                    pcj_details,
                }
            },
            prospector_details: ProspectorDetails::new(reader.read_option_i64()?),
            join_selectivity_details: JoinSelectivityDetails::new(reader.read_option_i64()?),
        };
        reader.finish()?;
        Ok(details)
    }
}

fn write_bool(bytes: &mut Vec<u8>, value: bool) {
    bytes.push(u8::from(value));
}

fn write_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn write_i64(bytes: &mut Vec<u8>, value: i64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn write_string(bytes: &mut Vec<u8>, value: &str) {
    write_u32(bytes, value.len() as u32);
    bytes.extend_from_slice(value.as_bytes());
}

fn write_option_string(bytes: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            bytes.push(1);
            write_string(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn write_option_i64(bytes: &mut Vec<u8>, value: Option<i64>) {
    match value {
        Some(value) => {
            bytes.push(1);
            write_i64(bytes, value);
        }
        None => bytes.push(0),
    }
}

struct DetailsReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> DetailsReader<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self, RyaDetailsRepositoryError> {
        if !bytes.starts_with(SERIALIZATION_MAGIC) {
            return Err(RyaDetailsRepositoryError::Serialization(
                "wrong type of object was deserialized".to_string(),
            ));
        }
        Ok(Self {
            bytes,
            offset: SERIALIZATION_MAGIC.len(),
        })
    }

    fn read_u8(&mut self) -> Result<u8, RyaDetailsRepositoryError> {
        let value = *self
            .bytes
            .get(self.offset)
            .ok_or_else(|| self.truncated_error())?;
        self.offset += 1;
        Ok(value)
    }

    fn read_bool(&mut self) -> Result<bool, RyaDetailsRepositoryError> {
        match self.read_u8()? {
            0 => Ok(false),
            1 => Ok(true),
            other => Err(RyaDetailsRepositoryError::Serialization(format!(
                "invalid boolean tag: {other}"
            ))),
        }
    }

    fn read_u32(&mut self) -> Result<u32, RyaDetailsRepositoryError> {
        let bytes = self.read_exact(4)?;
        Ok(u32::from_be_bytes(bytes.try_into().expect("four bytes")))
    }

    fn read_i64(&mut self) -> Result<i64, RyaDetailsRepositoryError> {
        let bytes = self.read_exact(8)?;
        Ok(i64::from_be_bytes(bytes.try_into().expect("eight bytes")))
    }

    fn read_string(&mut self) -> Result<String, RyaDetailsRepositoryError> {
        let len = self.read_u32()? as usize;
        let bytes = self.read_exact(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|e| {
            RyaDetailsRepositoryError::Serialization(format!("invalid UTF-8 string: {e}"))
        })
    }

    fn read_option_string(&mut self) -> Result<Option<String>, RyaDetailsRepositoryError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => self.read_string().map(Some),
            other => Err(RyaDetailsRepositoryError::Serialization(format!(
                "invalid optional string tag: {other}"
            ))),
        }
    }

    fn read_option_i64(&mut self) -> Result<Option<i64>, RyaDetailsRepositoryError> {
        match self.read_u8()? {
            0 => Ok(None),
            1 => self.read_i64().map(Some),
            other => Err(RyaDetailsRepositoryError::Serialization(format!(
                "invalid optional timestamp tag: {other}"
            ))),
        }
    }

    fn read_exact(&mut self, len: usize) -> Result<&'a [u8], RyaDetailsRepositoryError> {
        let end = self
            .offset
            .checked_add(len)
            .ok_or_else(|| self.truncated_error())?;
        let bytes = self
            .bytes
            .get(self.offset..end)
            .ok_or_else(|| self.truncated_error())?;
        self.offset = end;
        Ok(bytes)
    }

    fn finish(&self) -> Result<(), RyaDetailsRepositoryError> {
        if self.offset == self.bytes.len() {
            Ok(())
        } else {
            Err(RyaDetailsRepositoryError::Serialization(
                "serialized RyaDetails had trailing bytes".to_string(),
            ))
        }
    }

    fn truncated_error(&self) -> RyaDetailsRepositoryError {
        RyaDetailsRepositoryError::Serialization("truncated serialized RyaDetails".to_string())
    }
}

pub trait RyaDetailsRepository {
    fn is_initialized(&self) -> Result<bool, RyaDetailsRepositoryError>;

    fn initialize(&self, details: RyaDetails) -> Result<(), RyaDetailsRepositoryError>;

    fn get_rya_instance_details(&self) -> Result<RyaDetails, RyaDetailsRepositoryError>;

    fn update(
        &self,
        old_details: &RyaDetails,
        new_details: RyaDetails,
    ) -> Result<(), RyaDetailsRepositoryError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RyaDetailsRepositoryError {
    AlreadyInitialized(String),
    NotInitialized(String),
    ConcurrentUpdate(String),
    Serialization(String),
    Repository(String),
}

impl std::fmt::Display for RyaDetailsRepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyInitialized(message)
            | Self::NotInitialized(message)
            | Self::ConcurrentUpdate(message)
            | Self::Serialization(message)
            | Self::Repository(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for RyaDetailsRepositoryError {}

#[derive(Clone, Debug, Default)]
pub struct InMemoryFjallDetailsConnector {
    tables: Rc<RefCell<BTreeMap<String, BTreeMap<DetailsCell, Vec<u8>>>>>,
}

impl InMemoryFjallDetailsConnector {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn table_exists(&self, table_name: &str) -> bool {
        self.tables.borrow().contains_key(table_name)
    }

    pub fn create_table(&self, table_name: impl Into<String>) {
        self.tables
            .borrow_mut()
            .entry(table_name.into())
            .or_default();
    }

    pub fn table_names(&self) -> Vec<String> {
        self.tables.borrow().keys().cloned().collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct DetailsCell {
    row: String,
    column_family: String,
    column_qualifier: String,
}

impl DetailsCell {
    fn rya_details() -> Self {
        Self {
            row: INSTANCE_DETAILS_ROW_ID.to_string(),
            column_family: INSTANCE_DETAILS_COL_FAMILY.to_string(),
            column_qualifier: INSTANCE_DETAILS_COL_QUALIFIER.to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct FjallRyaInstanceDetailsRepository {
    connector: InMemoryFjallDetailsConnector,
    instance_name: String,
    details_table_name: String,
    serializer: RyaDetailsSerializer,
}

impl FjallRyaInstanceDetailsRepository {
    pub fn new(connector: InMemoryFjallDetailsConnector, instance_name: impl Into<String>) -> Self {
        let instance_name = instance_name.into();
        let details_table_name = format!("{instance_name}{INSTANCE_DETAILS_TABLE_SUFFIX}");
        Self {
            connector,
            instance_name,
            details_table_name,
            serializer: RyaDetailsSerializer,
        }
    }

    pub fn details_table_name(&self) -> &str {
        &self.details_table_name
    }

    fn validate_instance_name(
        &self,
        details: &RyaDetails,
        label: &str,
    ) -> Result<(), RyaDetailsRepositoryError> {
        if details.rya_instance_name == self.instance_name {
            Ok(())
        } else {
            Err(RyaDetailsRepositoryError::Repository(format!(
                "The instance name that was in the provided '{label}' does not match the instance name that this repository is connected to."
            )))
        }
    }
}

impl RyaDetailsRepository for FjallRyaInstanceDetailsRepository {
    fn is_initialized(&self) -> Result<bool, RyaDetailsRepositoryError> {
        Ok(self
            .connector
            .tables
            .borrow()
            .get(&self.details_table_name)
            .and_then(|table| table.get(&DetailsCell::rya_details()))
            .is_some())
    }

    fn initialize(&self, details: RyaDetails) -> Result<(), RyaDetailsRepositoryError> {
        self.validate_instance_name(&details, "details")?;
        if self.is_initialized()? {
            return Err(RyaDetailsRepositoryError::AlreadyInitialized(format!(
                "The repository has already been initialized for the Rya instance named '{}'.",
                self.instance_name
            )));
        }

        let mut tables = self.connector.tables.borrow_mut();
        let table = tables.entry(self.details_table_name.clone()).or_default();
        table.insert(
            DetailsCell::rya_details(),
            self.serializer.serialize(&details),
        );
        Ok(())
    }

    fn get_rya_instance_details(&self) -> Result<RyaDetails, RyaDetailsRepositoryError> {
        let bytes = self
            .connector
            .tables
            .borrow()
            .get(&self.details_table_name)
            .and_then(|table| table.get(&DetailsCell::rya_details()).cloned())
            .ok_or_else(|| {
                RyaDetailsRepositoryError::NotInitialized(format!(
                    "Could not fetch the details for the Rya instance named '{}' because it has not been initialized yet.",
                    self.instance_name
                ))
            })?;
        self.serializer.deserialize(&bytes)
    }

    fn update(
        &self,
        old_details: &RyaDetails,
        new_details: RyaDetails,
    ) -> Result<(), RyaDetailsRepositoryError> {
        self.validate_instance_name(&new_details, "newDetails")?;
        if !self.is_initialized()? {
            return Err(RyaDetailsRepositoryError::NotInitialized(format!(
                "Could not update the details for the Rya instance named '{}' because it has not been initialized yet.",
                self.instance_name
            )));
        }

        let old_details_bytes = self.serializer.serialize(old_details);
        let new_details_bytes = self.serializer.serialize(&new_details);
        let mut tables = self.connector.tables.borrow_mut();
        let current = tables
            .get_mut(&self.details_table_name)
            .and_then(|table| table.get_mut(&DetailsCell::rya_details()))
            .ok_or_else(|| {
                RyaDetailsRepositoryError::NotInitialized(format!(
                    "Could not update the details for the Rya instance named '{}' because it has not been initialized yet.",
                    self.instance_name
                ))
            })?;

        if *current != old_details_bytes {
            return Err(RyaDetailsRepositoryError::ConcurrentUpdate(format!(
                "Could not update the details for the Rya instance named '{}' because the old value is out of date.",
                self.instance_name
            )));
        }

        *current = new_details_bytes;
        Ok(())
    }
}

pub fn hash_code<T: Hash>(value: &T) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
#[path = "../tests/instance_tests.rs"]
mod tests;
