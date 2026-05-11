use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;

use tree_sitter::{Node, Parser};

pub const CODE_NS: &str = "http://omrya.local/code/";
pub const PKG_NS: &str = "http://omrya.local/pkg/";
pub const SEC_NS: &str = "http://omrya.local/sec/";
pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

const MAX_REPO_FILES: usize = 400;
const MAX_FILE_BYTES: usize = 65_536;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeTriple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
}

impl CodeTriple {
    fn new(
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object: impl Into<String>,
    ) -> Self {
        Self {
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
        }
    }

    pub fn into_tuple(self) -> (String, String, String) {
        (self.subject, self.predicate, self.object)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PackageFact {
    uri: String,
    manager: String,
    name: String,
    version: Option<String>,
}

#[derive(Default)]
struct RepoCodeGraph {
    repo_uri: String,
    packages: BTreeMap<String, PackageFact>,
    triples: Vec<CodeTriple>,
    seen: BTreeSet<(String, String, String)>,
}

impl RepoCodeGraph {
    fn new(repo_uri: impl Into<String>) -> Self {
        Self {
            repo_uri: repo_uri.into(),
            ..Self::default()
        }
    }

    fn triple(
        &mut self,
        subject: impl Into<String>,
        predicate: impl Into<String>,
        object: impl Into<String>,
    ) {
        let triple = (subject.into(), predicate.into(), object.into());
        if self.seen.insert(triple.clone()) {
            self.triples
                .push(CodeTriple::new(triple.0, triple.1, triple.2));
        }
    }

    fn package(&mut self, manager: &str, name: &str, version: Option<&str>) -> String {
        let key = package_key(manager, name);
        if let Some(existing) = self.packages.get(&key) {
            return existing.uri.clone();
        }

        let uri = format!("{PKG_NS}{}/{}", slug(manager), slug(name));
        let class = match manager {
            "cargo" => format!("{PKG_NS}CargoCrate"),
            "npm" => format!("{PKG_NS}NpmPackage"),
            other => format!("{PKG_NS}{}Package", pascal_case(other)),
        };
        self.triple(uri.clone(), RDF_TYPE, class);
        self.triple(uri.clone(), format!("{PKG_NS}name"), name.to_string());
        self.triple(uri.clone(), format!("{PKG_NS}manager"), manager.to_string());
        if let Some(version) = version.filter(|value| !value.trim().is_empty()) {
            self.triple(
                uri.clone(),
                format!("{PKG_NS}hasVersion"),
                version.to_string(),
            );
        }
        self.packages.insert(
            key,
            PackageFact {
                uri: uri.clone(),
                manager: manager.to_string(),
                name: name.to_string(),
                version: version.map(str::to_string),
            },
        );
        uri
    }

    fn record_manifest(
        &mut self,
        repo_uri: &str,
        manifest_uri: &str,
        manager: &str,
        content: &str,
    ) {
        self.triple(manifest_uri, RDF_TYPE, format!("{CODE_NS}PackageManifest"));
        self.triple(repo_uri, format!("{CODE_NS}hasManifest"), manifest_uri);
        for (name, version) in manifest_dependencies(manager, content) {
            let package_uri = self.package(manager, &name, version.as_deref());
            self.triple(
                manifest_uri,
                format!("{CODE_NS}dependsOnPackage"),
                package_uri.clone(),
            );
            self.triple(repo_uri, format!("{CODE_NS}dependsOnPackage"), package_uri);
        }
    }

    fn record_file(
        &mut self,
        repo_uri: &str,
        file_uri: &str,
        path_text: &str,
        language: &str,
        line_count: usize,
    ) {
        self.triple(repo_uri, format!("{CODE_NS}containsFile"), file_uri);
        self.triple(file_uri, RDF_TYPE, format!("{CODE_NS}SourceFile"));
        self.triple(file_uri, format!("{CODE_NS}path"), path_text.to_string());
        self.triple(file_uri, format!("{CODE_NS}language"), language.to_string());
        self.triple(
            file_uri,
            format!("{CODE_NS}lineCount"),
            line_count.to_string(),
        );
    }

    fn record_symbol(
        &mut self,
        file_uri: &str,
        symbol_uri: &str,
        name: &str,
        kind: &str,
        exported: bool,
    ) {
        self.triple(symbol_uri, RDF_TYPE, format!("{CODE_NS}{kind}"));
        self.triple(symbol_uri, format!("{CODE_NS}name"), name.to_string());
        self.triple(symbol_uri, format!("{CODE_NS}definedIn"), file_uri);
        self.triple(file_uri, format!("{CODE_NS}defines"), symbol_uri);
        if exported {
            self.triple(file_uri, format!("{CODE_NS}exports"), symbol_uri);
        }
    }

    fn record_import(&mut self, file_uri: &str, import_name: &str) {
        let import_name = import_name.trim();
        if import_name.is_empty() {
            return;
        }

        if let Some(package) = self.package_for_import(import_name) {
            self.triple(file_uri, format!("{CODE_NS}imports"), package);
            return;
        }

        if matches!(import_name, "std" | "core" | "alloc") {
            let package = self.package("cargo", import_name, None);
            self.triple(file_uri, format!("{CODE_NS}imports"), package);
            return;
        }

        let module_uri = format!("{}/module/{}", self.repo_uri, slug(import_name));
        self.triple(module_uri.clone(), RDF_TYPE, format!("{CODE_NS}Module"));
        self.triple(
            module_uri.clone(),
            format!("{CODE_NS}name"),
            import_name.to_string(),
        );
        self.triple(file_uri, format!("{CODE_NS}imports"), module_uri);
    }

    fn package_for_import(&self, import_name: &str) -> Option<String> {
        let normalized = import_name.replace('-', "_");
        self.packages.values().find_map(|package| {
            let package_name = package.name.replace('-', "_");
            (package.name == import_name || package_name == normalized).then(|| package.uri.clone())
        })
    }

    fn record_call(&mut self, caller_uri: &str, call_name: &str) {
        let call_name = call_name.trim();
        if call_name.is_empty() {
            return;
        }
        let callee_uri = format!("{}/symbol/{}", self.repo_uri, slug(call_name));
        self.triple(callee_uri.clone(), RDF_TYPE, format!("{CODE_NS}Callable"));
        self.triple(
            callee_uri.clone(),
            format!("{CODE_NS}name"),
            call_name.to_string(),
        );
        self.triple(caller_uri, format!("{CODE_NS}calls"), callee_uri.clone());
        self.triple(caller_uri, format!("{CODE_NS}references"), callee_uri);
    }

    fn record_vulnerability(&mut self, package_ref: &str, cve: &str) {
        let Some((manager, name, version)) = parse_package_ref(package_ref) else {
            return;
        };
        let package = self.package(&manager, &name, version.as_deref());
        let vuln = format!("{SEC_NS}vuln/{cve}");
        self.triple(vuln.clone(), RDF_TYPE, format!("{SEC_NS}Vulnerability"));
        self.triple(vuln.clone(), format!("{SEC_NS}cve"), cve.to_string());
        self.triple(package, format!("{SEC_NS}hasVulnerability"), vuln);
    }
}

pub fn repo_to_rdf_triples(repo_path: &Path, repo_uri: &str) -> Result<Vec<CodeTriple>, String> {
    let files = repo_files(repo_path)?;
    let mut graph = RepoCodeGraph::new(repo_uri);
    graph.triple(repo_uri, RDF_TYPE, format!("{CODE_NS}Repository"));

    for relative in &files {
        let absolute = repo_path.join(relative);
        let path_text = normalized_path(relative);
        let file_uri = file_uri(repo_uri, &path_text);
        let Ok(content) = read_limited_text(&absolute, MAX_FILE_BYTES) else {
            continue;
        };
        if is_manifest_path(&path_text) {
            if let Some(manager) = manifest_manager(&path_text) {
                graph.record_manifest(repo_uri, &file_uri, manager, &content);
            }
        }
    }

    for relative in files {
        let absolute = repo_path.join(&relative);
        let path_text = normalized_path(&relative);
        let file_uri = file_uri(repo_uri, &path_text);
        let Ok(content) = read_limited_text(&absolute, MAX_FILE_BYTES) else {
            continue;
        };
        let language = language_for_path(&path_text);
        graph.record_file(
            repo_uri,
            &file_uri,
            &path_text,
            language,
            content.lines().count(),
        );

        if path_text.ends_with(".rs") {
            analyze_rust_file(&mut graph, &file_uri, &path_text, &content)?;
        } else if is_javascript_like(&path_text) {
            analyze_javascript_like_file(&mut graph, &file_uri, &content);
        }

        if is_vulnerability_data_path(&path_text) {
            analyze_vulnerability_data(&mut graph, &content);
        }
    }

    Ok(graph.triples)
}

fn analyze_rust_file(
    graph: &mut RepoCodeGraph,
    file_uri: &str,
    path_text: &str,
    content: &str,
) -> Result<(), String> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_rust::LANGUAGE.into())
        .map_err(|error| format!("Failed to initialize Rust parser for {path_text}: {error}"))?;
    let tree = parser
        .parse(content, None)
        .ok_or_else(|| format!("Failed to parse Rust source {path_text}"))?;
    visit_rust_node(graph, file_uri, content.as_bytes(), tree.root_node(), None);
    Ok(())
}

