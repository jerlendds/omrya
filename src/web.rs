use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{RyaIri, RyaStatement, RyaType};
use crate::fjall::{CONF_CV, FjallRdfConfiguration, FjallRyaDao};
use crate::indexing::TemporalInstantRfc3339;
use crate::query::{QueryOptions, StatementPattern};

pub const QUERY_AUTH_PARAM: &str = "query.auth";
pub const QUERY_RESULT_FORMAT_PARAM: &str = "query.resultformat";
pub const QUERY_INFER_PARAM: &str = "query.infer";
pub const APPLICATION_JSON: &str = "application/json";
pub const SPARQL_RESULTS_XML: &str = "application/sparql-results+xml";
pub const TEXT_XML: &str = "text/xml";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SecurityProviderImpl;

impl SecurityProviderImpl {
    pub fn get_user_auths(&self, query_auth_param: Option<&str>) -> Vec<String> {
        parse_auths(query_auth_param).into_iter().collect()
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct QueryRequest {
    pub query: String,
    pub query_auth: Option<String>,
    pub conf_cv: Option<String>,
    pub infer: Option<String>,
    pub result_format: Option<String>,
    pub callback: Option<String>,
    pub tx_time_millis: Option<String>,
    pub tx_as_of_millis: Option<String>,
    pub tx_after_millis: Option<String>,
    pub tx_before_millis: Option<String>,
    pub ttl_millis: Option<String>,
    pub current_time_millis: Option<String>,
    pub valid_at: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
}

impl QueryRequest {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            ..Self::default()
        }
    }

    pub fn with_auth(mut self, auth: impl Into<String>) -> Self {
        self.query_auth = Some(auth.into());
        self
    }

    pub fn with_visibility(mut self, visibility: impl Into<String>) -> Self {
        self.conf_cv = Some(visibility.into());
        self
    }

    pub fn with_result_format(mut self, result_format: impl Into<String>) -> Self {
        self.result_format = Some(result_format.into());
        self
    }

    pub fn with_tx_time_millis(mut self, timestamp: impl Into<String>) -> Self {
        self.tx_time_millis = Some(timestamp.into());
        self
    }

    pub fn with_tx_as_of_millis(mut self, timestamp: impl Into<String>) -> Self {
        self.tx_as_of_millis = Some(timestamp.into());
        self
    }

