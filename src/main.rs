use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use omrya::web::{QueryRequest, RdfController};

const DEFAULT_ADDR: &str = "127.0.0.1:7878";
const DEFAULT_TABLE: &str = "sro";
const SESSION_FILE: &str = "session.token";
const SRO_LOG_FILE: &str = "sro.log";
const SPARQL_HTML: &str = include_str!("../sparql.html");
const FAVICON_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><rect width="64" height="64" rx="10" fill="#050a0f"/><circle cx="32" cy="32" r="22" fill="none" stroke="#4ba3ff" stroke-width="4"/><text x="32" y="38" text-anchor="middle" font-family="monospace" font-size="16" font-weight="700" fill="#d6e3eb">RYA</text></svg>"##;
const AVR_QUERY_ALL: &str = "SELECT * WHERE { ?s ?p ?o }";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const CODE_NS: &str = "http://omrya.local/code/";
const AVR_NS: &str = "http://omrya.local/avr/";
const AVR_PLAIN_ENGLISH: &str = "http://omrya.local/avr/plainEnglish";
const REPO_GRAPH: &str = "urn:graph:repo-exploit-memory";
const MAX_REPO_FILES: usize = 400;
const MAX_FILE_BYTES: usize = 65_536;

struct AvrDataset {
    name: &'static str,
    label: &'static str,
    description: &'static str,
    query: &'static str,
    triples: &'static [(&'static str, &'static str, &'static str)],
}

const AVR_MEMORY_TRIPLES: &[(&str, &str, &str)] = &[
    (
        "http://omrya.local/avr/runtime/orchestrator",
        "http://omrya.local/avr/hasComponent",
        "http://omrya.local/avr/component/planner",
    ),
    (
        "http://omrya.local/avr/runtime/orchestrator",
        "http://omrya.local/avr/hasComponent",
        "http://omrya.local/avr/component/schema-explorer",
    ),
    (
        "http://omrya.local/avr/runtime/orchestrator",
        "http://omrya.local/avr/hasComponent",
        "http://omrya.local/avr/component/query-mutator",
    ),
    (
        "http://omrya.local/avr/runtime/orchestrator",
        "http://omrya.local/avr/hasComponent",
        "http://omrya.local/avr/component/executor",
    ),
    (
        "http://omrya.local/avr/runtime/orchestrator",
        "http://omrya.local/avr/hasComponent",
        "http://omrya.local/avr/component/oracle-scorer",
    ),
    (
        "http://omrya.local/avr/runtime/orchestrator",
        "http://omrya.local/avr/hasComponent",
        "http://omrya.local/avr/component/minimizer",
    ),
    (
        "http://omrya.local/avr/runtime/orchestrator",
        "http://omrya.local/avr/hasComponent",
        "http://omrya.local/avr/component/replay-engine",
    ),
    (
        "http://omrya.local/avr/component/planner",
        "http://omrya.local/avr/writes",
        "http://omrya.local/avr/entity/campaign-spec",
    ),
    (
        "http://omrya.local/avr/component/schema-explorer",
        "http://omrya.local/avr/profiles",
        "http://omrya.local/avr/entity/dataset-snapshot",
    ),
    (
        "http://omrya.local/avr/component/query-mutator",
        "http://omrya.local/avr/appliesOperator",
        "http://omrya.local/avr/operator/AddOptional",
    ),
    (
        "http://omrya.local/avr/component/query-mutator",
        "http://omrya.local/avr/appliesOperator",
        "http://omrya.local/avr/operator/AddPropertyPathPlus",
    ),
    (
        "http://omrya.local/avr/component/executor",
        "http://omrya.local/avr/records",
        "http://omrya.local/avr/entity/execution",
    ),
    (
        "http://omrya.local/avr/component/oracle-scorer",
        "http://omrya.local/avr/classifies",
        "http://omrya.local/avr/entity/observation",
    ),
    (
        "http://omrya.local/avr/component/minimizer",
        "http://omrya.local/avr/produces",
        "http://omrya.local/avr/entity/reproducer",
    ),
    (
        "http://omrya.local/avr/component/replay-engine",
        "http://omrya.local/avr/replays",
        "http://omrya.local/avr/entity/regression-test",
    ),
    (
        "http://omrya.local/avr/memory/kg",
        "http://omrya.local/avr/stores",
        "http://omrya.local/avr/entity/exploit-lineage-graph",
    ),
    (
        "http://omrya.local/avr/memory/kg",
        "http://omrya.local/avr/stores",
        "http://omrya.local/avr/entity/query-mutation-history",
    ),
    (
        "http://omrya.local/avr/memory/kg",
        "http://omrya.local/avr/stores",
        "http://omrya.local/avr/entity/failure-signature",
    ),
    (
        "http://omrya.local/avr/memory/kg",
        "http://omrya.local/avr/stores",
        "http://omrya.local/avr/entity/reusable-attack-pattern",
    ),
    (
        "http://omrya.local/avr/runtime/orchestrator",
        RDF_TYPE,
        "http://omrya.local/avr/AgentRuntime",
    ),
    (
        "http://omrya.local/avr/memory/kg",
        RDF_TYPE,
        "http://omrya.local/avr/MemoryGraph",
    ),
];