fn visit_rust_node(
    graph: &mut RepoCodeGraph,
    file_uri: &str,
    source: &[u8],
    node: Node<'_>,
    current_symbol: Option<String>,
) {
    match node.kind() {
        "use_declaration" => {
            if let Some(import_name) = rust_import_root(node_text(node, source).as_str()) {
                graph.record_import(file_uri, &import_name);
            }
        }
        "function_item" | "struct_item" | "enum_item" | "trait_item" | "impl_item" => {
            let kind = rust_symbol_kind(node.kind());
            if let Some(name_node) = node.child_by_field_name("name") {
                let name = node_text(name_node, source);
                let symbol_uri = symbol_uri_for_file(file_uri, &name);
                let exported = node_text(node, source).trim_start().starts_with("pub ");
                graph.record_symbol(file_uri, &symbol_uri, &name, kind, exported);
                let context = (node.kind() == "function_item").then_some(symbol_uri);
                visit_children(graph, file_uri, source, node, context.or(current_symbol));
                return;
            }
        }
        "call_expression" => {
            if let Some(caller) = current_symbol.as_deref()
                && let Some(function) = node
                    .child_by_field_name("function")
                    .or_else(|| node.child(0))
            {
                graph.record_call(caller, &node_text(function, source));
            }
        }
        _ => {}
    }

    visit_children(graph, file_uri, source, node, current_symbol);
}

