#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetiredJavaModule {
    pub root: &'static str,
    pub reason: &'static str,
    pub retained_rust_fixture: Option<&'static str>,
}

pub const RYA_7_COMMIT: &str = "80faf06d48165e65271a52856238896521fcf218";
pub const RYA_7_SUBJECT: &str = "RYA-7 POM and License Clean-up for Apache Move";

pub const RYA_7_RETIRED_PARTITION_MODULES: &[RetiredJavaModule] = &[
    RetiredJavaModule {
        root: "partition/common-query",
        reason: "legacy Cloudbase iterator and filter module removed during Apache move cleanup",
        retained_rust_fixture: Some("src/partition.rs"),
    },
    RetiredJavaModule {
        root: "partition/iterator-test",
        reason: "ad hoc Cloudbase iterator test harness removed during Apache move cleanup",
        retained_rust_fixture: None,
    },
    RetiredJavaModule {
        root: "partition/mr.partition.rdf",
        reason: "legacy Hadoop MapReduce partition RDF module removed during Apache move cleanup",
        retained_rust_fixture: Some("src/partition.rs"),
    },
    RetiredJavaModule {
        root: "partition/partition.rdf",
        reason: "legacy Sesame/Cloudbase partition Sail module removed during Apache move cleanup",
        retained_rust_fixture: Some("src/partition.rs"),
    },
];

pub const RYA_7_DELETED_TEST_PATHS_IN_PART_3: &[&str] = &[
    "partition/common-query/src/test/java/GVDateFilterTest.java",
    "partition/common-query/src/test/java/GVFrequencyFilterTest.java",
    "partition/common-query/src/test/java/IteratorTest.java",
    "partition/common-query/src/test/java/JTSFilterTest.java",
    "partition/common-query/src/test/java/OGCFilterTest.java",
    "partition/mr.partition.rdf/src/test/java/mvm/mmrts/rdf/partition/mr/compat/ChangeShardDateFormatToolTest.java",
    "partition/mr.partition.rdf/src/test/java/mvm/mmrts/rdf/partition/mr/fileinput/RdfFileInputToolTest.java",
    "partition/mr.partition.rdf/src/test/java/mvm/mmrts/rdf/partition/mr/fileinput/bulk/EmbedKeyRangePartitionerTest.java",
    "partition/partition.rdf/src/test/java/mvm/mmrts/rdf/partition/PartitionConnectionTest.java",
    "partition/partition.rdf/src/test/java/mvm/mmrts/rdf/partition/shard/DateHashModShardValueGeneratorTest.java",
    "partition/partition.rdf/src/test/java/mvm/mmrts/rdf/partition/utils/RdfIOTest.java",
];

pub fn rya7_retired_module_for_path(path: &str) -> Option<&'static RetiredJavaModule> {
    let path = normalize_repo_path(path);
    RYA_7_RETIRED_PARTITION_MODULES.iter().find(|module| {
        path == module.root
            || path.starts_with(module.root) && path[module.root.len()..].starts_with('/')
    })
}

pub fn is_rya7_deleted_test_path(path: &str) -> bool {
    let path = normalize_repo_path(path);
    RYA_7_DELETED_TEST_PATHS_IN_PART_3.contains(&path)
}

pub fn retained_fixture_for_rya7_path(path: &str) -> Option<&'static str> {
    rya7_retired_module_for_path(path).and_then(|module| module.retained_rust_fixture)
}

fn normalize_repo_path(path: &str) -> &str {
    path.strip_prefix("./").unwrap_or(path)
}

#[cfg(test)]
#[path = "tests/legacy_tests.rs"]
mod tests;