const AVR_LINEAGE_TRIPLES: &[(&str, &str, &str)] = &[
    (
        "http://omrya.local/avr/query/seed-001",
        RDF_TYPE,
        "http://omrya.local/avr/Query",
    ),
    (
        "http://omrya.local/avr/query/optional-017",
        "http://omrya.local/avr/derivedFrom",
        "http://omrya.local/avr/query/seed-001",
    ),
    (
        "http://omrya.local/avr/query/path-042",
        "http://omrya.local/avr/derivedFrom",
        "http://omrya.local/avr/query/optional-017",
    ),
    (
        "http://omrya.local/avr/query/union-055",
        "http://omrya.local/avr/derivedFrom",
        "http://omrya.local/avr/query/seed-001",
    ),
    (
        "http://omrya.local/avr/query/path-042",
        "http://omrya.local/avr/mutatedBy",
        "http://omrya.local/avr/operator/AddPropertyPathPlus",
    ),
    (
        "http://omrya.local/avr/query/optional-017",
        "http://omrya.local/avr/mutatedBy",
        "http://omrya.local/avr/operator/AddOptional",
    ),
    (
        "http://omrya.local/avr/query/union-055",
        "http://omrya.local/avr/mutatedBy",
        "http://omrya.local/avr/operator/AddUnionBranch",
    ),
    (
        "http://omrya.local/avr/query/path-042",
        "http://omrya.local/avr/usesFeature",
        "http://omrya.local/sparql/feature/property-path-plus",
    ),
    (
        "http://omrya.local/avr/query/path-042",
        "http://omrya.local/avr/usesFeature",
        "http://omrya.local/sparql/feature/optional-chain",
    ),
    (
        "http://omrya.local/avr/execution/9421",
        "http://omrya.local/avr/executedQuery",
        "http://omrya.local/avr/query/path-042",
    ),
    (
        "http://omrya.local/avr/execution/9421",
        "http://omrya.local/avr/triggered",
        "http://omrya.local/avr/signature/timeout-17",
    ),
    (
        "http://omrya.local/avr/signature/timeout-17",
        "http://omrya.local/avr/failureClass",
        "Timeout",
    ),
    (
        "http://omrya.local/avr/signature/timeout-17",
        "http://omrya.local/avr/resourceAmplificationScore",
        "0.91",
    ),
    (
        "http://omrya.local/avr/query/path-042",
        "http://omrya.local/avr/minimizedTo",
        "http://omrya.local/avr/reproducer/min-path-042",
    ),
    (
        "http://omrya.local/avr/reproducer/min-path-042",
        "http://omrya.local/avr/replaysAs",
        "http://omrya.local/avr/regression/property-path-timeout",
    ),
    (
        "http://omrya.local/avr/query/path-042",
        "http://omrya.local/avr/belongsToCluster",
        "http://omrya.local/avr/cluster/property-path-optional-timeout",
    ),
];

const AVR_SELF_IMPROVE_TRIPLES: &[(&str, &str, &str)] = &[
    (
        "http://omrya.local/avr/campaign/2026-05-10",
        "http://omrya.local/avr/clustersFailuresInto",
        "http://omrya.local/avr/cluster/property-path-optional-timeout",
    ),
    (
        "http://omrya.local/avr/cluster/property-path-optional-timeout",
        "http://omrya.local/avr/promotesPattern",
        "http://omrya.local/avr/pattern/HighFanoutOptionalPathSortPattern",
    ),
    (
        "http://omrya.local/avr/pattern/HighFanoutOptionalPathSortPattern",
        "http://omrya.local/avr/promotedToTemplate",
        "http://omrya.local/avr/template/high-fanout-optional-path-sort",
    ),
    (
        "http://omrya.local/avr/template/high-fanout-optional-path-sort",
        "http://omrya.local/avr/usesFeature",
        "http://omrya.local/sparql/feature/optional-chain",
    ),
    (
        "http://omrya.local/avr/template/high-fanout-optional-path-sort",
        "http://omrya.local/avr/usesFeature",
        "http://omrya.local/sparql/feature/property-path-plus",
    ),
    (
        "http://omrya.local/avr/template/high-fanout-optional-path-sort",
        "http://omrya.local/avr/usesFeature",
        "http://omrya.local/sparql/feature/order-by",
    ),
    (
        "http://omrya.local/avr/operator/AddPropertyPathPlus",
        "http://omrya.local/avr/operatorYield",
        "0.42",
    ),
    (
        "http://omrya.local/avr/operator/AddOptional",
        "http://omrya.local/avr/operatorYield",
        "0.31",
    ),
    (
        "http://omrya.local/avr/operator/AddUnionBranch",
        "http://omrya.local/avr/operatorYield",
        "0.14",
    ),
    (
        "http://omrya.local/avr/planner/prior/property-path",
        "http://omrya.local/avr/upweightsOperator",
        "http://omrya.local/avr/operator/AddPropertyPathPlus",
    ),
    (
        "http://omrya.local/avr/planner/prior/sterile-regex",
        "http://omrya.local/avr/downranksOperator",
        "http://omrya.local/avr/operator/AddRegexFilter",
    ),
    (
        "http://omrya.local/avr/retriever/hybrid",
        "http://omrya.local/avr/retrievesSimilarTo",
        "http://omrya.local/avr/query/path-042",
    ),
    (
        "http://omrya.local/avr/retriever/hybrid",
        "http://omrya.local/avr/usesIndex",
        "http://omrya.local/avr/vector/query-shape-embedding",
    ),
    (
        "http://omrya.local/avr/retriever/hybrid",
        "http://omrya.local/avr/usesIndex",
        "http://omrya.local/avr/symbolic/sparql-memory",
    ),
];

const AVR_DATASETS: &[AvrDataset] = &[
    AvrDataset {
        name: "memory",
        label: "Graph Exploit Memory",
        description: "Runtime architecture for self-improving RDF/SPARQL fuzzing memory.",
        query: AVR_QUERY_ALL,
        triples: AVR_MEMORY_TRIPLES,
    },
    AvrDataset {
        name: "lineage",
        label: "Query Lineage",
        description: "Mutation genealogy, executions, signatures, clusters, and reproducers.",
        query: "SELECT * WHERE { ?s <http://omrya.local/avr/derivedFrom> ?o }",
        triples: AVR_LINEAGE_TRIPLES,
    },
    AvrDataset {
        name: "self-improve",
        label: "Self-Improvement",
        description: "Cluster promotion, operator yield, planner priors, and hybrid retrieval.",
        query: "SELECT * WHERE { ?s <http://omrya.local/avr/promotesPattern> ?o }",
        triples: AVR_SELF_IMPROVE_TRIPLES,
    },
];

fn main() {
    if let Err(error) = run(env::args().collect()) {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    match args.get(1).map(String::as_str) {
        Some("server") => match args.get(2).map(String::as_str) {
            Some("start") => start_server(&args[3..]),
            Some("metrics") => print_metrics(&args[3..]),
            _ => {
                print_usage();
                Ok(())
            }
        },
        Some("web") => start_web(&args[2..]),
        Some("metrics") => print_metrics(&args[2..]),
        Some("signin") | Some("sign-in") | Some("connect") => signin(&args[2..]),
        Some("sparql") | Some("sparql-query") => execute_sparql(&args[2..]),
        Some("logs") | Some("sro-logs") => view_logs(&args[2..]),
        Some("help") | Some("--help") | Some("-h") | None => {
            print_usage();
            Ok(())
        }
        Some(other) => Err(format!("Unknown command: {other}\n\n{}", usage())),
    }
}

fn start_server(args: &[String]) -> Result<(), String> {
    let opts = CliOptions::parse(args)?;
    run_server(opts, false)
}

fn start_web(args: &[String]) -> Result<(), String> {
    let opts = CliOptions::parse(args)?;
    run_server(opts, true)
}

fn run_server(opts: CliOptions, open_web: bool) -> Result<(), String> {
    let state_dir = opts.state_dir.unwrap_or_else(default_state_dir);
    fs::create_dir_all(&state_dir)
        .map_err(|e| format!("Failed to create state dir {}: {e}", state_dir.display()))?;

    let listener = TcpListener::bind(&opts.addr)
        .map_err(|e| format!("Failed to bind server at {}: {e}", opts.addr))?;
    let state = Arc::new(Mutex::new(ServerState::new(state_dir.clone())));

    println!("omrya server listening on {}", opts.addr);
    println!("state dir: {}", state_dir.display());
    if open_web {
        let url = format!("http://{}/sparql.html", opts.addr);
        println!("SPARQL web interface: {url}");
        if !opts.no_open {
            if let Err(error) = open_browser(&url) {
                eprintln!("Failed to open browser: {error}");
                eprintln!("Open {url}");
            }
        }
    }

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(error) = handle_connection(&mut stream, &state) {
                    let _ = write_response(&mut stream, 500, "text/plain", &error);
                }
            }
            Err(error) => eprintln!("accept failed: {error}"),
        }
    }

    Ok(())
}

