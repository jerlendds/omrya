use std::collections::{BTreeMap, BTreeSet};

use crate::domain::{RyaIri, RyaStatement, RyaType};
use crate::indexing::InMemorySecondaryIndexer;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct QueryOptions {
    pub limit: Option<usize>,
    pub ttl_millis: Option<u64>,
    pub current_time_millis: Option<u64>,
    pub start_time_millis: Option<u64>,
    pub end_time_millis: Option<u64>,
    pub regex_subject: Option<String>,
    pub regex_predicate: Option<String>,
    pub regex_object: Option<String>,
    pub auths: BTreeSet<String>,
}

impl QueryOptions {
    pub fn with_auths(mut self, auths: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.auths = auths.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct StatementPattern {
    pub subject: Option<RyaIri>,
    pub predicate: Option<RyaIri>,
    pub object: Option<RyaType>,
    pub context: Option<RyaIri>,
    pub qualifier: Option<String>,
}

impl StatementPattern {
    pub fn new(
        subject: Option<RyaIri>,
        predicate: Option<RyaIri>,
        object: Option<RyaType>,
    ) -> Self {
        Self {
            subject,
            predicate,
            object,
            context: None,
            qualifier: None,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct InMemoryRyaDao {
    statements: Vec<RyaStatement>,
    namespaces: BTreeMap<String, String>,
    initialized: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JoinStrategy {
    Hash,
    Iterative,
    Merge,
}

impl InMemoryRyaDao {
    pub fn new() -> Self {
        Self {
            initialized: true,
            ..Self::default()
        }
    }

    pub fn add(&mut self, statement: RyaStatement) {
        self.statements.push(statement);
    }

    pub fn add_with_indexers(
        &mut self,
        statement: RyaStatement,
        indexers: &mut [&mut dyn InMemorySecondaryIndexer],
    ) {
        for indexer in indexers.iter_mut() {
            indexer.store_statement(&statement);
        }
        self.add(statement);
    }

    pub fn delete_exact(&mut self, statement: &RyaStatement) {
        self.statements.retain(|stored| stored != statement);
    }

    pub fn delete_exact_with_indexers(
        &mut self,
        statement: &RyaStatement,
        indexers: &mut [&mut dyn InMemorySecondaryIndexer],
    ) {
        self.delete_exact(statement);
        for indexer in indexers.iter_mut() {
            indexer.delete_statement(statement);
        }
    }

    pub fn delete_matching_with_indexers(
        &mut self,
        pattern: &StatementPattern,
        options: &QueryOptions,
        indexers: &mut [&mut dyn InMemorySecondaryIndexer],
    ) -> Vec<RyaStatement> {
        let matched = self.query(pattern, options);
        for statement in &matched {
            self.delete_exact(statement);
            for indexer in indexers.iter_mut() {
                indexer.delete_statement(statement);
            }
        }
        matched
    }

    pub fn query(&self, pattern: &StatementPattern, options: &QueryOptions) -> Vec<RyaStatement> {
        let mut results = Vec::new();
        let current_time = options
            .current_time_millis
            .or_else(|| self.statements.iter().map(|stmt| stmt.timestamp).max());

        for statement in &self.statements {
            if !matches_pattern(statement, pattern) {
                continue;
            }
            if !matches_regexes(statement, options) {
                continue;
            }
            if !visibility_allowed(statement.column_visibility.as_deref(), &options.auths) {
                continue;
            }
            if !ttl_allowed(
                statement.timestamp,
                options.ttl_millis,
                current_time,
                options.start_time_millis,
                options.end_time_millis,
            ) {
                continue;
            }
            results.push(statement.clone());
            if options.limit == Some(results.len()) {
                break;
            }
        }

        results
    }

    pub fn query_with_bindings<B: Clone>(
        &self,
        patterns: &[(StatementPattern, B)],
        options: &QueryOptions,
    ) -> Vec<(RyaStatement, B)> {
        let mut results = Vec::new();
        for (pattern, binding) in patterns {
            for statement in self.query(pattern, options) {
                results.push((statement, binding.clone()));
                if options.limit == Some(results.len()) {
                    return results;
                }
            }
        }
        results
    }

    pub fn property_object_join(
        &self,
        pairs: &[(RyaIri, RyaType)],
        options: &QueryOptions,
        _strategy: JoinStrategy,
    ) -> Vec<RyaIri> {
        let Some((first_predicate, first_object)) = pairs.first() else {
            return Vec::new();
        };

        let mut subjects = self
            .query(
                &StatementPattern::new(
                    None,
                    Some(first_predicate.clone()),
                    Some(first_object.clone()),
                ),
                options,
            )
            .into_iter()
            .map(|stmt| stmt.subject)
            .collect::<BTreeSet<_>>();

        for (predicate, object) in &pairs[1..] {
            let matching = self
                .query(
                    &StatementPattern::new(None, Some(predicate.clone()), Some(object.clone())),
                    options,
                )
                .into_iter()
                .map(|stmt| stmt.subject)
                .collect::<BTreeSet<_>>();
            subjects = subjects.intersection(&matching).cloned().collect();
        }

        subjects.into_iter().collect()
    }

    pub fn predicate_join(
        &self,
        predicates: &[RyaIri],
        options: &QueryOptions,
        _strategy: JoinStrategy,
    ) -> Vec<RyaStatement> {
        let Some(first_predicate) = predicates.first() else {
            return Vec::new();
        };

        let mut subjects = self
            .query(
                &StatementPattern::new(None, Some(first_predicate.clone()), None),
                options,
            )
            .into_iter()
            .map(|stmt| stmt.subject)
            .collect::<BTreeSet<_>>();

        for predicate in &predicates[1..] {
            let matching = self
                .query(
                    &StatementPattern::new(None, Some(predicate.clone()), None),
                    options,
                )
                .into_iter()
                .map(|stmt| stmt.subject)
                .collect::<BTreeSet<_>>();
            subjects = subjects.intersection(&matching).cloned().collect();
        }

        self.query(
            &StatementPattern::new(None, Some(first_predicate.clone()), None),
            options,
        )
        .into_iter()
        .filter(|stmt| subjects.contains(&stmt.subject))
        .collect()
    }

    pub fn add_namespace(&mut self, prefix: impl Into<String>, namespace: impl Into<String>) {
        self.namespaces.insert(prefix.into(), namespace.into());
    }

    pub fn get_namespace(&self, prefix: &str) -> Option<&str> {
        self.namespaces.get(prefix).map(String::as_str)
    }

    pub fn remove_namespace(&mut self, prefix: &str) {
        self.namespaces.remove(prefix);
    }

    pub fn drop_and_destroy(&mut self) {
        self.statements.clear();
        self.namespaces.clear();
        self.initialized = false;
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

fn matches_pattern(statement: &RyaStatement, pattern: &StatementPattern) -> bool {
    pattern
        .subject
        .as_ref()
        .is_none_or(|v| v == &statement.subject)
        && pattern
            .predicate
            .as_ref()
            .is_none_or(|v| v == &statement.predicate)
        && pattern
            .object
            .as_ref()
            .is_none_or(|v| v == &statement.object)
        && pattern.context.as_ref().is_none_or(|v| {
            statement
                .context
                .as_ref()
                .is_some_and(|context| context == v)
        })
        && pattern
            .qualifier
            .as_ref()
            .is_none_or(|v| statement.qualifier.as_ref().is_some_and(|q| q == v))
}

fn matches_regexes(statement: &RyaStatement, options: &QueryOptions) -> bool {
    regex_matches(&options.regex_subject, statement.subject.data())
        && regex_matches(&options.regex_predicate, statement.predicate.data())
        && regex_matches(&options.regex_object, statement.object.data())
}

fn regex_matches(pattern: &Option<String>, value: &str) -> bool {
    let Some(pattern) = pattern else {
        return true;
    };
    let candidates = expand_single_char_classes(pattern);
    candidates.iter().any(|candidate| {
        if let Some(inner) = candidate
            .strip_prefix(".*")
            .and_then(|s| s.strip_suffix(".*"))
        {
            value.contains(inner)
        } else {
            value == candidate
        }
    })
}

fn expand_single_char_classes(pattern: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut chars = pattern.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '[' {
            let mut class = Vec::new();
            let mut closed = false;
            for class_ch in chars.by_ref() {
                if class_ch == ']' {
                    closed = true;
                    break;
                }
                class.push(class_ch);
            }
            if closed && !class.is_empty() {
                let mut next = Vec::new();
                for prefix in &out {
                    for option in &class {
                        let mut candidate = prefix.clone();
                        candidate.push(*option);
                        next.push(candidate);
                    }
                }
                out = next;
            } else {
                for prefix in &mut out {
                    prefix.push('[');
                    for option in &class {
                        prefix.push(*option);
                    }
                }
            }
        } else {
            for prefix in &mut out {
                prefix.push(ch);
            }
        }
    }
    out
}

fn visibility_allowed(column_visibility: Option<&[u8]>, auths: &BTreeSet<String>) -> bool {
    let Some(column_visibility) = column_visibility else {
        return true;
    };
    if column_visibility.is_empty() {
        return true;
    }
    if auths.is_empty() {
        return false;
    }
    let expression = String::from_utf8_lossy(column_visibility);
    expression.split('|').any(|disjunct| {
        disjunct
            .split('&')
            .map(clean_visibility_term)
            .filter(|term| !term.is_empty())
            .all(|term| auths.contains(term))
    })
}

fn clean_visibility_term(term: &str) -> &str {
    term.trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .trim()
}

fn ttl_allowed(
    timestamp: u64,
    ttl_millis: Option<u64>,
    current_time_millis: Option<u64>,
    start_time_millis: Option<u64>,
    end_time_millis: Option<u64>,
) -> bool {
    if let Some(start_time) = start_time_millis
        && timestamp <= start_time
    {
        return false;
    }
    if let Some(end_time) = end_time_millis
        && timestamp > end_time
    {
        return false;
    }
    match (ttl_millis, current_time_millis) {
        (Some(_), Some(current_time)) if timestamp > current_time => false,
        (Some(ttl), Some(current_time)) => current_time.saturating_sub(timestamp) < ttl,
        _ => true,
    }
}

#[cfg(test)]
#[path = "tests/query_tests.rs"]
mod tests;
