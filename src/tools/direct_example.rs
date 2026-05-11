use std::collections::BTreeMap;

use crate::indexes::USE_PCJ;
use crate::pcj::indexing::PCJ_STORAGE_TYPE;
use crate::pcj::{InMemoryPcjTables, VariableOrder};
use crate::storage::store::{
    CONF_FJALL_PASSWORD, CONF_FJALL_USER, CONF_INFER, CONF_TABLE_PREFIX, RyaStoreFactory,
};

pub const RYA_DIRECT_EXAMPLE_COMMIT_67: &str = "6d857e4fc09f63f0e66337f8b66d02b60368de3b";
pub const RYA_DIRECT_EXAMPLE_TABLE_PREFIX: &str = "x_test_triplestore_";
pub const RYA_DIRECT_EXAMPLE_PCJ_TABLES: [&str; 2] =
    ["x_test_triplestore_INDEX_1", "x_test_triplestore_INDEX_2"];

pub const QUERY_STRING_1: &str = "SELECT ?e ?c ?l ?o {  ?c a ?e .   ?e <http://www.w3.org/2000/01/rdf-schema#label> ?l .   ?e <uri:talksTo> ?o . }";
pub const QUERY_STRING_2: &str = "SELECT ?e ?c ?l ?o {  ?e a ?c .   ?e <http://www.w3.org/2000/01/rdf-schema#label> ?l .   ?e <uri:talksTo> ?o . }";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectExamplePcjCreation {
    pub infer_requested: bool,
    pub infer_engine_attached: bool,
    pub pcj_enabled_for_temporary_store: bool,
    pub repository_initialized: bool,
    pub connection_opened: bool,
    pub connection_closed: bool,
    pub repository_closed: bool,
    pub pcj_tables: Vec<String>,
}

pub fn create_pcj_for_direct_example(
    parent_conf: &BTreeMap<String, String>,
) -> Result<DirectExamplePcjCreation, String> {
    let mut config = parent_conf.clone();
    config.insert(USE_PCJ.to_string(), "false".to_string());

    let store = RyaStoreFactory::get_instance(&config)?;
    let infer_requested = config
        .get(CONF_INFER)
        .is_some_and(|value| value.eq_ignore_ascii_case("true"));
    let infer_engine_attached = store.has_inference_engine();
    let pcj_enabled_for_temporary_store = config
        .get(USE_PCJ)
        .is_some_and(|value| value.eq_ignore_ascii_case("true"));

    let mut repository = DirectExampleRepository::new();
    let mut connection = None;
    let mut pcj_tables = Vec::new();

    let result = (|| {
        repository.initialize();
        connection = Some(repository.get_connection());

        let mut tables = InMemoryPcjTables::default();
        let order = VariableOrder::new(["e", "c", "l", "o"]);
        for (name, query) in [
            (RYA_DIRECT_EXAMPLE_PCJ_TABLES[0], QUERY_STRING_1),
            (RYA_DIRECT_EXAMPLE_PCJ_TABLES[1], QUERY_STRING_2),
        ] {
            tables.create_pcj_table(name, [order.clone()], query);
            pcj_tables.push(name.to_string());
        }

        Ok::<(), String>(())
    })();

    let connection_opened = connection.is_some();
    let connection_closed = connection
        .as_mut()
        .map(DirectExampleConnection::close)
        .unwrap_or(false);
    let repository_closed = repository.close();

    result?;

    Ok(DirectExamplePcjCreation {
        infer_requested,
        infer_engine_attached,
        pcj_enabled_for_temporary_store,
        repository_initialized: repository.initialized,
        connection_opened,
        connection_closed,
        repository_closed,
        pcj_tables,
    })
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct DirectExampleRepository {
    initialized: bool,
    closed: bool,
}

impl DirectExampleRepository {
    fn new() -> Self {
        Self::default()
    }

    fn initialize(&mut self) {
        self.initialized = true;
    }

    fn get_connection(&self) -> DirectExampleConnection {
        DirectExampleConnection { closed: false }
    }

    fn close(&mut self) -> bool {
        self.closed = true;
        self.closed
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DirectExampleConnection {
    closed: bool,
}

impl DirectExampleConnection {
    fn close(&mut self) -> bool {
        self.closed = true;
        self.closed
    }
}

pub fn direct_example_parent_conf(infer: bool) -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            CONF_TABLE_PREFIX.to_string(),
            RYA_DIRECT_EXAMPLE_TABLE_PREFIX.to_string(),
        ),
        (USE_PCJ.to_string(), "true".to_string()),
        (PCJ_STORAGE_TYPE.to_string(), "FJALL".to_string()),
        (CONF_FJALL_USER.to_string(), "root".to_string()),
        (CONF_FJALL_PASSWORD.to_string(), "".to_string()),
        (CONF_INFER.to_string(), infer.to_string()),
    ])
}

#[cfg(test)]
#[path = "../tests/direct_example_tests.rs"]
mod tests;