fn open_browser(url: &str) -> Result<(), String> {
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg(url);
        command
    } else {
        let mut command = Command::new("xdg-open");
        command.arg(url);
        command
    };

    command
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("browser launcher failed: {e}"))
}

fn print_metrics(args: &[String]) -> Result<(), String> {
    let opts = CliOptions::parse(args)?;
    let response = request(
        &opts.addr,
        "GET",
        "/metrics",
        &[auth_header(opts.state_dir.as_deref())?],
        "",
    )?;
    print!("{}", response.body);
    Ok(())
}

fn signin(args: &[String]) -> Result<(), String> {
    let opts = CliOptions::parse(args)?;
    let user = opts
        .user
        .as_deref()
        .ok_or_else(|| "signin requires --user <name>".to_string())?;
    let password = opts
        .password
        .as_deref()
        .ok_or_else(|| "signin requires --password <secret>".to_string())?;
    let path = format!(
        "/signin?user={}&password={}",
        url_encode(user),
        url_encode(password)
    );
    let response = request(&opts.addr, "POST", &path, &[], "")?;
    if response.status != 200 {
        return Err(response.body);
    }

    let token = response.body.trim();
    let state_dir = opts.state_dir.unwrap_or_else(default_state_dir);
    fs::create_dir_all(&state_dir)
        .map_err(|e| format!("Failed to create state dir {}: {e}", state_dir.display()))?;
    fs::write(state_dir.join(SESSION_FILE), token)
        .map_err(|e| format!("Failed to write session token: {e}"))?;

    println!("Signed in as {user}");
    Ok(())
}

fn execute_sparql(args: &[String]) -> Result<(), String> {
    let opts = CliOptions::parse(args)?;
    let query = match (opts.query, opts.file) {
        (Some(query), None) => query,
        (None, Some(file)) => fs::read_to_string(&file)
            .map_err(|e| format!("Failed to read SPARQL file {}: {e}", file.display()))?,
        (None, None) => read_stdin_query()?,
        (Some(_), Some(_)) => return Err("Use either --query or --file, not both".to_string()),
    };
    let path = format!("/sparql?query={}", url_encode(&query));
    let response = request(
        &opts.addr,
        "POST",
        &path,
        &[auth_header(opts.state_dir.as_deref())?],
        "",
    )?;
    if response.status != 200 {
        return Err(response.body);
    }
    println!("{}", response.body);
    Ok(())
}