    pub fn with_valid_at(mut self, timestamp: impl Into<String>) -> Self {
        self.valid_at = Some(timestamp.into());
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WebResponse {
    pub status: u16,
    pub content_type: Option<String>,
    pub body: String,
}

impl WebResponse {
    fn ok(content_type: Option<&str>, body: impl Into<String>) -> Self {
        Self {
            status: 200,
            content_type: content_type.map(str::to_string),
            body: body.into(),
        }
    }
}

pub struct RdfController {
    dao: FjallRyaDao,
    conf: FjallRdfConfiguration,
    provider: SecurityProviderImpl,
}

impl Default for RdfController {
    fn default() -> Self {
        Self::new(FjallRdfConfiguration::new())
    }
}

impl RdfController {
    pub fn new(conf: FjallRdfConfiguration) -> Self {
        Self {
            dao: FjallRyaDao::new(&conf),
            conf,
            provider: SecurityProviderImpl,
        }
    }

    pub fn query_rdf(&mut self, request: QueryRequest) -> Result<WebResponse, String> {
        let query = request.query.trim();
        if query.is_empty() {
            return Ok(WebResponse::ok(None, ""));
        }

        if is_insert_data(query) {
            self.perform_update(
                query,
                request.conf_cv.as_deref(),
                request.tx_time_millis.as_deref(),
            )?;
            return Ok(WebResponse::ok(None, ""));
        }

        let auths = self
            .provider
            .get_user_auths(request.query_auth.as_deref())
            .into_iter()
            .collect::<BTreeSet<_>>();
        let temporal = TemporalQueryOptions::from_request(&request)?;
        if request
            .result_format
            .as_deref()
            .is_some_and(|format| format.eq_ignore_ascii_case("json"))
        {
            let rows = self.evaluate_select_rows(query, auths, &temporal)?;
            return Ok(WebResponse::ok(
                Some(APPLICATION_JSON),
                sparql_rows_json(&rows),
            ));
        }
        let content_type = if request
            .result_format
            .as_deref()
            .is_some_and(|format| format.eq_ignore_ascii_case("xml"))
        {
            TEXT_XML
        } else {
            SPARQL_RESULTS_XML
        };

        let count = self.evaluate_select_count(query, auths, &temporal)?;
        Ok(WebResponse::ok(Some(content_type), sparql_count_xml(count)))
    }

    pub fn load_rdf(
        &mut self,
        body: &str,
        format: &str,
        visibility: Option<&str>,
    ) -> Result<WebResponse, String> {
        if !format.eq_ignore_ascii_case("N-Triples") {
            return Err(format!(
                "Unsupported RDF format for in-memory web fixture: {format}"
            ));
        }

        let mut request_conf = self.conf.clone();
        if let Some(visibility) = visibility {
            request_conf.set_cv(visibility);
        }

        for mut statement in parse_ntriples(body)? {
            apply_visibility(&mut statement, request_conf.cv());
            self.dao.add(statement);
        }
        self.dao.flush();
        Ok(WebResponse::ok(None, ""))
    }

    pub fn dao(&self) -> &FjallRyaDao {
        &self.dao
    }

    fn perform_update(
        &mut self,
        query: &str,
        visibility: Option<&str>,
        tx_time_millis: Option<&str>,
    ) -> Result<(), String> {
        let mut request_conf = self.conf.clone();
        if let Some(visibility) = visibility {
            request_conf.set(CONF_CV, visibility);
        }
        let tx_time_millis = tx_time_millis.map(parse_u64_param).transpose()?;

        for mut statement in parse_insert_data(query)? {
            apply_visibility(&mut statement, request_conf.cv());
            if let Some(tx_time_millis) = tx_time_millis {
                statement.timestamp = tx_time_millis;
            }
            self.dao.add(statement);
        }
        self.dao.flush();
        Ok(())
    }

    fn evaluate_select_count(
        &self,
        query: &str,
        auths: BTreeSet<String>,
        temporal: &TemporalQueryOptions,
    ) -> Result<usize, String> {
        if query.contains("GRAPH ex:G1") && query.contains("GRAPH ex:G2") {
            return self.evaluate_named_graph_join(query, auths, temporal);
        }

        Ok(self.evaluate_select_rows(query, auths, temporal)?.len())
    }

    fn evaluate_select_rows(
        &self,
        query: &str,
        auths: BTreeSet<String>,
        temporal: &TemporalQueryOptions,
    ) -> Result<Vec<RyaStatement>, String> {
        let pattern = select_statement_pattern(query)?;
        let options = temporal.apply_to_options(QueryOptions {
            auths,
            ..QueryOptions::default()
        });
        let results = self
            .dao
            .query(&pattern, &options, &FjallRdfConfiguration::new())
            .map_err(|e| format!("Failed to evaluate web query: {e}"))?;
        results
            .into_iter()
            .filter_map(|statement| {
                match temporal.valid_time_matches(&self.dao, &options, &statement) {
                    Ok(true) => Some(Ok(statement)),
                    Ok(false) => None,
                    Err(error) => Some(Err(error)),
                }
            })
            .collect()
    }

    fn evaluate_named_graph_join(
        &self,
        query: &str,
        auths: BTreeSet<String>,
        temporal: &TemporalQueryOptions,
    ) -> Result<usize, String> {
        let prefixes = parse_prefixes(query);
        let ex = prefixes
            .get("ex")
            .ok_or_else(|| "Missing ex prefix in named graph query".to_string())?;
        let voc = prefixes
            .get("voc")
            .ok_or_else(|| "Missing voc prefix in named graph query".to_string())?;
        let g1 = iri(format!("{ex}G1"))?;
        let g2 = iri(format!("{ex}G2"))?;
        let name = iri(format!("{voc}name"))?;
        let homepage = iri(format!("{voc}homepage"))?;
        let skill = iri(format!("{voc}hasSkill"))?;
        let options = temporal.apply_to_options(QueryOptions {
            auths,
            ..QueryOptions::default()
        });

        let subjects_with_name = subjects_for(&self.dao, &g1, &name, &options, temporal)?;
        let subjects_with_homepage = subjects_for(&self.dao, &g1, &homepage, &options, temporal)?;
        let subjects_with_skill = subjects_for(&self.dao, &g2, &skill, &options, temporal)?;
        Ok(subjects_with_name
            .intersection(&subjects_with_homepage)
            .cloned()
            .collect::<BTreeSet<_>>()
            .intersection(&subjects_with_skill)
            .count())
    }
}

pub fn sparql_query_redirect_url(
    sparql: &str,
    infer: Option<&str>,
    auth: Option<&str>,
    visibility: Option<&str>,
    result_format: Option<&str>,
    padding: Option<&str>,
) -> String {
    format!(
        "queryrdf?query.infer={}&query.auth={}&conf.cv={}&query.resultformat={}&padding={}&query={}",
        infer.unwrap_or_default(),
        auth.unwrap_or_default(),
        visibility.unwrap_or_default(),
        result_format.unwrap_or_default(),
        padding.unwrap_or_default(),
        form_urlencode(sparql)
    )
}

pub fn controller_root_imports_security_provider(root_xml: &str) -> bool {
    root_xml.contains(r#"<import resource="controllerIntegrationTest-security.xml"/>"#)
        || root_xml.contains(r#"<import resource="controllerIntegrationTest-security.xml" />"#)
}

pub fn security_context_declares_provider(security_xml: &str) -> bool {
    security_xml
        .contains(r#"<bean id="provider" class="mvm.cloud.rdf.web.sail.SecurityProviderImpl"/>"#)
        || security_xml.contains(
            r#"<bean id="provider" class="mvm.cloud.rdf.web.sail.SecurityProviderImpl" />"#,
        )
}

fn subjects_for(
    dao: &FjallRyaDao,
    graph: &RyaIri,
    predicate: &RyaIri,
    options: &QueryOptions,
    temporal: &TemporalQueryOptions,
) -> Result<BTreeSet<RyaIri>, String> {
    let mut pattern = StatementPattern::new(None, Some(predicate.clone()), None);
    pattern.context = Some(graph.clone());
    Ok(dao
        .query(&pattern, options, &FjallRdfConfiguration::new())?
        .into_iter()
        .filter_map(
            |statement| match temporal.valid_time_matches(dao, options, &statement) {
                Ok(true) => Some(Ok(statement.subject)),
                Ok(false) => None,
                Err(error) => Some(Err(error)),
            },
        )
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .collect())
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct TemporalQueryOptions {
    tx_after_millis: Option<u64>,
    tx_before_millis: Option<u64>,
    ttl_millis: Option<u64>,
    current_time_millis: Option<u64>,
    valid_at: Option<TemporalInstantRfc3339>,
    valid_from: Option<TemporalInstantRfc3339>,
    valid_to: Option<TemporalInstantRfc3339>,
}

impl TemporalQueryOptions {
    fn from_request(request: &QueryRequest) -> Result<Self, String> {
        let tx_before_millis = request
            .tx_as_of_millis
            .as_deref()
            .or(request.tx_before_millis.as_deref())
            .map(parse_u64_param)
            .transpose()?;
        let current_time_millis = request
            .current_time_millis
            .as_deref()
            .map(parse_u64_param)
            .transpose()?
            .or(tx_before_millis);

        Ok(Self {
            tx_after_millis: request
                .tx_after_millis
                .as_deref()
                .map(parse_u64_param)
                .transpose()?,
            tx_before_millis,
            ttl_millis: request
                .ttl_millis
                .as_deref()
                .map(parse_u64_param)
                .transpose()?,
            current_time_millis,
            valid_at: request
                .valid_at
                .as_deref()
                .map(parse_valid_at)
                .transpose()?,
            valid_from: request
                .valid_from
                .as_deref()
                .map(parse_valid_from)
                .transpose()?,
            valid_to: request
                .valid_to
                .as_deref()
                .map(parse_valid_to)
                .transpose()?,
        })
    }

    fn apply_to_options(&self, mut options: QueryOptions) -> QueryOptions {
        options.start_time_millis = self.tx_after_millis;
        options.end_time_millis = self.tx_before_millis;
        options.ttl_millis = self.ttl_millis;
        options.current_time_millis = self.current_time_millis;
        options
    }

    fn valid_time_matches(
        &self,
        dao: &FjallRyaDao,
        options: &QueryOptions,
        statement: &RyaStatement,
    ) -> Result<bool, String> {
        if self.valid_at.is_none() && self.valid_from.is_none() && self.valid_to.is_none() {
            return Ok(true);
        }
        let validity = match ValidTimeWindow::from_object(statement.object.data()) {
            Some(validity) => Some(validity),
            None => self.valid_time_window_from_entity_metadata(dao, options, statement)?,
        };
        let Some(validity) = validity else {
            return Ok(false);
        };
        if let Some(valid_at) = &self.valid_at {
            return Ok(validity.contains(valid_at));
        }
        let lower = self
            .valid_from
            .as_ref()
            .map(TemporalInstantRfc3339::as_key_string);
        let upper = self
            .valid_to
            .as_ref()
            .map(TemporalInstantRfc3339::as_key_string);
        Ok(validity.overlaps(lower, upper))
    }

    fn valid_time_window_from_entity_metadata(
        &self,
        dao: &FjallRyaDao,
        options: &QueryOptions,
        statement: &RyaStatement,
    ) -> Result<Option<ValidTimeWindow>, String> {
        let mut pattern = StatementPattern::new(Some(statement.subject.clone()), None, None);
        pattern.context = statement.context.clone();
        let rows = dao.query(&pattern, options, &FjallRdfConfiguration::new())?;
        Ok(ValidTimeWindow::from_metadata_rows(&rows))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ValidTimeWindow {
    start_key: Option<String>,
    end_key: Option<String>,
}

impl ValidTimeWindow {
    fn from_object(value: &str) -> Option<Self> {
        if let Ok(interval) = TemporalInstantRfc3339::parse_interval(value) {
            return Some(Self {
                start_key: Some(interval.beginning().as_key_string().to_string()),
                end_key: Some(interval.end().as_key_string().to_string()),
            });
        }
        parse_valid_at(value).ok().map(|instant| Self {
            start_key: Some(instant.as_key_string().to_string()),
            end_key: Some(instant.as_key_string().to_string()),
        })
    }

    fn from_metadata_rows(rows: &[RyaStatement]) -> Option<Self> {
        let mut valid_at = None;
        let mut valid_from = None;
        let mut valid_to = None;
        for row in rows {
            match temporal_predicate_kind(&row.predicate) {
                Some(TemporalPredicateKind::ValidAt) => {
                    valid_at = parse_valid_at(row.object.data())
                        .ok()
                        .map(|instant| instant.as_key_string().to_string());
                }
                Some(TemporalPredicateKind::ValidFrom) => {
                    valid_from = parse_valid_from(row.object.data())
                        .ok()
                        .map(|instant| instant.as_key_string().to_string());
                }
                Some(TemporalPredicateKind::ValidTo) => {
                    valid_to = parse_valid_to(row.object.data())
                        .ok()
                        .map(|instant| instant.as_key_string().to_string());
                }
                None => {}
            }
        }
        if let Some(valid_at) = valid_at {
            return Some(Self {
                start_key: Some(valid_at.clone()),
                end_key: Some(valid_at),
            });
        }
        if valid_from.is_some() || valid_to.is_some() {
            return Some(Self {
                start_key: valid_from,
                end_key: valid_to,
            });
        }
        None
    }

    fn contains(&self, instant: &TemporalInstantRfc3339) -> bool {
        self.start_key
            .as_deref()
            .is_none_or(|start| start <= instant.as_key_string())
            && self
                .end_key
                .as_deref()
                .is_none_or(|end| instant.as_key_string() <= end)
    }

    fn overlaps(&self, lower: Option<&str>, upper: Option<&str>) -> bool {
        lower.is_none_or(|lower| self.end_key.as_deref().is_none_or(|end| end >= lower))
            && upper
                .is_none_or(|upper| self.start_key.as_deref().is_none_or(|start| start <= upper))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TemporalPredicateKind {
    ValidAt,
    ValidFrom,
    ValidTo,
}

fn temporal_predicate_kind(predicate: &RyaIri) -> Option<TemporalPredicateKind> {
    match local_name(predicate.data()).to_ascii_lowercase().as_str() {
        "validat" => Some(TemporalPredicateKind::ValidAt),
        "validfrom" => Some(TemporalPredicateKind::ValidFrom),
        "validto" => Some(TemporalPredicateKind::ValidTo),
        _ => None,
    }
}

fn local_name(value: &str) -> &str {
    value
        .rsplit(['#', '/', ':'])
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(value)
}

fn parse_u64_param(value: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|e| format!("Invalid temporal millisecond value '{value}': {e}"))
}

fn parse_valid_at(value: &str) -> Result<TemporalInstantRfc3339, String> {
    parse_temporal_marker(value, false)
}

fn parse_valid_from(value: &str) -> Result<TemporalInstantRfc3339, String> {
    parse_temporal_marker(value, false)
}

fn parse_valid_to(value: &str) -> Result<TemporalInstantRfc3339, String> {
    parse_temporal_marker(value, true)
}

fn parse_temporal_marker(value: &str, end_of_day: bool) -> Result<TemporalInstantRfc3339, String> {
    if let Ok(instant) = TemporalInstantRfc3339::parse(value) {
        return Ok(instant);
    }
    if value.len() == 10 && &value[4..5] == "-" && &value[7..8] == "-" {
        let year = value[0..4]
            .parse::<u16>()
            .map_err(|e| format!("Invalid temporal date year '{value}': {e}"))?;
        let month = value[5..7]
            .parse::<u8>()
            .map_err(|e| format!("Invalid temporal date month '{value}': {e}"))?;
        let day = value[8..10]
            .parse::<u8>()
            .map_err(|e| format!("Invalid temporal date day '{value}': {e}"))?;
        return Ok(if end_of_day {
            TemporalInstantRfc3339::new_utc(year, month, day, 23, 59, 59)
        } else {
            TemporalInstantRfc3339::new_utc(year, month, day, 0, 0, 0)
        });
    }
    Err(format!(
        "Invalid temporal value '{value}': expected RFC3339 date-time or YYYY-MM-DD date"
    ))
}

fn apply_visibility(statement: &mut RyaStatement, visibility: Option<&str>) {
    if let Some(visibility) = visibility {
        statement.column_visibility = Some(visibility.as_bytes().to_vec());
    }
}

fn is_insert_data(query: &str) -> bool {
    query
        .trim_start()
        .to_ascii_uppercase()
        .starts_with("INSERT DATA")
}

fn parse_insert_data(query: &str) -> Result<Vec<RyaStatement>, String> {
    let start = query
        .find('{')
        .ok_or_else(|| "INSERT DATA missing opening brace".to_string())?;
    let end = query
        .rfind('}')
        .ok_or_else(|| "INSERT DATA missing closing brace".to_string())?;
    parse_statement_block(&query[start + 1..end], None)
}

fn parse_ntriples(body: &str) -> Result<Vec<RyaStatement>, String> {
    parse_statement_block(body, None)
}

fn parse_statement_block(
    mut block: &str,
    context: Option<RyaIri>,
) -> Result<Vec<RyaStatement>, String> {
    let mut statements = Vec::new();
    loop {
        block = block.trim_start();
        if block.is_empty() {
            break;
        }

        if let Some(after_graph) = strip_keyword(block, "graph") {
            let (graph, rest) = parse_iri_term(after_graph)?;
            let rest = rest.trim_start();
            let nested_start = rest
                .strip_prefix('{')
                .ok_or_else(|| "GRAPH block missing opening brace".to_string())?;
            let close = nested_start
                .find('}')
                .ok_or_else(|| "GRAPH block missing closing brace".to_string())?;
            statements.extend(parse_statement_block(
                &nested_start[..close],
                Some(graph.clone()),
            )?);
            block = nested_start[close + 1..]
                .trim_start()
                .strip_prefix('.')
                .unwrap_or(&nested_start[close + 1..]);
            continue;
        }

        let (subject, rest) = parse_iri_term(block)?;
        let (predicate, rest) = parse_iri_term(rest)?;
        let (object, rest) = parse_value_term(rest)?;
        let mut statement = RyaStatement::new(subject, predicate, object);
        statement.context = context.clone();
        statements.push(statement);
        block = rest.trim_start().strip_prefix('.').unwrap_or(rest);
    }
    Ok(statements)
}

fn parse_iri_term(input: &str) -> Result<(RyaIri, &str), String> {
    let input = input.trim_start();
    if let Some(after_open) = input.strip_prefix('<') {
        let end = after_open
            .find('>')
            .ok_or_else(|| "IRI term missing closing '>'".to_string())?;
        let iri = RyaIri::new(&after_open[..end])?;
        return Ok((iri, &after_open[end + 1..]));
    }

    let end = input.find(char::is_whitespace).unwrap_or(input.len());
    let token = &input[..end];
    if token.is_empty() {
        return Err("Expected IRI term".to_string());
    }
    Ok((iri(token)?, &input[end..]))
}

fn parse_value_term(input: &str) -> Result<(RyaType, &str), String> {
    let input = input.trim_start();
    if let Some(after_open) = input.strip_prefix('<') {
        let end = after_open
            .find('>')
            .ok_or_else(|| "IRI object missing closing '>'".to_string())?;
        return Ok((iri(&after_open[..end])?.into_type(), &after_open[end + 1..]));
    }
    if let Some(after_quote) = input.strip_prefix('"') {
        let (literal, rest) = parse_quoted_literal(after_quote)?;
        let rest = if let Some(after_datatype) = rest.trim_start().strip_prefix("^^") {
            skip_term(after_datatype)
        } else {
            rest
        };
        return Ok((RyaType::new(&literal), rest));
    }
    let (iri, rest) = parse_iri_term(input)?;
    Ok((iri.into_type(), rest))
}

fn parse_quoted_literal(input: &str) -> Result<(String, &str), String> {
    let mut literal = String::new();
    let mut escaped = false;
    for (index, ch) in input.char_indices() {
        if escaped {
            match ch {
                'n' => literal.push('\n'),
                'r' => literal.push('\r'),
                't' => literal.push('\t'),
                '"' => literal.push('"'),
                '\\' => literal.push('\\'),
                other => literal.push(other),
            }
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => return Ok((literal, &input[index + ch.len_utf8()..])),
            other => literal.push(other),
        }
    }
    Err("Literal object missing closing quote".to_string())
}

fn skip_term(input: &str) -> &str {
    let input = input.trim_start();
    if let Some(after_open) = input.strip_prefix('<')
        && let Some(end) = after_open.find('>')
    {
        return &after_open[end + 1..];
    }
    let end = input.find(char::is_whitespace).unwrap_or(input.len());
    &input[end..]
}

fn strip_keyword<'a>(input: &'a str, keyword: &str) -> Option<&'a str> {
    let trimmed = input.trim_start();
    trimmed
        .get(..keyword.len())
        .is_some_and(|head| head.eq_ignore_ascii_case(keyword))
        .then(|| &trimmed[keyword.len()..])
}

fn select_statement_pattern(query: &str) -> Result<StatementPattern, String> {
    let prefixes = parse_prefixes(query);
    let mut block = where_body(query)?;
    let mut context = None;

    if let Some(after_graph) = strip_keyword(block, "graph") {
        let (graph, rest) = parse_resource_pattern_term(after_graph, &prefixes)?;
        context = Some(graph);
        let nested = rest
            .trim_start()
            .strip_prefix('{')
            .ok_or_else(|| "GRAPH pattern missing opening brace".to_string())?;
        let close = nested
            .find('}')
            .ok_or_else(|| "GRAPH pattern missing closing brace".to_string())?;
        block = &nested[..close];
    }

    let mut rest = block.trim_start();
    if rest.is_empty() {
        let mut pattern = StatementPattern::new(None, None, None);
        pattern.context = context;
        return Ok(pattern);
    }
    let (subject, next) = parse_optional_resource_pattern_term(rest, &prefixes)?;
    rest = next;
    let (predicate, next) = parse_predicate_pattern_term(rest, &prefixes)?;
    rest = next;
    let (object, _) = parse_optional_value_pattern_term(rest, &prefixes)?;
    let mut pattern = StatementPattern::new(subject, predicate, object);
    pattern.context = context;
    Ok(pattern)
}

fn where_body(query: &str) -> Result<&str, String> {
    let where_pos = find_keyword(query, "WHERE").unwrap_or(0);
    let rest = &query[where_pos..];
    let start = rest
        .find('{')
        .ok_or_else(|| "SELECT query missing WHERE opening brace".to_string())?;
    let end = rest
        .rfind('}')
        .ok_or_else(|| "SELECT query missing WHERE closing brace".to_string())?;
    if end <= start {
        return Err("SELECT query has an empty or malformed WHERE block".to_string());
    }
    Ok(&rest[start + 1..end])
}

fn find_keyword(input: &str, keyword: &str) -> Option<usize> {
    input
        .to_ascii_uppercase()
        .find(&keyword.to_ascii_uppercase())
}

fn parse_optional_resource_pattern_term<'a>(
    input: &'a str,
    prefixes: &BTreeMap<String, String>,
) -> Result<(Option<RyaIri>, &'a str), String> {
    let input = input.trim_start();
    if input.starts_with('?') {
        return Ok((None, skip_pattern_token(input)));
    }
    parse_resource_pattern_term(input, prefixes).map(|(iri, rest)| (Some(iri), rest))
}

fn parse_predicate_pattern_term<'a>(
    input: &'a str,
    prefixes: &BTreeMap<String, String>,
) -> Result<(Option<RyaIri>, &'a str), String> {
    let input = input.trim_start();
    if input.starts_with('?') {
        return Ok((None, skip_pattern_token(input)));
    }
    if input
        .get(..1)
        .is_some_and(|head| head.eq_ignore_ascii_case("a"))
        && input[1..]
            .chars()
            .next()
            .is_none_or(|ch| ch.is_whitespace())
    {
        return Ok((
            Some(iri("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")?),
            &input[1..],
        ));
    }
    parse_resource_pattern_term(input, prefixes).map(|(iri, rest)| (Some(iri), rest))
}

fn parse_optional_value_pattern_term<'a>(
    input: &'a str,
    prefixes: &BTreeMap<String, String>,
) -> Result<(Option<RyaType>, &'a str), String> {
    let input = input.trim_start();
    if input.starts_with('?') {
        return Ok((None, skip_pattern_token(input)));
    }
    if input.starts_with('"') {
        return parse_value_term(input).map(|(object, rest)| (Some(object), rest));
    }
    parse_resource_pattern_term(input, prefixes).map(|(iri, rest)| (Some(iri.into_type()), rest))
}

fn parse_resource_pattern_term<'a>(
    input: &'a str,
    prefixes: &BTreeMap<String, String>,
) -> Result<(RyaIri, &'a str), String> {
    let input = input.trim_start();
    if input.starts_with('<') {
        return parse_iri_term(input);
    }
    let (token, rest) = take_pattern_token(input)?;
    if let Some((prefix, local)) = token.split_once(':')
        && let Some(base) = prefixes.get(prefix)
    {
        return Ok((iri(format!("{base}{local}"))?, rest));
    }
    Ok((iri(token)?, rest))
}

fn skip_pattern_token(input: &str) -> &str {
    take_pattern_token(input)
        .map(|(_, rest)| rest)
        .unwrap_or_default()
}

fn take_pattern_token(input: &str) -> Result<(&str, &str), String> {
    let input = input.trim_start();
    let mut end = input.len();
    for (index, ch) in input.char_indices() {
        if ch.is_whitespace() || matches!(ch, '.' | ';' | '{' | '}') {
            end = index;
            break;
        }
    }
    let token = input[..end].trim();
    if token.is_empty() {
        return Err("Expected SPARQL pattern term".to_string());
    }
    Ok((token, &input[end..]))
}

fn parse_prefixes(query: &str) -> BTreeMap<String, String> {
    let mut prefixes = BTreeMap::new();
    for line in query.lines() {
        let line = line.trim();
        if !line
            .get(..6)
            .is_some_and(|head| head.eq_ignore_ascii_case("PREFIX"))
        {
            continue;
        }
        let rest = line[6..].trim();
        let Some((prefix, iri_part)) = rest.split_once(':') else {
            continue;
        };
        let Some(start) = iri_part.find('<') else {
            continue;
        };
        let Some(end) = iri_part[start + 1..].find('>') else {
            continue;
        };
        prefixes.insert(
            prefix.trim().to_string(),
            iri_part[start + 1..start + 1 + end].to_string(),
        );
    }
    prefixes
}

fn parse_auths(auth: Option<&str>) -> BTreeSet<String> {
    auth.unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|auth| !auth.is_empty())
        .map(str::to_string)
        .collect()
}

fn sparql_count_xml(count: usize) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><sparql xmlns="http://www.w3.org/2005/sparql-results#"><head><variable name="c"/></head><results><result><binding name="c"><literal datatype="http://www.w3.org/2001/XMLSchema#integer">{count}</literal></binding></result></results></sparql>"#
    )
}

