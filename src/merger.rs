use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{RyaIri, RyaStatement};
use crate::fjall::{CONF_QUERY_AUTH, FjallRdfConfiguration};
use crate::fjall_mr::{
    AC_AUTH_PROP, AC_INSTANCE_PROP, AC_MOCK_PROP, AC_PWD_PROP, AC_USERNAME_PROP, AC_COORDINATOR_PROP,
    TABLE_PREFIX_PROPERTY,
};
use crate::pcj;
use crate::query::{InMemoryRyaDao, QueryOptions, StatementPattern};

pub const CHILD_SUFFIX: &str = ".child";
pub const START_TIME_PROP: &str = "tool.start.time";
pub const USE_START_TIME_DIALOG: &str = "dialog";
pub const USE_MERGE_FILE_INPUT: &str = "use.merge.file.input";
pub const MERGE_FILE_INPUT_PATH: &str = "merge.file.input.path";

pub const COPY_TABLE_LIST_PROP: &str = "copy.table.list";
pub const CREATE_CHILD_INSTANCE_TYPE_PROP: &str = "create.child.instance.type";
pub const PARENT_TIME_OFFSET_PROP: &str = "time.offset";
pub const CHILD_TIME_OFFSET_PROP: &str = "time.offset.child";
pub const NTP_SERVER_HOST_PROP: &str = "ntp.server.host";
pub const DEFAULT_TIME_SERVER_HOST: &str = "time-a.nist.gov";
pub const PARENT_TOMCAT_URL_PROP: &str = "tomcat.url";
pub const CHILD_TOMCAT_URL_PROP: &str = "tomcat.url.child";
pub const COPY_RUN_TIME_PROP: &str = "copy.run.time";
pub const USE_NTP_SERVER_PROP: &str = "use.ntp.server";
pub const USE_COPY_FILE_OUTPUT: &str = "use.copy.file.output";
pub const COPY_FILE_OUTPUT_PATH: &str = "copy.file.output.path";
pub const COPY_FILE_OUTPUT_COMPRESSION_TYPE: &str = "copy.file.output.compression.type";
pub const USE_COPY_FILE_OUTPUT_DIRECTORY_CLEAR: &str = "use.copy.file.output.directory.clear";
pub const COPY_FILE_IMPORT_DIRECTORY: &str = "copy.file.import.directory";
pub const USE_COPY_FILE_IMPORT: &str = "use.copy.file.import";
pub const QUERY_STRING_PROP: &str = "ac.copy.query";
pub const QUERY_FILE_PROP: &str = "ac.copy.queryfile";

pub const PARENT_USER_NAME: &str = "parent_user";
pub const PARENT_PASSWORD: &str = "parent_pwd";
pub const PARENT_INSTANCE: &str = "parent_instance";
pub const PARENT_TABLE_PREFIX: &str = "pt_";
pub const PARENT_AUTH: &str = "parent_auth";
pub const CHILD_USER_NAME: &str = "child_user";
pub const CHILD_PASSWORD: &str = "child_pwd";
pub const CHILD_INSTANCE: &str = "child_instance";
pub const CHILD_TABLE_PREFIX: &str = "ct_";
pub const CHILD_AUTH: &str = "child_auth";

const CONFIG_USE_MOCK_INSTANCE: &str = "sc.use_mock_instance";
const CONFIG_FJALL_INSTANCE: &str = "sc.fjall.instancename";
const CONFIG_FJALL_USER: &str = "sc.fjall.username";
const CONFIG_FJALL_PASSWORD: &str = "sc.fjall.password";
const CONFIG_FJALL_AUTHS: &str = "sc.fjall.auths";
const CONFIG_FJALL_COORDINATORS: &str = "sc.fjall.coordinators";
const CONFIG_FJALL_TBL_PREFIX: &str = "sc.fjall.tableprefix";
const RDF_TBL_PREFIX: &str = "rdf.tablePrefix";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CalendarUnit {
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}

