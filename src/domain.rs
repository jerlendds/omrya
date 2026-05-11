use std::cmp::Ordering;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

pub const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
pub const XSD_ANY_URI: &str = "http://www.w3.org/2001/XMLSchema#anyURI";
pub const XSD_DATE: &str = "http://www.w3.org/2001/XMLSchema#date";
pub const XSD_DATETIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";
pub const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct RyaType {
    data_type: Option<String>,
    data: String,
    language: Option<String>,
}

impl RyaType {
    pub fn new(data: impl Into<String>) -> Self {
        Self::with_data_type(XSD_STRING, data)
    }

    pub fn with_data_type(data_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::with_data_type_and_language(data_type, data, None)
    }

    pub fn with_data_type_and_language(
        data_type: impl Into<String>,
        data: impl Into<String>,
        language: Option<String>,
    ) -> Self {
        Self {
            data_type: Some(data_type.into()),
            data: data.into(),
            language,
        }
    }

    pub fn custom(data_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::with_data_type(data_type, data)
    }

    pub fn data_type(&self) -> Option<&str> {
        self.data_type.as_deref()
    }

    pub fn data(&self) -> &str {
        &self.data
    }

    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    pub(crate) fn from_parts(
        data_type: Option<String>,
        data: String,
        language: Option<String>,
    ) -> Self {
        Self {
            data_type,
            data,
            language,
        }
    }

    pub fn java_compare_parts(
        data: Option<&str>,
        data_type: Option<&str>,
        other_data: Option<&str>,
        other_data_type: Option<&str>,
    ) -> Ordering {
        java_nullable_cmp(data, other_data)
            .then_with(|| java_nullable_cmp(data_type, other_data_type))
    }

    pub fn java_equals_parts(
        data: Option<&str>,
        data_type: Option<&str>,
        other_data: Option<&str>,
        other_data_type: Option<&str>,
    ) -> bool {
        data == other_data && data_type == other_data_type
    }
}

fn java_nullable_cmp(left: Option<&str>, right: Option<&str>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

impl Ord for RyaType {
    fn cmp(&self, other: &Self) -> Ordering {
        self.data
            .cmp(&other.data)
            .then_with(|| self.data_type.cmp(&other.data_type))
            .then_with(|| self.language.cmp(&other.language))
    }
}

impl PartialOrd for RyaType {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct RyaIri(RyaType);

impl RyaIri {
    pub fn new(data: impl Into<String>) -> Result<Self, String> {
        let data = data.into();
        validate_iri(&data)?;
        Ok(Self(RyaType::with_data_type(XSD_ANY_URI, data)))
    }

    pub fn from_namespace(
        namespace: impl Into<String>,
        data: impl Into<String>,
    ) -> Result<Self, String> {
        Self::new(format!("{}{}", namespace.into(), data.into()))
    }

    pub fn data(&self) -> &str {
        self.0.data()
    }

    pub fn as_type(&self) -> &RyaType {
        &self.0
    }

    pub(crate) fn into_type(self) -> RyaType {
        self.0
    }
}

impl fmt::Display for RyaIri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.data())
    }
}

fn validate_iri(data: &str) -> Result<(), String> {
    if data.is_empty() {
        return Err("Empty not IRI".to_string());
    }
    if data.contains(':') {
        Ok(())
    } else {
        Err(format!("No local name index in IRI: {data}"))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RyaStatement {
    pub subject: RyaIri,
    pub predicate: RyaIri,
    pub object: RyaType,
    pub context: Option<RyaIri>,
    pub qualifier: Option<String>,
    pub column_visibility: Option<Vec<u8>>,
    pub value: Option<Vec<u8>>,
    pub timestamp: u64,
}

impl RyaStatement {
    pub fn new(subject: RyaIri, predicate: RyaIri, object: RyaType) -> Self {
        Self {
            subject,
            predicate,
            object,
            context: None,
            qualifier: None,
            column_visibility: None,
            value: None,
            timestamp: current_time_millis(),
        }
    }

    pub fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.timestamp = timestamp;
        self
    }
}

fn current_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
#[path = "tests/domain_tests.rs"]
mod tests;