fn view_logs(args: &[String]) -> Result<(), String> {
    let opts = CliOptions::parse(args)?;
    let table = opts.table.as_deref().unwrap_or(DEFAULT_TABLE);
    let limit = opts.limit.unwrap_or(50);
    if opts.follow {
        loop {
            print!("\x1B[2J\x1B[H");
            println!("omrya {table} log viewer - refreshes every second; press Ctrl-C to exit\n");
            print_log_snapshot(&opts.addr, opts.state_dir.as_deref(), table, limit)?;
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    print_log_snapshot(&opts.addr, opts.state_dir.as_deref(), table, limit)
}

fn print_log_snapshot(
    addr: &str,
    state_dir: Option<&Path>,
    table: &str,
    limit: usize,
) -> Result<(), String> {
    let path = format!("/logs?table={}&limit={limit}", url_encode(table));
    let response = request(addr, "GET", &path, &[auth_header(state_dir)?], "")?;
    if response.status != 200 {
        return Err(response.body);
    }
    print!("{}", response.body);
    Ok(())
}

fn handle_connection(
    stream: &mut TcpStream,
    state: &Arc<Mutex<ServerState>>,
) -> Result<(), String> {
    let request = read_request(stream)?;
    let mut state = state
        .lock()
        .map_err(|_| "Server state lock poisoned".to_string())?;
    state.metrics.requests += 1;

    match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/") | ("GET", "/sparql.html") => {
            write_response(stream, 200, "text/html; charset=utf-8", SPARQL_HTML)
        }
        ("GET", "/favicon.ico") => write_response(stream, 200, "image/svg+xml", FAVICON_SVG),
        ("GET", "/health") => write_response(stream, 200, "text/plain", "ok\n"),
        ("GET", "/metrics") => {
            state.metrics.metrics_requests += 1;
            let body = state
                .metrics
                .render(state.sessions.len(), &state.log_path());
            write_response(stream, 200, "text/plain", &body)
        }
        ("GET", "/avr/datasets") => {
            write_response(stream, 200, "application/json", &avr_dataset_catalog_json())
        }
        ("POST", "/avr/seed") => {
            if let Err(error) = require_token(&request, &state) {
                return write_response(stream, 401, "text/plain", &(error + "\n"));
            }
            let dataset_name = request
                .query
                .get("dataset")
                .map(String::as_str)
                .unwrap_or("memory");
            let dataset = avr_dataset(dataset_name)
                .ok_or_else(|| format!("Unknown AVR dataset: {dataset_name}"))?;
            let query = avr_insert_query(dataset);
            state
                .controller
                .query_rdf(QueryRequest::new(query))
                .map_err(|e| format!("AVR seed failed: {e}"))?;
            state.append_sro_log("avr-seed", dataset.name)?;
            write_response(
                stream,
                200,
                "application/json",
                &avr_dataset_rows_json(dataset),
            )
        }
        ("POST", "/avr/repo/ingest") => {
            if let Err(error) = require_token(&request, &state) {
                return write_response(stream, 401, "text/plain", &(error + "\n"));
            }
            let target = request
                .query
                .get("target")
                .filter(|value| !value.trim().is_empty())
                .ok_or_else(|| "Missing authorized repository target".to_string())?;
            let objective = request
                .query
                .get("objective")
                .map(String::as_str)
                .unwrap_or("authorized offensive query evolution");
            let mode = request
                .query
                .get("mode")
                .map(String::as_str)
                .unwrap_or("clone");
            let triples = ingest_repo_triples(&state.state_dir, target, objective, mode)?;
            let query = insert_query_from_owned_triples(&triples);
            state
                .controller
                .query_rdf(QueryRequest::new(query))
                .map_err(|e| format!("Repository graph insert failed: {e}"))?;
            state.append_sro_log("repo-ingest", target)?;
            write_response(
                stream,
                200,
                "application/json",
                &owned_triples_rows_json(
                    "repo-ingest",
                    "Repository Exploit Memory",
                    AVR_QUERY_ALL,
                    REPO_GRAPH,
                    &triples,
                ),
            )
        }
        (_, "/signin") => {
            let user = request
                .query
                .get("user")
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "Missing user".to_string())?
                .to_string();
            let password = request
                .query
                .get("password")
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "Missing password".to_string())?
                .to_string();
            let token = make_token(&user, &password);
            state.sessions.insert(token.clone());
            state.metrics.signins += 1;
            state.append_sro_log("signin", &format!("user={user}"))?;
            write_response(stream, 200, "text/plain", &format!("{token}\n"))
        }
        (_, "/sparql") => {
            if let Err(error) = require_token(&request, &state) {
                return write_response(stream, 401, "text/plain", &(error + "\n"));
            }
            let query = request
                .query
                .get("query")
                .cloned()
                .filter(|query| !query.trim().is_empty())
                .unwrap_or(request.body);
            if query.trim().is_empty() {
                return write_response(stream, 400, "text/plain", "Missing SPARQL query\n");
            }

            state.metrics.sparql_queries += 1;
            let mut query_request = QueryRequest::new(query.clone());
            query_request.query_auth = request.query.get("query.auth").cloned();
            query_request.conf_cv = request
                .query
                .get("conf.cv")
                .or_else(|| request.query.get("visibility"))
                .cloned();
            query_request.infer = request.query.get("query.infer").cloned();
            query_request.result_format = request.query.get("query.resultformat").cloned();
            query_request.tx_time_millis = request
                .query
                .get("temporal.txTime")
                .or_else(|| request.query.get("txTime"))
                .cloned();
            query_request.tx_as_of_millis = request
                .query
                .get("temporal.asOf")
                .or_else(|| request.query.get("asOf"))
                .cloned();
            query_request.tx_after_millis = request
                .query
                .get("temporal.after")
                .or_else(|| request.query.get("after"))
                .cloned();
            query_request.tx_before_millis = request
                .query
                .get("temporal.before")
                .or_else(|| request.query.get("before"))
                .cloned();
            query_request.ttl_millis = request
                .query
                .get("temporal.ttl")
                .or_else(|| request.query.get("ttl"))
                .cloned();
            query_request.current_time_millis = request
                .query
                .get("temporal.now")
                .or_else(|| request.query.get("now"))
                .cloned();
            query_request.valid_at = request
                .query
                .get("temporal.validAt")
                .or_else(|| request.query.get("validAt"))
                .cloned();
            query_request.valid_from = request
                .query
                .get("temporal.validFrom")
                .or_else(|| request.query.get("validFrom"))
                .cloned();
            query_request.valid_to = request
                .query
                .get("temporal.validTo")
                .or_else(|| request.query.get("validTo"))
                .cloned();
            let response = state
                .controller
                .query_rdf(query_request)
                .map_err(|e| format!("SPARQL failed: {e}"))?;
            state.append_sro_log("sparql", one_line(&query).as_str())?;
            write_response(
                stream,
                usize::from(response.status),
                response.content_type.as_deref().unwrap_or("text/plain"),
                &response.body,
            )
        }
        ("GET", "/logs") => {
            if let Err(error) = require_token(&request, &state) {
                return write_response(stream, 401, "text/plain", &(error + "\n"));
            }
            let table = request
                .query
                .get("table")
                .map(String::as_str)
                .unwrap_or(DEFAULT_TABLE);
            if !table.eq_ignore_ascii_case(DEFAULT_TABLE) {
                return write_response(stream, 404, "text/plain", "Unknown log table\n");
            }
            let limit = request
                .query
                .get("limit")
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(50);
            state.metrics.log_views += 1;
            let body = read_tail(&state.log_path(), limit)?;
            write_response(stream, 200, "text/plain", &body)
        }
        _ => write_response(stream, 404, "text/plain", "Not found\n"),
    }
}

fn require_token(request: &HttpRequest, state: &ServerState) -> Result<(), String> {
    let Some(token) = request.headers.get("x-omrya-token") else {
        return Err(
            "Missing session token. Run `omrya signin --user <name> --password <secret>`."
                .to_string(),
        );
    };
    if state.sessions.contains(token) {
        Ok(())
    } else {
        Err("Invalid session token. Sign in again.".to_string())
    }
}

fn avr_dataset(name: &str) -> Option<&'static AvrDataset> {
    AVR_DATASETS
        .iter()
        .find(|dataset| dataset.name.eq_ignore_ascii_case(name))
}

fn avr_dataset_catalog_json() -> String {
    let mut body = String::from("{\"datasets\":[");
    for (index, dataset) in AVR_DATASETS.iter().enumerate() {
        if index > 0 {
            body.push(',');
        }
        body.push_str("{\"name\":\"");
        body.push_str(&json_escape(dataset.name));
        body.push_str("\",\"label\":\"");
        body.push_str(&json_escape(dataset.label));
        body.push_str("\",\"description\":\"");
        body.push_str(&json_escape(dataset.description));
        body.push_str("\",\"query\":\"");
        body.push_str(&json_escape(dataset.query));
        body.push_str("\",\"tripleCount\":");
        body.push_str(&dataset.triples.len().to_string());
        body.push('}');
    }
    body.push_str("]}");
    body
}

fn avr_dataset_rows_json(dataset: &AvrDataset) -> String {
    let mut body = String::from("{\"columns\":[\"s\",\"p\",\"o\",\"g\"],\"dataset\":\"");
    body.push_str(&json_escape(dataset.name));
    body.push_str("\",\"label\":\"");
    body.push_str(&json_escape(dataset.label));
    body.push_str("\",\"query\":\"");
    body.push_str(&json_escape(dataset.query));
    body.push_str("\",\"rows\":[");
    for (index, (subject, predicate, object)) in dataset.triples.iter().enumerate() {
        if index > 0 {
            body.push(',');
        }
        body.push_str("{\"s\":\"");
        body.push_str(&json_escape(subject));
        body.push_str("\",\"p\":\"");
        body.push_str(&json_escape(predicate));
        body.push_str("\",\"o\":\"");
        body.push_str(&json_escape(object));
        body.push_str("\",\"g\":\"urn:graph:avr-memory\"}");
    }
    body.push_str("]}");
    body
}