fn visit_children(
    graph: &mut RepoCodeGraph,
    file_uri: &str,
    source: &[u8],
    node: Node<'_>,
    current_symbol: Option<String>,
) {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        visit_rust_node(graph, file_uri, source, child, current_symbol.clone());
    }
}

fn analyze_javascript_like_file(graph: &mut RepoCodeGraph, file_uri: &str, content: &str) {
    for line in content.lines() {
        if let Some(import_name) = js_import_package(line) {
            graph.record_import(file_uri, &import_name);
        }
    }
}

fn analyze_vulnerability_data(graph: &mut RepoCodeGraph, content: &str) {
    let cves = cve_ids(content);
    if cves.is_empty() {
        return;
    }
    let package_refs = package_refs(content);
    for package_ref in package_refs {
        for cve in &cves {
            graph.record_vulnerability(&package_ref, cve);
        }
    }
}

fn repo_files(repo_path: &Path) -> Result<Vec<PathBuf>, String> {
    if let Ok(output) = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["ls-files"])
        .output()
        && output.status.success()
    {
        let files = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.trim().is_empty())
            .take(MAX_REPO_FILES)
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        if !files.is_empty() {
            return Ok(files);
        }
    }

    let mut files = Vec::new();
    collect_files(repo_path, repo_path, &mut files)?;
    files.truncate(MAX_REPO_FILES);
    Ok(files)
}

