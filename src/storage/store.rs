use std::collections::BTreeMap;

use crate::domain::{RyaIri, RyaStatement};
use crate::inference::InferenceEngine;
use crate::sparql::query::InMemoryRyaDao;
use crate::storage::fjall::{CONF_TBL_PREFIX, FjallRdfConfiguration, FjallRyaDao};
use crate::storage::instance::{RyaDetailsRepository, RyaDetailsToConfiguration};

pub const LEGACY_FJALL_STORE_CONFIG_NAMESPACE: &str = "http://omrya.local/config/store/fjall#";
pub const FJALL_SERVER: &str = "http://omrya.local/config/store/fjall#server";
pub const FJALL_PORT: &str = "http://omrya.local/config/store/fjall#port";
pub const FJALL_INSTANCE: &str = "http://omrya.local/config/store/fjall#instance";
pub const FJALL_USER: &str = "http://omrya.local/config/store/fjall#user";
pub const FJALL_PASSWORD: &str = "http://omrya.local/config/store/fjall#password";
pub const RYA_FJALL_STORE_CONFIG_NAMESPACE: &str = "http://omrya.local/RyaFjallStore/Config#";
pub const RYA_FJALL_STORE_TYPE: &str = "rya:FjallStore";
pub const RYA_FJALL_USER: &str = "http://omrya.local/RyaFjallStore/Config#user";
pub const RYA_FJALL_PASSWORD: &str = "http://omrya.local/RyaFjallStore/Config#password";
pub const RYA_FJALL_INSTANCE: &str = "http://omrya.local/RyaFjallStore/Config#instance";
pub const RYA_FJALL_IS_MOCK: &str = "http://omrya.local/RyaFjallStore/Config#isMock";
pub const RYA_FJALL_STORE_FACTORY_SERVICE: &str = "omrya::storage::store::FjallStoreFactory";
pub const RYA_STORE_FACTORY_CLASS: &str = "omrya::storage::store::RyaStoreFactory";
pub const REMOVED_LEGACY_STORE_TYPE: &str = "removed:cloud-triple-store";
pub const RYA_FJALL_STORE_TEMPLATE_PATH: &str = "data/RyaFjallStore.jsonld";
pub const RYA_FJALL_STORE_SERVICE_PATH: &str =
    "src/storage/store.rs::RYA_FJALL_STORE_FACTORY_SERVICE";
pub const CONF_TABLE_PREFIX: &str = "rya.tableprefix";
pub const CONF_INFER: &str = "query.infer";
pub const CONF_FJALL_USER: &str = "sc.fjall.username";
pub const CONF_FJALL_PASSWORD: &str = "sc.fjall.password";

pub const RYA_FJALL_STORE_TEMPLATE: &str = r#"{
  "@context": {
    "rya": "http://omrya.local/config/store#",
    "rac": "http://omrya.local/RyaFjallStore/Config#"
  },
  "@type": "rya:Repository",
  "rya:repositoryID": "{%Repository ID|RyaFjallStore%}",
  "rya:label": "{%Repository title|RyaFjallStore%}",
  "rya:storeType": "rya:FjallStore",
  "rac:user": "{%Rya Fjall user|root%}",
  "rac:password": "{%Rya Fjall password|root%}",
  "rac:instance": "{%Rya Fjall instance|dev%}",
  "rac:isMock": "{%Rya Fjall is mock|false|true%}"
}"#;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RdfStoreConfig {
    pub server: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub instance: String,
}

impl Default for RdfStoreConfig {
    fn default() -> Self {
        Self {
            server: "stratus13".to_string(),
            port: 2181,
            user: "root".to_string(),
            password: "password".to_string(),
            instance: "stratus".to_string(),
        }
    }
}