fn avr_insert_query(dataset: &AvrDataset) -> String {
    let mut query = String::from("INSERT DATA { ");
    for (subject, predicate, object) in dataset.triples {
        query.push_str(&sparql_term(subject));
        query.push(' ');
        query.push_str(&sparql_term(predicate));
        query.push(' ');
        query.push_str(&sparql_term(object));
        query.push_str(" . ");
    }
    query.push('}');
    query
}

fn insert_query_from_owned_triples(triples: &[(String, String, String)]) -> String {
    let mut query = String::from("INSERT DATA { ");
    for (subject, predicate, object) in triples {
        query.push_str(&sparql_term(subject));
        query.push(' ');
        query.push_str(&sparql_term(predicate));
        query.push(' ');
        query.push_str(&sparql_term(object));
        query.push_str(" . ");
    }
    query.push('}');
    query
}

fn owned_triples_rows_json(
    dataset: &str,
    label: &str,
    query: &str,
    graph: &str,
    triples: &[(String, String, String)],
) -> String {
    let mut body = String::from("{\"columns\":[\"s\",\"p\",\"o\",\"g\"],\"dataset\":\"");
    body.push_str(&json_escape(dataset));
    body.push_str("\",\"label\":\"");
    body.push_str(&json_escape(label));
    body.push_str("\",\"query\":\"");
    body.push_str(&json_escape(query));
    body.push_str("\",\"rows\":[");
    for (index, (subject, predicate, object)) in triples.iter().enumerate() {
        if index > 0 {
            body.push(',');
        }
        body.push_str("{\"s\":\"");
        body.push_str(&json_escape(subject));
        body.push_str("\",\"p\":\"");
        body.push_str(&json_escape(predicate));
        body.push_str("\",\"o\":\"");
        body.push_str(&json_escape(object));
        body.push_str("\",\"g\":\"");
        body.push_str(&json_escape(graph));
        body.push_str("\"}");
    }
    body.push_str("]}");
    body
}

fn ingest_repo_triples(
    state_dir: &Path,
    target: &str,
    objective: &str,
    mode: &str,
) -> Result<Vec<(String, String, String)>, String> {
    let repo_path = prepare_repo_target(state_dir, target, mode)?;
    let repo_slug = slug(
        repo_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("repo"),
    );
    let repo_uri = format!("{CODE_NS}repo/{repo_slug}");
    let campaign_uri = format!(
        "{AVR_NS}campaign/repo-{}-{}",
        stable_hash(target),
        unix_seconds()
    );
    let mut triples = vec![
        (
            repo_uri.clone(),
            RDF_TYPE.to_string(),
            format!("{CODE_NS}Repository"),
        ),
        (
            repo_uri.clone(),
            format!("{CODE_NS}sourceTarget"),
            target.to_string(),
        ),
        (
            repo_uri.clone(),
            AVR_PLAIN_ENGLISH.to_string(),
            format!(
                "Repository opened from {target}. Omrya indexes files as SourceFile nodes and links generated hypotheses back to file paths."
            ),
        ),
        (
            campaign_uri.clone(),
            RDF_TYPE.to_string(),
            format!("{AVR_NS}OffensiveQueryEvolutionCampaign"),
        ),
        (
            campaign_uri.clone(),
            format!("{AVR_NS}authorizedScope"),
            "local repository analysis only".to_string(),
        ),
        (
            campaign_uri.clone(),
            format!("{AVR_NS}objective"),
            objective.to_string(),
        ),
        (
            campaign_uri.clone(),
            AVR_PLAIN_ENGLISH.to_string(),
            "One authorized repository analysis run. It connects the repository to hypotheses, generated probes, and evidence so the graph can be audited.".to_string(),
        ),
        (
            campaign_uri.clone(),
            format!("{AVR_NS}analyzesRepository"),
            repo_uri.clone(),
        ),
        (
            campaign_uri.clone(),
            format!("{AVR_NS}usesComponent"),
            format!("{AVR_NS}component/graphrag-retriever"),
        ),
        (
            campaign_uri.clone(),
            format!("{AVR_NS}usesComponent"),
            format!("{AVR_NS}component/offensive-query-planner"),
        ),
        (
            campaign_uri.clone(),
            format!("{AVR_NS}usesComponent"),
            format!("{AVR_NS}component/static-oracle"),
        ),
        (
            campaign_uri.clone(),
            format!("{AVR_NS}usesComponent"),
            format!("{AVR_NS}component/evidence-writer"),
        ),
    ];

    let files = repo_files(&repo_path)?;
    triples.push((
        repo_uri.clone(),
        format!("{CODE_NS}indexedFileCount"),
        files.len().to_string(),
    ));

    let mut finding_index = 0;
    for relative in files {
        let absolute = repo_path.join(&relative);
        let path_text = relative.to_string_lossy().replace('\\', "/");
        let file_uri = format!("{repo_uri}/file/{}", slug(&path_text));
        let language = language_for(&path_text);
        triples.push((
            repo_uri.clone(),
            format!("{CODE_NS}containsFile"),
            file_uri.clone(),
        ));
        triples.push((
            file_uri.clone(),
            RDF_TYPE.to_string(),
            format!("{CODE_NS}SourceFile"),
        ));
        triples.push((
            file_uri.clone(),
            format!("{CODE_NS}path"),
            path_text.clone(),
        ));
        triples.push((
            file_uri.clone(),
            format!("{CODE_NS}language"),
            language.to_string(),
        ));
        triples.push((
            file_uri.clone(),
            AVR_PLAIN_ENGLISH.to_string(),
            format!(
                "{path_text} is an indexed {language} source file. Hypotheses target this node when a bounded analysis pattern matched the file."
            ),
        ));

        let Ok(content) = read_limited_text(&absolute, MAX_FILE_BYTES) else {
            continue;
        };
        triples.push((
            file_uri.clone(),
            format!("{CODE_NS}lineCount"),
            content.lines().count().to_string(),
        ));

        for finding in detect_code_findings(&path_text, &content) {
            finding_index += 1;
            let hypothesis_uri = format!(
                "{campaign_uri}/hypothesis/{finding_index}-{}",
                slug(finding.kind)
            );
            let probe_uri = format!(
                "{campaign_uri}/probe/{finding_index}-{}",
                slug(finding.operator)
            );
            let evidence_uri = format!("{campaign_uri}/evidence/{finding_index}");
            triples.push((
                campaign_uri.clone(),
                format!("{AVR_NS}hasHypothesis"),
                hypothesis_uri.clone(),
            ));
            triples.push((
                hypothesis_uri.clone(),
                RDF_TYPE.to_string(),
                format!("{AVR_NS}VulnerabilityHypothesis"),
            ));
            triples.push((
                hypothesis_uri.clone(),
                format!("{AVR_NS}failureClass"),
                finding.kind.to_string(),
            ));
            triples.push((
                hypothesis_uri.clone(),
                AVR_PLAIN_ENGLISH.to_string(),
                format!(
                    "Possible {} in {}:{}. This is a review lead, not proof; inspect the linked probe and evidence before acting.",
                    finding.kind, path_text, finding.line
                ),
            ));
            triples.push((
                hypothesis_uri.clone(),
                format!("{AVR_NS}targetsComponent"),
                file_uri.clone(),
            ));
            triples.push((
                hypothesis_uri.clone(),
                format!("{AVR_NS}generatedProbe"),
                probe_uri.clone(),
            ));
            triples.push((
                probe_uri.clone(),
                RDF_TYPE.to_string(),
                format!("{AVR_NS}CodeQueryProbe"),
            ));
            triples.push((
                probe_uri.clone(),
                format!("{AVR_NS}mutatedBy"),
                format!("{AVR_NS}operator/{}", finding.operator),
            ));
            triples.push((
                probe_uri.clone(),
                format!("{AVR_NS}queryIntent"),
                finding.query_intent.to_string(),
            ));
            triples.push((
                probe_uri.clone(),
                AVR_PLAIN_ENGLISH.to_string(),
                format!(
                    "Generated follow-up analysis idea: {}",
                    finding.query_intent
                ),
            ));
            triples.push((
                hypothesis_uri.clone(),
                format!("{AVR_NS}supportedByEvidence"),
                evidence_uri.clone(),
            ));
            triples.push((
                evidence_uri.clone(),
                RDF_TYPE.to_string(),
                format!("{AVR_NS}Evidence"),
            ));
            triples.push((
                evidence_uri.clone(),
                format!("{CODE_NS}path"),
                path_text.clone(),
            ));
            triples.push((
                evidence_uri.clone(),
                format!("{CODE_NS}line"),
                finding.line.to_string(),
            ));
            triples.push((
                evidence_uri.clone(),
                AVR_PLAIN_ENGLISH.to_string(),
                format!(
                    "Evidence came from {}:{}. The snippet is the source text that triggered this generated hypothesis.",
                    path_text, finding.line
                ),
            ));
            triples.push((evidence_uri, format!("{CODE_NS}snippet"), finding.snippet));
        }
    }

    if finding_index == 0 {
        triples.push((
            campaign_uri,
            format!("{AVR_NS}observedSignal"),
            "No static exploit-memory hypotheses matched the bounded pattern set".to_string(),
        ));
    }

    Ok(triples)
}