fn collect_files(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if files.len() >= MAX_REPO_FILES {
        return Ok(());
    }
    for entry in fs::read_dir(current)
        .map_err(|error| format!("Failed to read {}: {error}", current.display()))?
    {
        let entry = entry.map_err(|error| format!("Failed to read directory entry: {error}"))?;
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if matches!(name, ".git" | "target" | "node_modules" | "dist" | "build") {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else if path.is_file()
            && let Ok(relative) = path.strip_prefix(root)
        {
            files.push(relative.to_path_buf());
        }
    }
    Ok(())
}

fn read_limited_text(path: &Path, max_bytes: usize) -> Result<String, String> {
    let mut buffer =
        fs::read(path).map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    if buffer.len() > max_bytes {
        buffer.truncate(max_bytes);
    }
    if buffer.contains(&0) {
        return Err("binary file skipped".to_string());
    }
    String::from_utf8(buffer).map_err(|_| "non UTF-8 file skipped".to_string())
}

fn manifest_dependencies(manager: &str, content: &str) -> Vec<(String, Option<String>)> {
    match manager {
        "cargo" => cargo_dependencies(content),
        "npm" => npm_dependencies(content),
        _ => Vec::new(),
    }
}

fn cargo_dependencies(content: &str) -> Vec<(String, Option<String>)> {
    let mut dependencies = Vec::new();
    let mut in_dependencies = false;
    for line in content.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.starts_with('[') {
            in_dependencies = matches!(
                line,
                "[dependencies]" | "[dev-dependencies]" | "[build-dependencies]"
            );
            continue;
        }
        if !in_dependencies || line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once('=') else {
            continue;
        };
        let name = name.trim().trim_matches('"');
        let version = quoted_value(value)
            .or_else(|| object_field_value(value, "version"))
            .filter(|version| !version.starts_with("./") && !version.starts_with("../"));
        dependencies.push((name.to_string(), version));
    }
    dependencies
}

fn npm_dependencies(content: &str) -> Vec<(String, Option<String>)> {
    let mut dependencies = Vec::new();
    let mut in_dependency_object = false;
    let mut depth = 0_i32;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("\"dependencies\"")
            || trimmed.starts_with("\"devDependencies\"")
            || trimmed.starts_with("\"peerDependencies\"")
            || trimmed.starts_with("\"optionalDependencies\"")
        {
            in_dependency_object = true;
            depth = brace_delta(trimmed);
            continue;
        }
        if !in_dependency_object {
            continue;
        }
        if let Some((name, rest)) = json_property(trimmed) {
            if name != "dependencies"
                && name != "devDependencies"
                && name != "peerDependencies"
                && name != "optionalDependencies"
            {
                dependencies.push((name, quoted_value(rest)));
            }
        }
        depth += brace_delta(trimmed);
        if depth <= 0 {
            in_dependency_object = false;
        }
    }
    dependencies
}

fn json_property(line: &str) -> Option<(String, &str)> {
    let after_quote = line.strip_prefix('"')?;
    let end = after_quote.find('"')?;
    let name = after_quote[..end].to_string();
    let rest = after_quote[end + 1..].trim_start();
    let rest = rest.strip_prefix(':')?.trim_start();
    Some((name, rest))
}