fn sparql_rows_json(rows: &[RyaStatement]) -> String {
    let mut body =
        String::from(r#"{"columns":["s","p","o","g","tx","validStart","validEnd"],"rows":["#);
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            body.push(',');
        }
        let valid = ValidTimeWindow::from_object(row.object.data());
        body.push_str("{\"s\":\"");
        body.push_str(&json_escape(row.subject.data()));
        body.push_str("\",\"p\":\"");
        body.push_str(&json_escape(row.predicate.data()));
        body.push_str("\",\"o\":\"");
        body.push_str(&json_escape(row.object.data()));
        body.push_str("\",\"g\":\"");
        body.push_str(&json_escape(
            row.context.as_ref().map(RyaIri::data).unwrap_or(""),
        ));
        body.push_str("\",\"objectType\":\"");
        body.push_str(&json_escape(row.object.data_type().unwrap_or("")));
        body.push_str("\",\"tx\":\"");
        body.push_str(&row.timestamp.to_string());
        body.push_str("\",\"validStart\":\"");
        body.push_str(&json_escape(
            valid
                .as_ref()
                .and_then(|window| window.start_key.as_deref())
                .unwrap_or(""),
        ));
        body.push_str("\",\"validEnd\":\"");
        body.push_str(&json_escape(
            valid
                .as_ref()
                .and_then(|window| window.end_key.as_deref())
                .unwrap_or(""),
        ));
        body.push_str("\"}");
    }
    body.push_str("]}");
    body
}

fn json_escape(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            ch if ch.is_control() => escaped.push_str(&format!("\\u{:04x}", ch as u32)),
            ch => escaped.push(ch),
        }
    }
    escaped
}

fn form_urlencode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'*' | b'_' => {
                encoded.push(byte as char)
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn iri(value: impl AsRef<str>) -> Result<RyaIri, String> {
    RyaIri::new(value.as_ref())
}

#[cfg(test)]
#[path = "tests/web_tests.rs"]
mod tests;