impl RdfStoreConfig {
    pub fn parse_literals(values: &BTreeMap<String, String>) -> Result<Self, String> {
        let mut config = Self::default();

        if let Some(server) = values.get(FJALL_SERVER) {
            config.server = server.clone();
        }
        if let Some(port) = values.get(FJALL_PORT) {
            config.port = port
                .parse::<u16>()
                .map_err(|e| format!("Invalid Fjall store port '{port}': {e}"))?;
        }
        if let Some(instance) = values.get(FJALL_INSTANCE) {
            config.instance = instance.clone();
        }
        if let Some(user) = values.get(FJALL_USER) {
            config.user = user.clone();
        }
        if let Some(password) = values.get(FJALL_PASSWORD) {
            config.password = password.clone();
        }

        Ok(config)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FjallStoreConfig {
    pub user: String,
    pub password: String,
    pub instance: String,
    pub is_mock: bool,
}

impl Default for FjallStoreConfig {
    fn default() -> Self {
        Self {
            user: "root".to_string(),
            password: "root".to_string(),
            instance: "dev".to_string(),
            is_mock: false,
        }
    }
}

impl FjallStoreConfig {
    pub fn store_type(&self) -> &'static str {
        RYA_FJALL_STORE_TYPE
    }

    pub fn to_rdf_configuration(&self) -> FjallRdfConfiguration {
        FjallRdfConfiguration::new()
    }

    pub fn export_literals(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            (RYA_FJALL_USER.to_string(), self.user.clone()),
            (RYA_FJALL_PASSWORD.to_string(), self.password.clone()),
            (RYA_FJALL_INSTANCE.to_string(), self.instance.clone()),
            (RYA_FJALL_IS_MOCK.to_string(), self.is_mock.to_string()),
        ])
    }

    pub fn parse_literals(values: &BTreeMap<String, String>) -> Self {
        let mut config = Self::default();
        if let Some(user) = values.get(RYA_FJALL_USER) {
            config.user = user.clone();
        }
        if let Some(password) = values.get(RYA_FJALL_PASSWORD) {
            config.password = password.clone();
        }
        if let Some(instance) = values.get(RYA_FJALL_INSTANCE) {
            config.instance = instance.clone();
        }
        if let Some(is_mock) = values.get(RYA_FJALL_IS_MOCK) {
            config.is_mock = is_mock.eq_ignore_ascii_case("true");
        }
        config
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StoreBackend {
    Fjall {
        instance: String,
        user: String,
        is_mock: bool,
        table_prefix: Option<String>,
        display_query_plan: bool,
    },
}

pub struct RyaStore {
    pub backend: StoreBackend,
    pub fjall_dao: Option<FjallRyaDao>,
    pub inference_engine: Option<InferenceEngine>,
    effective_config: BTreeMap<String, String>,
}

impl RyaStore {
    pub fn has_inference_engine(&self) -> bool {
        self.inference_engine.is_some()
    }

    pub fn inference_engine(&self) -> Option<&InferenceEngine> {
        self.inference_engine.as_ref()
    }

    pub fn inference_engine_mut(&mut self) -> Option<&mut InferenceEngine> {
        self.inference_engine.as_mut()
    }

    pub fn effective_config(&self) -> &BTreeMap<String, String> {
        &self.effective_config
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RdfKvStore {
    conf: FjallRdfConfiguration,
}

impl RdfKvStore {
    pub fn new(conf: FjallRdfConfiguration) -> Self {
        Self { conf }
    }

    pub fn conf(&self) -> &FjallRdfConfiguration {
        &self.conf
    }

    pub fn conf_mut(&mut self) -> &mut FjallRdfConfiguration {
        &mut self.conf
    }

    pub fn get_connection(&self) -> RdfKvStoreConnection {
        RdfKvStoreConnection {
            conf: self.conf.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RdfKvStoreConnection {
    conf: FjallRdfConfiguration,
}

impl RdfKvStoreConnection {
    pub fn conf(&self) -> &FjallRdfConfiguration {
        &self.conf
    }

    pub fn conf_mut(&mut self) -> &mut FjallRdfConfiguration {
        &mut self.conf
    }
}

pub struct FjallStoreFactory;

impl FjallStoreFactory {
    pub const STORE_TYPE: &'static str = RYA_FJALL_STORE_TYPE;

    pub fn get_config() -> FjallStoreConfig {
        FjallStoreConfig::default()
    }

    pub fn get_store(config: &FjallStoreConfig) -> Result<RyaStore, String> {
        let conf = config.to_rdf_configuration();
        let dao = FjallRyaDao::try_new_with_indexers(&conf, Vec::new())?;
        Ok(RyaStore {
            backend: StoreBackend::Fjall {
                instance: config.instance.clone(),
                user: config.user.clone(),
                is_mock: config.is_mock,
                table_prefix: None,
                display_query_plan: true,
            },
            fjall_dao: Some(dao),
            inference_engine: None,
            effective_config: config.export_literals(),
        })
    }

    pub fn store_type() -> &'static str {
        Self::STORE_TYPE
    }
}

pub struct RyaStoreFactory;

impl RyaStoreFactory {
    pub fn get_instance(config: &BTreeMap<String, String>) -> Result<RyaStore, String> {
        Self::get_instance_with_details_repository(config, None)
    }

    pub fn get_instance_with_details_repository(
        config: &BTreeMap<String, String>,
        details_repository: Option<&dyn RyaDetailsRepository>,
    ) -> Result<RyaStore, String> {
        let mut effective_config = config.clone();
        if let Some(details_repository) = details_repository {
            if let Ok(details) = details_repository.get_rya_instance_details() {
                RyaDetailsToConfiguration::add_rya_details_to_configuration(
                    &details,
                    &mut effective_config,
                );
            }
        }

        let use_infer = get_bool(&effective_config, CONF_INFER);
        let table_prefix = effective_config
            .get(CONF_TABLE_PREFIX)
            .or_else(|| effective_config.get(CONF_TBL_PREFIX))
            .cloned()
            .ok_or_else(|| {
                format!(
                    "RyaInstance or table prefix is missing from configuration.{CONF_TBL_PREFIX}"
                )
            })?;
        effective_config.insert(CONF_TABLE_PREFIX.to_string(), table_prefix.clone());
        effective_config.insert(CONF_TBL_PREFIX.to_string(), table_prefix.clone());

        let user = required_config(&effective_config, CONF_FJALL_USER, "Fjall user name")?;
        required_config(
            &effective_config,
            CONF_FJALL_PASSWORD,
            "Fjall user password",
        )?;
        let fjall_config = FjallStoreConfig::default();
        let mut rdf_config = fjall_config.to_rdf_configuration();
        rdf_config.set_table_prefix(&table_prefix);
        let dao = FjallRyaDao::try_new_with_indexers(&rdf_config, Vec::new())?;
        let mut store = RyaStore {
            backend: StoreBackend::Fjall {
                instance: fjall_config.instance,
                user,
                is_mock: fjall_config.is_mock,
                table_prefix: Some(table_prefix),
                display_query_plan: true,
            },
            fjall_dao: Some(dao),
            inference_engine: None,
            effective_config: effective_config.clone(),
        };

        if use_infer {
            let mut inference_engine = InferenceEngine::new();
            inference_engine.init_from_dao(&InMemoryRyaDao::new());
            store.inference_engine = Some(inference_engine);
        }

        Ok(store)
    }
}

fn required_config(
    config: &BTreeMap<String, String>,
    key: &str,
    label: &str,
) -> Result<String, String> {
    config
        .get(key)
        .cloned()
        .ok_or_else(|| format!("{label} is missing from configuration.{key}"))
}

pub struct StoreRegistry;

impl StoreRegistry {
    pub fn keys() -> &'static [&'static str] {
        &[RYA_FJALL_STORE_TYPE]
    }

    pub fn has(store_type: &str) -> bool {
        Self::keys().contains(&store_type)
    }

    pub fn service_class(store_type: &str) -> Option<&'static str> {
        (store_type == RYA_FJALL_STORE_TYPE).then_some(RYA_FJALL_STORE_FACTORY_SERVICE)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemplateVariable {
    pub name: String,
    pub default_value: String,
    pub choices: Vec<String>,
}

pub fn fjall_store_template_variables() -> Vec<TemplateVariable> {
    extract_template_variables(RYA_FJALL_STORE_TEMPLATE)
}

pub fn render_fjall_store_template(values: &BTreeMap<String, String>) -> String {
    render_config_template(RYA_FJALL_STORE_TEMPLATE, values)
}

pub fn parse_fjall_store_template(rendered: &str) -> Result<FjallStoreConfig, String> {
    if !rendered.contains(r#""rya:storeType": "rya:FjallStore""#) {
        return Err("Rendered repository config does not declare rya:FjallStore".to_string());
    }

    let values = BTreeMap::from([
        (
            RYA_FJALL_USER.to_string(),
            jsonld_string(rendered, "rac:user")?,
        ),
        (
            RYA_FJALL_PASSWORD.to_string(),
            jsonld_string(rendered, "rac:password")?,
        ),
        (
            RYA_FJALL_INSTANCE.to_string(),
            jsonld_string(rendered, "rac:instance")?,
        ),
        (
            RYA_FJALL_IS_MOCK.to_string(),
            jsonld_string(rendered, "rac:isMock")?,
        ),
    ]);
    Ok(FjallStoreConfig::parse_literals(&values))
}

fn extract_template_variables(template: &str) -> Vec<TemplateVariable> {
    let mut variables = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find("{%") {
        rest = &rest[start + 2..];
        let Some(end) = rest.find("%}") else {
            break;
        };
        let token = &rest[..end];
        let parts = token.split('|').collect::<Vec<_>>();
        if parts.len() >= 2 {
            variables.push(TemplateVariable {
                name: parts[0].to_string(),
                default_value: parts[1].to_string(),
                choices: parts[2..].iter().map(|choice| choice.to_string()).collect(),
            });
        }
        rest = &rest[end + 2..];
    }
    variables
}

fn render_config_template(template: &str, values: &BTreeMap<String, String>) -> String {
    let mut rendered = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{%") {
        rendered.push_str(&rest[..start]);
        rest = &rest[start + 2..];
        let Some(end) = rest.find("%}") else {
            rendered.push_str("{%");
            rendered.push_str(rest);
            return rendered;
        };
        let token = &rest[..end];
        let parts = token.split('|').collect::<Vec<_>>();
        let replacement = parts
            .first()
            .and_then(|name| values.get(*name))
            .cloned()
            .or_else(|| parts.get(1).map(|default| (*default).to_string()))
            .unwrap_or_default();
        rendered.push_str(&replacement);
        rest = &rest[end + 2..];
    }
    rendered.push_str(rest);
    rendered
}

fn jsonld_string(rendered: &str, key: &str) -> Result<String, String> {
    let marker = format!(r#""{key}": ""#);
    let start = rendered
        .find(&marker)
        .ok_or_else(|| format!("Missing {key} value"))?
        + marker.len();
    let tail = &rendered[start..];
    let end = tail
        .find('"')
        .ok_or_else(|| format!("Unterminated {key} value"))?;
    Ok(tail[..end].to_string())
}

fn get_bool(config: &BTreeMap<String, String>, key: &str) -> bool {
    config
        .get(key)
        .is_some_and(|value| value.eq_ignore_ascii_case("true"))
}

pub fn combine_contexts(contexts: &[RyaIri], statement_context: &RyaIri) -> Vec<RyaIri> {
    let mut combined = Vec::with_capacity(contexts.len() + 1);
    combined.extend_from_slice(contexts);
    combined.push(statement_context.clone());
    combined
}

pub fn insertion_contexts(
    statement_context: Option<&RyaIri>,
    enforced_contexts: &[RyaIri],
) -> Vec<Option<RyaIri>> {
    if enforced_contexts.is_empty() {
        return vec![statement_context.cloned()];
    }

    let mut contexts = enforced_contexts
        .iter()
        .cloned()
        .map(Some)
        .collect::<Vec<_>>();
    if let Some(statement_context) = statement_context {
        contexts.push(Some(statement_context.clone()));
    }
    contexts
}

pub fn add_with_combined_contexts(
    dao: &mut InMemoryRyaDao,
    statement: &RyaStatement,
    enforced_contexts: &[RyaIri],
) {
    for context in insertion_contexts(statement.context.as_ref(), enforced_contexts) {
        let mut clone = statement.clone();
        clone.context = context;
        dao.add(clone);
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct NamespaceManager {
    store: BTreeMap<String, String>,
    cache: BTreeMap<String, String>,
    writes: usize,
}

impl NamespaceManager {
    pub fn add_namespace(&mut self, prefix: impl Into<String>, namespace: impl Into<String>) {
        let prefix = prefix.into();
        let namespace = namespace.into();
        if self
            .get_namespace(&prefix)
            .is_some_and(|saved| saved == namespace)
        {
            return;
        }

        self.cache.insert(prefix.clone(), namespace.clone());
        self.store.insert(prefix, namespace);
        self.writes += 1;
    }

    pub fn get_namespace(&mut self, prefix: &str) -> Option<String> {
        if let Some(namespace) = self.cache.get(prefix) {
            return Some(namespace.clone());
        }
        let namespace = self.store.get(prefix)?.clone();
        self.cache.insert(prefix.to_string(), namespace.clone());
        Some(namespace)
    }

    pub fn remove_namespace(&mut self, prefix: &str) {
        self.cache.remove(prefix);
        if self.store.remove(prefix).is_some() {
            self.writes += 1;
        }
    }

    pub fn iter_namespaces(&self) -> impl Iterator<Item = (&str, &str)> {
        self.store
            .iter()
            .map(|(prefix, namespace)| (prefix.as_str(), namespace.as_str()))
    }

    pub fn write_count(&self) -> usize {
        self.writes
    }
}

#[cfg(test)]
#[path = "../tests/store_tests.rs"]
mod tests;