fn prepare_repo_target(state_dir: &Path, target: &str, mode: &str) -> Result<PathBuf, String> {
    let target_path = Path::new(target);
    if mode.eq_ignore_ascii_case("local") || target_path.exists() {
        return target_path
            .canonicalize()
            .map_err(|e| format!("Failed to resolve local repository target {target}: {e}"));
    }

    if !target.starts_with("https://") && !target.starts_with("http://") {
        return Err(
            "Remote clone targets must be http(s) URLs; use mode=local for local paths."
                .to_string(),
        );
    }

    let repos_dir = state_dir.join("repos");
    fs::create_dir_all(&repos_dir)
        .map_err(|e| format!("Failed to create repo cache {}: {e}", repos_dir.display()))?;
    let repo_name = repo_name_from_target(target);
    let destination = repos_dir.join(repo_name);
    if destination.exists() {
        return Ok(destination);
    }

    let status = Command::new("git")
        .args(["clone", "--depth", "1", "--", target])
        .arg(&destination)
        .status()
        .map_err(|e| format!("Failed to start git clone: {e}"))?;
    if !status.success() {
        return Err(format!("git clone failed for {target}"));
    }
    Ok(destination)
}

fn repo_name_from_target(target: &str) -> String {
    let tail = target
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("repo")
        .trim_end_matches(".git");
    format!("{}-{}", slug(tail), stable_hash(target))
}

fn repo_files(repo_path: &Path) -> Result<Vec<PathBuf>, String> {
    if let Ok(output) = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["ls-files"])
        .output()
    {
        if output.status.success() {
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
    for entry in
        fs::read_dir(current).map_err(|e| format!("Failed to read {}: {e}", current.display()))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if name == ".git" || name == "target" || name == "node_modules" {
            continue;
        }
        if path.is_dir() {
            collect_files(root, &path, files)?;
        } else if path.is_file() {
            if let Ok(relative) = path.strip_prefix(root) {
                files.push(relative.to_path_buf());
            }
        }
    }
    Ok(())
}

fn read_limited_text(path: &Path, max_bytes: usize) -> Result<String, String> {
    let mut file =
        fs::File::open(path).map_err(|e| format!("Failed to open {}: {e}", path.display()))?;
    let mut buffer = vec![0_u8; max_bytes];
    let len = file
        .read(&mut buffer)
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    buffer.truncate(len);
    if buffer.contains(&0) {
        return Err("binary file skipped".to_string());
    }
    String::from_utf8(buffer).map_err(|_| "non UTF-8 file skipped".to_string())
}

struct CodeFinding {
    kind: &'static str,
    operator: &'static str,
    query_intent: &'static str,
    line: usize,
    snippet: String,
}

