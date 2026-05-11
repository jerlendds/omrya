#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JavaCompileFixKind {
    PropagatesInferenceEngineException,
    UsesSailConfigFactoryImport,
    RemovesStaleDuplicatePcjTest,
    RemovesMergeConflictDuplicateBlock,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JavaCompileFix {
    pub path: &'static str,
    pub symbol: &'static str,
    pub kind: JavaCompileFixKind,
    pub retained_rust_fixture: Option<&'static str>,
}

pub const RYA_11_COMPILE_FIX_COMMIT: &str = "358c13b83dc04887f95afee1c326971602d31647";
pub const RYA_11_COMPILE_FIX_SUBJECT: &str = "RYA-11 fixing compile errors";
pub const MERGE_CONFLICT_FIX_COMMIT: &str = "8ac8b394aefd51be7b9e854a1e14db48d9152e2e";
pub const RYA_SAIL_CONFIG_FACTORY_IMPORT: &str = "mvm.rya.sail.config.RyaSailFactory";
pub const REMOVED_INDEXING_FACTORY_IMPORT: &str = "mvm.rya.indexing.RyaSailFactory";
pub const REMOVED_DIRECT_AUTH_EXAMPLE_STEP: &str =
    "Running SPARQL Example: Add with Visibilities and Query with Authorizations";
pub const RYA_DIRECT_EXAMPLE_COMMIT_64_STEPS: &[&str] = &[
    "Running SPARQL Example: Add and Delete",
    "Running SAIL/SPARQL Example: PCJ Search",
    "Running SAIL/SPARQL Example: Add and Temporal Search",
    "Running SAIL/SPARQL Example: Add and Free Text Search",
    "Running SAIL/SPARQL Example: Add Point and Geo Search",
    "Running SAIL/SPARQL Example: Add and Free Text Search with PCJ",
    "Running SPARQL Example: Add Point and Geo Search with PCJ",
    "Running SPARQL Example: Temporal, Freetext, and Geo Search",
    "Running SPARQL Example: Geo, Freetext, and PCJ Search",
    "Running SPARQL Example: Delete Temporal Data",
    "Running SPARQL Example: Delete Free Text Data",
    "Running SPARQL Example: Delete Geo Data",
];

pub const RYA_11_COMPILE_FIXES: &[JavaCompileFix] = &[
    JavaCompileFix {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/external/FjallConstantPcjIntegrationTest.java",
        symbol: "init",
        kind: JavaCompileFixKind::PropagatesInferenceEngineException,
        retained_rust_fixture: Some("src/pcj.rs"),
    },
    JavaCompileFix {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/external/FjallPcjIntegrationTest.java",
        symbol: "init",
        kind: JavaCompileFixKind::PropagatesInferenceEngineException,
        retained_rust_fixture: Some("src/pcj.rs"),
    },
    JavaCompileFix {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/external/PcjIntegrationTestingUtil.java",
        symbol: "getPcjRepo",
        kind: JavaCompileFixKind::PropagatesInferenceEngineException,
        retained_rust_fixture: Some("src/store.rs"),
    },
    JavaCompileFix {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/external/PcjIntegrationTestingUtil.java",
        symbol: "getNonPcjRepo",
        kind: JavaCompileFixKind::PropagatesInferenceEngineException,
        retained_rust_fixture: Some("src/store.rs"),
    },
    JavaCompileFix {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/external/PrecompJoinOptimizerIntegrationTest.java",
        symbol: "test methods creating PCJ repositories",
        kind: JavaCompileFixKind::PropagatesInferenceEngineException,
        retained_rust_fixture: Some("src/pcj.rs"),
    },
    JavaCompileFix {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/external/PrecompJoinOptimizerIntegrationTest.java",
        symbol: "stale merge-conflict test body",
        kind: JavaCompileFixKind::RemovesMergeConflictDuplicateBlock,
        retained_rust_fixture: Some("src/pcj.rs"),
    },
    JavaCompileFix {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/external/tupleSet/FjallIndexSetTest.java",
        symbol: "init",
        kind: JavaCompileFixKind::PropagatesInferenceEngineException,
        retained_rust_fixture: Some("src/pcj.rs"),
    },
    JavaCompileFix {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/external/FjallIndexSetTest2.java",
        symbol: "class",
        kind: JavaCompileFixKind::RemovesStaleDuplicatePcjTest,
        retained_rust_fixture: Some("src/pcj.rs"),
    },
    JavaCompileFix {
        path: "extras/indexingExample/src/main/java/RyaDirectExample.java",
        symbol: "RyaSailFactory import",
        kind: JavaCompileFixKind::UsesSailConfigFactoryImport,
        retained_rust_fixture: Some("src/store.rs"),
    },
    JavaCompileFix {
        path: "extras/indexingExample/src/main/java/RyaDirectExample.java",
        symbol: "createPCJ",
        kind: JavaCompileFixKind::PropagatesInferenceEngineException,
        retained_rust_fixture: Some("src/pcj.rs"),
    },
];

pub fn compile_fixes_for_path(path: &str) -> Vec<&'static JavaCompileFix> {
    let path = normalize_repo_path(path);
    RYA_11_COMPILE_FIXES
        .iter()
        .filter(|fix| fix.path == path)
        .collect()
}

pub fn requires_inference_engine_exception(path: &str, symbol: &str) -> bool {
    compile_fixes_for_path(path).into_iter().any(|fix| {
        fix.kind == JavaCompileFixKind::PropagatesInferenceEngineException && fix.symbol == symbol
    })
}

pub fn uses_sail_config_factory_import(path: &str) -> bool {
    compile_fixes_for_path(path)
        .into_iter()
        .any(|fix| fix.kind == JavaCompileFixKind::UsesSailConfigFactoryImport)
}

pub fn is_removed_stale_pcj_test(path: &str) -> bool {
    compile_fixes_for_path(path)
        .into_iter()
        .any(|fix| fix.kind == JavaCompileFixKind::RemovesStaleDuplicatePcjTest)
}

pub fn retained_fixture_for_compile_fix(path: &str) -> Option<&'static str> {
    compile_fixes_for_path(path)
        .into_iter()
        .find_map(|fix| fix.retained_rust_fixture)
}

fn normalize_repo_path(path: &str) -> &str {
    path.strip_prefix("./").unwrap_or(path)
}

#[cfg(test)]
#[path = "tests/compile_fixes_tests.rs"]
mod tests;