impl CalendarUnit {
    pub fn milliseconds(self) -> i64 {
        match self {
            Self::Millisecond => 1,
            Self::Second => 1_000,
            Self::Minute => 60 * Self::Second.milliseconds(),
            Self::Hour => 60 * Self::Minute.milliseconds(),
            Self::Day => 24 * Self::Hour.milliseconds(),
            Self::Week => 7 * Self::Day.milliseconds(),
            Self::Month => 31 * Self::Day.milliseconds(),
            Self::Year => 365 * Self::Day.milliseconds(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Occurrence {
    Before,
    After,
}

impl Occurrence {
    fn sign(self) -> i64 {
        match self {
            Self::Before => -1,
            Self::After => 1,
        }
    }
}

pub fn time_from(
    time_millis: i64,
    duration: i64,
    unit: CalendarUnit,
    occurrence: Occurrence,
) -> i64 {
    time_millis + occurrence.sign() * duration * unit.milliseconds()
}

pub fn day_before(time_millis: i64) -> i64 {
    time_from(time_millis, 1, CalendarUnit::Day, Occurrence::Before)
}

pub fn month_before(time_millis: i64) -> i64 {
    time_from(time_millis, 1, CalendarUnit::Month, Occurrence::Before)
}

pub fn create_rya_uri(local_name: &str) -> RyaIri {
    RyaIri::from_namespace("#:", local_name).expect("test namespace is a valid Rya URI")
}

pub fn create_rya_statement(
    subject: &str,
    predicate: &str,
    object: &str,
    timestamp: Option<u64>,
) -> RyaStatement {
    let mut statement = RyaStatement::new(
        create_rya_uri(subject),
        create_rya_uri(predicate),
        create_rya_uri(object).into_type(),
    );
    if let Some(timestamp) = timestamp {
        statement.timestamp = timestamp;
    }
    statement
}

pub fn copy_rya_statement(statement: &RyaStatement) -> RyaStatement {
    statement.clone()
}

pub fn make_argument(key: &str, value: impl ToString) -> String {
    format!("-D{}={}", key, value.to_string())
}

pub fn start_time_string(start_time_millis: u64, dialog_enabled: bool) -> String {
    if dialog_enabled {
        USE_START_TIME_DIALOG.to_string()
    } else {
        start_time_millis.to_string()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ToolConfig {
    values: BTreeMap<String, String>,
}

impl ToolConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.values.insert(key.into(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }
}

pub fn duplicate_key_map() -> BTreeMap<&'static str, Vec<&'static str>> {
    BTreeMap::from([
        (AC_MOCK_PROP, vec![CONFIG_USE_MOCK_INSTANCE]),
        (AC_INSTANCE_PROP, vec![CONFIG_FJALL_INSTANCE]),
        (AC_USERNAME_PROP, vec![CONFIG_FJALL_USER]),
        (AC_PWD_PROP, vec![CONFIG_FJALL_PASSWORD]),
        (AC_AUTH_PROP, vec![CONFIG_FJALL_AUTHS, CONF_QUERY_AUTH]),
        (AC_COORDINATOR_PROP, vec![CONFIG_FJALL_COORDINATORS]),
        (
            TABLE_PREFIX_PROPERTY,
            vec![CONFIG_FJALL_TBL_PREFIX, RDF_TBL_PREFIX],
        ),
        ("ac.mock.child", vec!["sc.use_mock_instance.child"]),
        ("ac.instance.child", vec!["sc.fjall.instancename.child"]),
        ("ac.username.child", vec!["sc.fjall.username.child"]),
        ("ac.pwd.child", vec!["sc.fjall.password.child"]),
        (
            "ac.auth.child",
            vec!["sc.fjall.auths.child", "query.auth.child"],
        ),
        ("ac.coordinator.child", vec!["sc.fjall.coordinators.child"]),
        (
            "rdf.tablePrefix.child",
            vec!["sc.fjall.tableprefix.child", "rdf.tablePrefix.child"],
        ),
    ])
}

pub fn set_duplicate_keys(config: &mut ToolConfig) {
    for (key, duplicate_keys) in duplicate_key_map() {
        if let Some(value) = config.get(key).map(str::to_string) {
            for duplicate_key in duplicate_keys {
                config.set(duplicate_key, value.clone());
            }
        }
    }
}

pub fn set_duplicate_keys_for_property(
    config: &mut ToolConfig,
    property: &'static str,
    value: impl Into<String>,
) {
    let value = value.into();
    config.set(property, value.clone());
    if let Some(duplicate_keys) = duplicate_key_map().get(property) {
        for duplicate_key in duplicate_keys {
            config.set(*duplicate_key, value.clone());
        }
    }
}

pub fn copy_parent_prop_to_child(config: &mut ToolConfig, parent_property_name: &str) {
    let parent_value = config.get(parent_property_name).unwrap_or("").to_string();
    config.set(
        format!("{parent_property_name}{CHILD_SUFFIX}"),
        parent_value,
    );
}

pub fn convert_child_prop_to_parent_prop(
    child_config: &mut ToolConfig,
    parent_config: &ToolConfig,
    parent_property_name: &str,
) {
    let child_value = parent_config
        .get(&format!("{parent_property_name}{CHILD_SUFFIX}"))
        .unwrap_or("")
        .to_string();
    child_config.set(parent_property_name, child_value);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstanceType {
    Mock,
    Mini,
    Distributed,
}

impl InstanceType {
    pub fn as_config_value(self) -> &'static str {
        match self {
            Self::Mock => "MOCK",
            Self::Mini => "MINI",
            Self::Distributed => "DISTRIBUTED",
        }
    }

}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FjallInstanceConfig {
    pub label: String,
    pub is_mock: bool,
    pub should_create_indices: bool,
    pub is_read_only: bool,
    pub initially_exists: bool,
    pub user: String,
    pub password: String,
    pub instance: String,
    pub table_prefix: String,
    pub auth: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FjallDualInstanceDriver {
    pub is_mock: bool,
    pub should_create_indices: bool,
    pub is_parent_read_only: bool,
    pub is_child_read_only: bool,
    pub does_child_initially_exist: bool,
    pub parent: FjallInstanceConfig,
    pub child: FjallInstanceConfig,
}

impl FjallDualInstanceDriver {
    pub fn new(
        is_mock: bool,
        should_create_indices: bool,
        is_parent_read_only: bool,
        is_child_read_only: bool,
        does_child_initially_exist: bool,
    ) -> Self {
        let parent_user = if is_mock { PARENT_USER_NAME } else { "root" };
        let child_user = if is_mock { CHILD_USER_NAME } else { "root" };
        Self {
            is_mock,
            should_create_indices,
            is_parent_read_only,
            is_child_read_only,
            does_child_initially_exist,
            parent: FjallInstanceConfig {
                label: "Parent".to_string(),
                is_mock,
                should_create_indices,
                is_read_only: is_parent_read_only,
                initially_exists: true,
                user: parent_user.to_string(),
                password: PARENT_PASSWORD.to_string(),
                instance: PARENT_INSTANCE.to_string(),
                table_prefix: PARENT_TABLE_PREFIX.to_string(),
                auth: PARENT_AUTH.to_string(),
            },
            child: FjallInstanceConfig {
                label: "Child".to_string(),
                is_mock,
                should_create_indices,
                is_read_only: is_child_read_only,
                initially_exists: does_child_initially_exist,
                user: child_user.to_string(),
                password: CHILD_PASSWORD.to_string(),
                instance: CHILD_INSTANCE.to_string(),
                table_prefix: CHILD_TABLE_PREFIX.to_string(),
                auth: CHILD_AUTH.to_string(),
            },
        }
    }

    pub fn parent_config(&self) -> ToolConfig {
        instance_tool_config(&self.parent)
    }

    pub fn child_config(&self) -> ToolConfig {
        instance_tool_config(&self.child)
    }
}

fn instance_tool_config(instance: &FjallInstanceConfig) -> ToolConfig {
    let mut config = ToolConfig::new();
    config.set(AC_MOCK_PROP, instance.is_mock.to_string());
    config.set(AC_INSTANCE_PROP, instance.instance.clone());
    config.set(AC_USERNAME_PROP, instance.user.clone());
    config.set(AC_PWD_PROP, instance.password.clone());
    config.set(TABLE_PREFIX_PROPERTY, instance.table_prefix.clone());
    config.set(AC_AUTH_PROP, instance.auth.clone());
    config.set(AC_COORDINATOR_PROP, "localhost");
    set_duplicate_keys(&mut config);
    config
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartupScriptKind {
    CopyTool,
    MergeTool,
    BatchCopyTool,
    BatchMergeTool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartupScriptPlatform {
    Unix,
    Windows,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StartupScript {
    pub name: &'static str,
    pub platform: StartupScriptPlatform,
    pub command: &'static str,
    pub launch_command: &'static str,
    pub artifact_pattern: &'static str,
    pub config_path: &'static str,
    pub log_config_path: Option<&'static str>,
}

pub const COPY_COMMAND: &str = "omrya merger copy";
pub const MERGE_COMMAND: &str = "omrya merger merge";

pub fn startup_script(kind: StartupScriptKind, platform: StartupScriptPlatform) -> StartupScript {
    match (kind, platform) {
        (StartupScriptKind::CopyTool, StartupScriptPlatform::Unix) => StartupScript {
            name: "copy_tool.sh",
            platform,
            command: COPY_COMMAND,
            launch_command: COPY_COMMAND,
            artifact_pattern: "omrya-*",
            config_path: "config/copy_tool.toml",
            log_config_path: Some("config/copy_tool_logging.toml"),
        },
        (StartupScriptKind::CopyTool, StartupScriptPlatform::Windows) => StartupScript {
            name: "copy_tool.bat",
            platform,
            command: COPY_COMMAND,
            launch_command: COPY_COMMAND,
            artifact_pattern: "omrya-*",
            config_path: "config/copy_tool.toml",
            log_config_path: Some("config/copy_tool_logging.toml"),
        },
        (StartupScriptKind::MergeTool, StartupScriptPlatform::Unix) => StartupScript {
            name: "merge_tool.sh",
            platform,
            command: MERGE_COMMAND,
            launch_command: MERGE_COMMAND,
            artifact_pattern: "omrya-*",
            config_path: "config/merge_tool.toml",
            log_config_path: Some("config/merge_tool_logging.toml"),
        },
        (StartupScriptKind::MergeTool, StartupScriptPlatform::Windows) => StartupScript {
            name: "merge_tool.bat",
            platform,
            command: MERGE_COMMAND,
            launch_command: MERGE_COMMAND,
            artifact_pattern: "omrya-*",
            config_path: "config/merge_tool.toml",
            log_config_path: Some("config/merge_tool_logging.toml"),
        },
        (StartupScriptKind::BatchCopyTool, StartupScriptPlatform::Unix) => StartupScript {
            name: "batch_copy_tool.sh",
            platform,
            command: COPY_COMMAND,
            launch_command: COPY_COMMAND,
            artifact_pattern: "omrya-*",
            config_path: "config/copy_tool.toml",
            log_config_path: None,
        },
        (StartupScriptKind::BatchCopyTool, StartupScriptPlatform::Windows) => StartupScript {
            name: "batch_copy_tool.bat",
            platform,
            command: COPY_COMMAND,
            launch_command: COPY_COMMAND,
            artifact_pattern: "omrya-*",
            config_path: "config/copy_tool.toml",
            log_config_path: None,
        },
        (StartupScriptKind::BatchMergeTool, StartupScriptPlatform::Unix) => StartupScript {
            name: "batch_merge_tool.sh",
            platform,
            command: MERGE_COMMAND,
            launch_command: MERGE_COMMAND,
            artifact_pattern: "omrya-*",
            config_path: "config/merge_tool.toml",
            log_config_path: None,
        },
        (StartupScriptKind::BatchMergeTool, StartupScriptPlatform::Windows) => StartupScript {
            name: "batch_merge_tool.bat",
            platform,
            command: MERGE_COMMAND,
            launch_command: MERGE_COMMAND,
            artifact_pattern: "omrya-*",
            config_path: "config/merge_tool.toml",
            log_config_path: None,
        },
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct MergeReport {
    pub added_to_parent: usize,
    pub deleted_from_parent: usize,
    pub visibility_updates: usize,
}

pub fn copy_parent_to_child(
    parent: &InMemoryRyaDao,
    child: &mut InMemoryRyaDao,
    start_time_millis: u64,
) -> usize {
    let copied = parent.query(
        &StatementPattern::default(),
        &QueryOptions {
            start_time_millis: Some(start_time_millis.saturating_sub(1)),
            auths: tool_scan_auths(),
            ..QueryOptions::default()
        },
    );
    let count = copied.len();
    for statement in copied {
        upsert_statement(child, statement);
    }
    count
}

pub fn merge_child_into_parent(
    parent: &mut InMemoryRyaDao,
    child: &InMemoryRyaDao,
    start_time_millis: u64,
) -> MergeReport {
    let mut report = MergeReport::default();
    let mut parent_rows = rows_by_identity(parent);
    let child_rows = rows_by_identity(child);
    let mut identities = parent_rows
        .keys()
        .chain(child_rows.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    for identity in std::mem::take(&mut identities) {
        match (
            parent_rows.remove(&identity),
            child_rows.get(&identity).cloned(),
        ) {
            (Some(mut parent_statement), Some(child_statement)) => {
                let parent_visibility = visibility_string(&parent_statement);
                let child_visibility = visibility_string(&child_statement);
                if parent_visibility != child_visibility && !child_visibility.is_empty() {
                    parent.delete_exact(&parent_statement);
                    parent_statement.column_visibility = Some(
                        combine_column_visibilities(&parent_visibility, &child_visibility)
                            .into_bytes(),
                    );
                    upsert_statement(parent, parent_statement);
                    report.visibility_updates += 1;
                }
            }
            (Some(parent_statement), None) => {
                if parent_statement.timestamp < start_time_millis {
                    parent.delete_exact(&parent_statement);
                    report.deleted_from_parent += 1;
                }
            }
            (None, Some(child_statement)) => {
                if child_statement.timestamp >= start_time_millis {
                    upsert_statement(parent, child_statement);
                    report.added_to_parent += 1;
                }
            }
            (None, None) => {}
        }
    }

    report
}

pub fn combine_column_visibilities(parent: &str, child: &str) -> String {
    if parent.is_empty() {
        child.to_string()
    } else if child.is_empty() || parent == child {
        parent.to_string()
    } else {
        format!("({parent})|({child})")
    }
}

pub fn assert_statement_count(
    description: &str,
    expected: usize,
    statement: &RyaStatement,
    dao: &InMemoryRyaDao,
    auths: impl IntoIterator<Item = impl Into<String>>,
) {
    let auths = auths.into_iter().map(Into::into).collect::<BTreeSet<_>>();
    let results = dao.query(
        &StatementPattern::new(
            Some(statement.subject.clone()),
            Some(statement.predicate.clone()),
            Some(statement.object.clone()),
        ),
        &QueryOptions {
            auths,
            ..QueryOptions::default()
        },
    );
    assert_eq!(expected, results.len(), "{description}");
}

pub fn query_auths(config: &FjallRdfConfiguration) -> BTreeSet<String> {
    config
        .get(CONF_QUERY_AUTH)
        .unwrap_or("")
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect()
}

fn upsert_statement(dao: &mut InMemoryRyaDao, statement: RyaStatement) {
    let mut existing = dao.query(
        &StatementPattern::new(
            Some(statement.subject.clone()),
            Some(statement.predicate.clone()),
            Some(statement.object.clone()),
        ),
        &QueryOptions::default(),
    );
    existing.retain(|candidate| same_identity(candidate, &statement));
    for candidate in existing {
        dao.delete_exact(&candidate);
    }
    dao.add(statement);
}

fn rows_by_identity(dao: &InMemoryRyaDao) -> BTreeMap<StatementIdentity, RyaStatement> {
    dao.query(
        &StatementPattern::default(),
        &QueryOptions {
            auths: tool_scan_auths(),
            ..QueryOptions::default()
        },
    )
    .into_iter()
    .map(|statement| (StatementIdentity::from(&statement), statement))
    .collect()
}

fn tool_scan_auths() -> BTreeSet<String> {
    [PARENT_AUTH, CHILD_AUTH]
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn same_identity(left: &RyaStatement, right: &RyaStatement) -> bool {
    StatementIdentity::from(left) == StatementIdentity::from(right)
}

fn visibility_string(statement: &RyaStatement) -> String {
    statement
        .column_visibility
        .as_deref()
        .map(|bytes| String::from_utf8_lossy(bytes).to_string())
        .unwrap_or_default()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StatementIdentity {
    subject: String,
    predicate: String,
    object_data_type: String,
    object_language: String,
    object_data: String,
    context: String,
    qualifier: String,
}

impl From<&RyaStatement> for StatementIdentity {
    fn from(statement: &RyaStatement) -> Self {
        Self {
            subject: statement.subject.data().to_string(),
            predicate: statement.predicate.data().to_string(),
            object_data_type: statement.object.data_type().unwrap_or("").to_string(),
            object_language: statement.object.language().unwrap_or("").to_string(),
            object_data: statement.object.data().to_string(),
            context: statement
                .context
                .as_ref()
                .map(|context| context.data())
                .unwrap_or("")
                .to_string(),
            qualifier: statement.qualifier.clone().unwrap_or_default(),
        }
    }
}

impl Ord for StatementIdentity {
    fn cmp(&self, other: &Self) -> Ordering {
        (
            &self.subject,
            &self.predicate,
            &self.object_data_type,
            &self.object_language,
            &self.object_data,
            &self.context,
            &self.qualifier,
        )
            .cmp(&(
                &other.subject,
                &other.predicate,
                &other.object_data_type,
                &other.object_language,
                &other.object_data,
                &other.context,
                &other.qualifier,
            ))
    }
}

impl PartialOrd for StatementIdentity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub fn visibility_authorized(
    visibility: &str,
    auths: impl IntoIterator<Item = impl Into<String>>,
) -> bool {
    pcj::visibility_allowed(
        visibility,
        &auths.into_iter().map(Into::into).collect::<BTreeSet<_>>(),
    )
}

#[cfg(test)]
#[path = "tests/merger_tests.rs"]
mod tests;