fn quoted_value(input: &str) -> Option<String> {
    let start = input.find('"')?;
    let rest = &input[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn object_field_value(input: &str, field: &str) -> Option<String> {
    let field = format!("\"{field}\"");
    let start = input.find(&field)?;
    quoted_value(&input[start + field.len()..])
}

fn brace_delta(line: &str) -> i32 {
    line.chars().fold(0, |delta, ch| match ch {
        '{' => delta + 1,
        '}' => delta - 1,
        _ => delta,
    })
}

fn rust_import_root(use_declaration: &str) -> Option<String> {
    let import = use_declaration
        .trim()
        .strip_prefix("use")?
        .trim()
        .trim_end_matches(';')
        .trim();
    let import = import.strip_prefix("::").unwrap_or(import);
    let first = import
        .split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_')
        .find(|part| !part.is_empty())?;
    Some(first.to_string())
}

fn js_import_package(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("import ") {
        if let Some((_, rest)) = trimmed.rsplit_once(" from ") {
            return quoted_import_value(rest).map(package_root);
        }
        return quoted_import_value(trimmed).map(package_root);
    }
    if let Some(index) = trimmed.find("require(") {
        return quoted_import_value(&trimmed[index + "require(".len()..]).map(package_root);
    }
    None
}

fn quoted_import_value(input: &str) -> Option<String> {
    let quote = input.chars().find(|ch| *ch == '\'' || *ch == '"')?;
    let start = input.find(quote)? + quote.len_utf8();
    let rest = &input[start..];
    let end = rest.find(quote)?;
    Some(rest[..end].to_string())
}

fn package_root(import: String) -> String {
    if import.starts_with('@') {
        let mut parts = import.split('/');
        let scope = parts.next().unwrap_or_default();
        let name = parts.next().unwrap_or_default();
        format!("{scope}/{name}")
    } else {
        import.split('/').next().unwrap_or(&import).to_string()
    }
}

fn cve_ids(content: &str) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    for token in content.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-') {
        if token.starts_with("CVE-") && token.len() >= "CVE-0000-0000".len() {
            ids.insert(token.to_string());
        }
    }
    ids
}

fn package_refs(content: &str) -> BTreeSet<String> {
    let mut refs = BTreeSet::new();
    for token in content
        .split(|ch: char| ch.is_whitespace() || ch == '"' || ch == '\'' || ch == ',' || ch == ']')
    {
        if let Some(start) = token.find("pkg:") {
            let value = token[start..]
                .trim_matches(|ch: char| matches!(ch, '"' | '\'' | ',' | ']' | '}' | ')'))
                .to_string();
            if value.starts_with("pkg:") {
                refs.insert(value);
            }
        }
    }
    refs
}

fn parse_package_ref(package_ref: &str) -> Option<(String, String, Option<String>)> {
    let rest = package_ref.strip_prefix("pkg:")?;
    let (manager, name_version) = rest.split_once('/')?;
    let name_version = name_version.split('?').next().unwrap_or(name_version);
    let (name, version) = match name_version.rsplit_once('@') {
        Some((name, version)) if !name.is_empty() => (name.to_string(), Some(version.to_string())),
        _ => (name_version.to_string(), None),
    };
    Some((manager.to_string(), name, version))
}

fn node_text(node: Node<'_>, source: &[u8]) -> String {
    std::str::from_utf8(&source[node.byte_range()])
        .unwrap_or_default()
        .to_string()
}

fn rust_symbol_kind(kind: &str) -> &'static str {
    match kind {
        "function_item" => "Function",
        "struct_item" => "Struct",
        "enum_item" => "Enum",
        "trait_item" => "Trait",
        "impl_item" => "Impl",
        _ => "Symbol",
    }
}

fn symbol_uri_for_file(file_uri: &str, name: &str) -> String {
    format!("{file_uri}/symbol/{}", slug(name))
}

fn file_uri(repo_uri: &str, path_text: &str) -> String {
    format!("{repo_uri}/file/{}", slug(path_text))
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn is_manifest_path(path: &str) -> bool {
    matches!(path, "Cargo.toml" | "package.json")
}

fn manifest_manager(path: &str) -> Option<&'static str> {
    match path {
        "Cargo.toml" => Some("cargo"),
        "package.json" => Some("npm"),
        _ => None,
    }
}