fn detect_code_findings(path: &str, content: &str) -> Vec<CodeFinding> {
    let mut findings = Vec::new();
    for (line_index, line) in content.lines().enumerate() {
        let lower = line.to_ascii_lowercase();
        if (lower.contains("sparql") || lower.contains("select "))
            && (lower.contains("where") || lower.contains("query"))
        {
            findings.push(code_finding(
                "SPARQLInjectionCandidate",
                "MutateSparqlConstruction",
                "Find query construction paths where untrusted symbols can enter SPARQL text.",
                line_index,
                line,
            ));
        }
        if (lower.contains("select ") || lower.contains("where "))
            && (line.contains("format!(") || line.contains(" + ") || line.contains("${"))
        {
            findings.push(code_finding(
                "DynamicQueryConstruction",
                "TraceQueryStringFlow",
                "Trace dynamic query assembly and evolve probes around variable interpolation.",
                line_index,
                line,
            ));
        }
        if contains_any(
            &lower,
            &["password", "secret", "api_key", "apikey", "token"],
        ) && contains_any(&lower, &["=", ":", "const", "let", "var"])
        {
            findings.push(code_finding(
                "SecretExposureCandidate",
                "SearchCredentialSurfaces",
                "Cluster credential-like constants for evidence review and sanitization.",
                line_index,
                line,
            ));
        }
        if contains_any(
            &lower,
            &["command::new", "std::process", "system(", "exec(", "popen("],
        ) {
            findings.push(code_finding(
                "CommandExecutionSurface",
                "TraceCommandArguments",
                "Trace command construction and argument provenance under local review.",
                line_index,
                line,
            ));
        }
        if contains_any(
            &lower,
            &["reqwest", "fetch(", "http.get", "requests.get", "urlopen"],
        ) {
            findings.push(code_finding(
                "NetworkRequestSurface",
                "TraceOutboundRequestInputs",
                "Find outbound request construction points for SSRF-style policy review.",
                line_index,
                line,
            ));
        }
        if lower.contains("unsafe") || lower.contains("transmute") || lower.contains("from_raw") {
            findings.push(code_finding(
                "MemorySafetyReviewSurface",
                "TraceUnsafeBoundary",
                "Collect unsafe boundaries for targeted code-audit query evolution.",
                line_index,
                line,
            ));
        }
        if findings.len() >= 80 {
            break;
        }
    }

    if path.ends_with("Cargo.toml") || path.ends_with("package.json") || path.ends_with("pom.xml") {
        findings.push(CodeFinding {
            kind: "DependencyGraphSurface",
            operator: "ExpandDependencyQueries",
            query_intent: "Use manifest files as seeds for dependency and version-risk graph expansion.",
            line: 1,
            snippet: format!("manifest: {path}"),
        });
    }

    findings
}

fn code_finding(
    kind: &'static str,
    operator: &'static str,
    query_intent: &'static str,
    line_index: usize,
    line: &str,
) -> CodeFinding {
    CodeFinding {
        kind,
        operator,
        query_intent,
        line: line_index + 1,
        snippet: line.trim().chars().take(220).collect(),
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn language_for(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or_default() {
        "rs" => "Rust",
        "py" => "Python",
        "js" | "mjs" | "cjs" => "JavaScript",
        "ts" | "tsx" => "TypeScript",
        "java" => "Java",
        "go" => "Go",
        "rb" => "Ruby",
        "php" => "PHP",
        "c" | "h" => "C",
        "cc" | "cpp" | "hpp" => "C++",
        "toml" => "TOML",
        "json" => "JSON",
        "xml" => "XML",
        "md" => "Markdown",
        _ => "Text",
    }
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
        "item".to_string()
    } else {
        slug.chars().take(96).collect()
    }
}

fn stable_hash(value: &str) -> String {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn sparql_term(value: &str) -> String {
    if looks_like_iri(value) {
        format!("<{value}>")
    } else {
        format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

fn looks_like_iri(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://") || value.starts_with("urn:")
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

struct ServerState {
    controller: RdfController,
    sessions: BTreeSet<String>,
    metrics: ServerMetrics,
    state_dir: PathBuf,
}

impl ServerState {
    fn new(state_dir: PathBuf) -> Self {
        Self {
            controller: RdfController::default(),
            sessions: BTreeSet::new(),
            metrics: ServerMetrics::new(),
            state_dir,
        }
    }

    fn log_path(&self) -> PathBuf {
        self.state_dir.join(SRO_LOG_FILE)
    }

    fn append_sro_log(&self, event: &str, detail: &str) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log_path())
            .map_err(|e| format!("Failed to open SRO log: {e}"))?;
        writeln!(
            file,
            "{} [{DEFAULT_TABLE}] {event}: {detail}",
            unix_seconds()
        )
        .map_err(|e| format!("Failed to write SRO log: {e}"))
    }
}

struct ServerMetrics {
    started_at: SystemTime,
    requests: usize,
    signins: usize,
    sparql_queries: usize,
    metrics_requests: usize,
    log_views: usize,
}

impl ServerMetrics {
    fn new() -> Self {
        Self {
            started_at: SystemTime::now(),
            requests: 0,
            signins: 0,
            sparql_queries: 0,
            metrics_requests: 0,
            log_views: 0,
        }
    }

    fn render(&self, sessions: usize, log_path: &Path) -> String {
        let uptime = self
            .started_at
            .elapsed()
            .map_or(0, |duration| duration.as_secs());
        format!(
            "omrya_server_up 1\nomrya_server_uptime_seconds {uptime}\nomrya_requests_total {}\nomrya_signins_total {}\nomrya_sparql_queries_total {}\nomrya_metrics_requests_total {}\nomrya_sro_log_views_total {}\nomrya_sessions {}\nomrya_sro_log_path {}\n",
            self.requests,
            self.signins,
            self.sparql_queries,
            self.metrics_requests,
            self.log_views,
            sessions,
            log_path.display()
        )
    }
}

#[derive(Debug)]
struct CliOptions {
    addr: String,
    state_dir: Option<PathBuf>,
    user: Option<String>,
    password: Option<String>,
    query: Option<String>,
    file: Option<PathBuf>,
    table: Option<String>,
    limit: Option<usize>,
    follow: bool,
    no_open: bool,
}

impl CliOptions {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut opts = Self {
            addr: DEFAULT_ADDR.to_string(),
            state_dir: None,
            user: None,
            password: None,
            query: None,
            file: None,
            table: None,
            limit: None,
            follow: false,
            no_open: false,
        };
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "--addr" => {
                    i += 1;
                    opts.addr = value(args, i, "--addr")?.to_string();
                }
                "--state-dir" => {
                    i += 1;
                    opts.state_dir = Some(PathBuf::from(value(args, i, "--state-dir")?));
                }
                "--user" | "--username" => {
                    i += 1;
                    opts.user = Some(value(args, i, "--user")?.to_string());
                }
                "--password" => {
                    i += 1;
                    opts.password = Some(value(args, i, "--password")?.to_string());
                }
                "--query" => {
                    i += 1;
                    opts.query = Some(value(args, i, "--query")?.to_string());
                }
                "--file" => {
                    i += 1;
                    opts.file = Some(PathBuf::from(value(args, i, "--file")?));
                }
                "--table" => {
                    i += 1;
                    opts.table = Some(value(args, i, "--table")?.to_string());
                }
                "--limit" => {
                    i += 1;
                    opts.limit = Some(
                        value(args, i, "--limit")?
                            .parse()
                            .map_err(|e| format!("Invalid --limit: {e}"))?,
                    );
                }
                "--follow" | "-f" => opts.follow = true,
                "--no-open" => opts.no_open = true,
                "--help" | "-h" => return Err(usage()),
                other => return Err(format!("Unknown option: {other}")),
            }
            i += 1;
        }
        Ok(opts)
    }
}