fn is_vulnerability_data_path(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".json")
        && (lower.contains("sbom")
            || lower.ends_with("bom.json")
            || lower.contains("audit")
            || lower.contains("vulnerab"))
}

fn is_javascript_like(path: &str) -> bool {
    matches!(
        Path::new(path).extension().and_then(|ext| ext.to_str()),
        Some("js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs")
    )
}

fn language_for_path(path: &str) -> &'static str {
    match Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
    {
        "rs" => "Rust",
        "js" | "mjs" | "cjs" => "JavaScript",
        "jsx" => "JSX",
        "ts" => "TypeScript",
        "tsx" => "TSX",
        "toml" => "TOML",
        "json" => "JSON",
        "md" => "Markdown",
        _ => "Text",
    }
}

fn package_key(manager: &str, name: &str) -> String {
    format!("{manager}:{name}")
}

fn pascal_case(value: &str) -> String {
    let mut output = String::new();
    let mut upper = true;
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            if upper {
                output.push(ch.to_ascii_uppercase());
                upper = false;
            } else {
                output.push(ch);
            }
        } else {
            upper = true;
        }
    }
    output
}

fn slug(value: &str) -> String {
    let mut slug = String::new();
    for ch in value.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        format!("item-{:016x}", hasher.finish())
    } else {
        slug.chars().take(96).collect()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    #[test]
    fn repo_to_rdf_triples_extracts_packages_imports_symbols_calls_and_cves() {
        let root = std::env::temp_dir().join(format!("omrya-code-intel-{}", nanos()));
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(
            root.join("Cargo.toml"),
            r#"
[package]
name = "fixture"
version = "0.1.0"

[dependencies]
serde = "1"
tree-sitter = { version = "0.26" }
"#,
        )
        .unwrap();
        fs::write(
            root.join("package.json"),
            r#"
{
  "dependencies": {
    "express": "4.18.2"
  }
}
"#,
        )
        .unwrap();
        fs::write(
            root.join("src").join("lib.rs"),
            r#"
use serde::Serialize;

pub fn auth_middleware() {
    verify_token();
}

fn verify_token() {}
"#,
        )
        .unwrap();
        fs::write(
            root.join("src").join("auth.js"),
            r#"
const express = require("express");
export function authMiddleware(req, res, next) { next(); }
"#,
        )
        .unwrap();
        fs::write(
            root.join("sbom.json"),
            r#"
{
  "components": [{"purl": "pkg:npm/express@4.18.2"}],
  "vulnerabilities": [{"id": "CVE-2024-12345", "affects": [{"ref": "pkg:npm/express@4.18.2"}]}]
}
"#,
        )
        .unwrap();

        let triples = repo_to_rdf_triples(&root, "http://omrya.local/code/repo/fixture").unwrap();
        let rendered = render(&triples);

        assert!(rendered.contains("http://omrya.local/pkg/cargo/serde"));
        assert!(rendered.contains("http://omrya.local/pkg/npm/express"));
        assert!(
            rendered.contains("http://omrya.local/code/imports http://omrya.local/pkg/cargo/serde")
        );
        assert!(
            rendered.contains("http://omrya.local/code/imports http://omrya.local/pkg/npm/express")
        );
        assert!(rendered.contains("http://omrya.local/code/defines"));
        assert!(rendered.contains("auth_middleware"));
        assert!(rendered.contains("http://omrya.local/code/calls"));
        assert!(rendered.contains("verify_token"));
        assert!(rendered.contains("http://omrya.local/sec/hasVulnerability"));
        assert!(rendered.contains("CVE-2024-12345"));

        fs::remove_dir_all(root).unwrap();
    }

    fn render(triples: &[CodeTriple]) -> String {
        triples
            .iter()
            .map(|triple| format!("{} {} {}", triple.subject, triple.predicate, triple.object))
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn nanos() -> u128 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }
}