fn value<'a>(args: &'a [String], index: usize, option: &str) -> Result<&'a str, String> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("{option} requires a value"))
}

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    query: BTreeMap<String, String>,
    headers: BTreeMap<String, String>,
    body: String,
}

fn read_request(stream: &mut TcpStream) -> Result<HttpRequest, String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| format!("Failed to set read timeout: {e}"))?;
    let mut buf = [0_u8; 65536];
    let len = stream
        .read(&mut buf)
        .map_err(|e| format!("Failed to read request: {e}"))?;
    let raw = String::from_utf8_lossy(&buf[..len]);
    let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((&raw, ""));
    let mut lines = head.lines();
    let request_line = lines
        .next()
        .ok_or_else(|| "Missing request line".to_string())?;
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let target = parts.next().unwrap_or("/");
    let (path, query) = split_target(target)?;
    let mut headers = BTreeMap::new();
    for line in lines {
        if let Some((key, value)) = line.split_once(':') {
            headers.insert(key.trim().to_ascii_lowercase(), value.trim().to_string());
        }
    }
    Ok(HttpRequest {
        method,
        path,
        query,
        headers,
        body: body.to_string(),
    })
}

fn split_target(target: &str) -> Result<(String, BTreeMap<String, String>), String> {
    let (path, query_string) = target.split_once('?').unwrap_or((target, ""));
    let mut query = BTreeMap::new();
    if !query_string.is_empty() {
        for pair in query_string.split('&') {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            query.insert(url_decode(key)?, url_decode(value)?);
        }
    }
    Ok((path.to_string(), query))
}

fn write_response(
    stream: &mut TcpStream,
    status: usize,
    content_type: &str,
    body: &str,
) -> Result<(), String> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(response.as_bytes())
        .map_err(|e| format!("Failed to write response: {e}"))
}

struct ClientResponse {
    status: usize,
    body: String,
}

fn request(
    addr: &str,
    method: &str,
    path: &str,
    headers: &[Option<(String, String)>],
    body: &str,
) -> Result<ClientResponse, String> {
    let mut stream =
        TcpStream::connect(addr).map_err(|e| format!("Failed to connect to {addr}: {e}"))?;
    let mut request = format!(
        "{method} {path} HTTP/1.1\r\nHost: {addr}\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (key, value) in headers.iter().flatten() {
        request.push_str(key);
        request.push_str(": ");
        request.push_str(value);
        request.push_str("\r\n");
    }
    request.push_str("\r\n");
    request.push_str(body);
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("Failed to send request: {e}"))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| format!("Failed to read response: {e}"))?;
    let (head, body) = response.split_once("\r\n\r\n").unwrap_or((&response, ""));
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|status| status.parse::<usize>().ok())
        .unwrap_or(500);
    Ok(ClientResponse {
        status,
        body: body.to_string(),
    })
}

fn auth_header(state_dir: Option<&Path>) -> Result<Option<(String, String)>, String> {
    let token_path = state_dir
        .map(Path::to_path_buf)
        .unwrap_or_else(default_state_dir)
        .join(SESSION_FILE);
    if !token_path.exists() {
        return Ok(None);
    }
    let token = fs::read_to_string(&token_path)
        .map_err(|e| format!("Failed to read session token {}: {e}", token_path.display()))?;
    Ok(Some((
        "X-Omrya-Token".to_string(),
        token.trim().to_string(),
    )))
}

fn read_tail(path: &Path, limit: usize) -> Result<String, String> {
    let content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(String::new()),
        Err(error) => return Err(format!("Failed to read {}: {error}", path.display())),
    };
    let lines = content.lines().collect::<Vec<_>>();
    let start = lines.len().saturating_sub(limit);
    Ok(lines[start..].join("\n") + if lines.is_empty() { "" } else { "\n" })
}

fn read_stdin_query() -> Result<String, String> {
    let mut query = String::new();
    std::io::stdin()
        .read_to_string(&mut query)
        .map_err(|e| format!("Failed to read SPARQL from stdin: {e}"))?;
    if query.trim().is_empty() {
        Err("No SPARQL query supplied. Use --query, --file, or stdin.".to_string())
    } else {
        Ok(query)
    }
}

fn make_token(user: &str, password: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in format!("{user}:{password}:{}", unix_nanos()).bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn one_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn default_state_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(env::temp_dir)
        .join(".omrya")
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

fn unix_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos())
}

fn url_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'.' | b'-' | b'_' | b'~' => {
                encoded.push(byte as char)
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn url_decode(value: &str) -> Result<String, String> {
    let mut decoded = Vec::new();
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => decoded.push(b' '),
            b'%' => {
                let hex = bytes
                    .get(i + 1..i + 3)
                    .ok_or_else(|| "Truncated percent encoding".to_string())?;
                let text =
                    std::str::from_utf8(hex).map_err(|e| format!("Invalid percent hex: {e}"))?;
                decoded.push(
                    u8::from_str_radix(text, 16)
                        .map_err(|e| format!("Invalid percent encoding: {e}"))?,
                );
                i += 2;
            }
            byte => decoded.push(byte),
        }
        i += 1;
    }
    String::from_utf8(decoded).map_err(|e| format!("Decoded URL is not UTF-8: {e}"))
}

fn print_usage() {
    println!("{}", usage());
}

fn usage() -> String {
    [
        "Usage:",
        "  omrya server start [--addr 127.0.0.1:7878] [--state-dir PATH]",
        "  omrya web [--addr 127.0.0.1:7878] [--state-dir PATH] [--no-open]",
        "  omrya server metrics [--addr ADDR]",
        "  omrya metrics [--addr ADDR]",
        "  omrya signin --user USER --password PASSWORD [--addr ADDR]",
        "  omrya sparql --query SPARQL [--addr ADDR]",
        "  omrya sparql --file query.rq [--addr ADDR]",
        "  omrya logs [--table sro] [--limit 50] [--follow] [--addr ADDR]",
        "",
        "Rya shell mapping:",
        "  connect/signin mirrors the Rya shell connection step.",
        "  sparql/sparql-query mirrors Rya shell sparql-query.",
        "  web starts the local server and opens /sparql.html.",
        "  logs provides a simple SRO table log viewer.",
    ]
    .join("\n")
}

#[cfg(test)]
#[path = "tests/main_tests.rs"]
mod tests;
