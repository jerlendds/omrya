#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApacheRatCheckConfig {
    pub plugin_group_id: &'static str,
    pub plugin_artifact_id: &'static str,
    pub active_in_default_build: bool,
    pub execution_id: &'static str,
    pub goal: &'static str,
    pub excludes: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApacheRatModuleConfig {
    pub module_path: &'static str,
    pub configured_in_plugin_management: bool,
    pub excludes: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EclipseLifecycleMapping {
    pub plugin_group_id: &'static str,
    pub plugin_artifact_id: &'static str,
    pub plugin_version: &'static str,
    pub affects_maven_build: bool,
    pub ignored_execution: PluginExecutionIgnore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PluginExecutionIgnore {
    pub plugin_group_id: &'static str,
    pub plugin_artifact_id: &'static str,
    pub version_range: &'static str,
    pub goal: &'static str,
    pub action: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JavaSourceLicenseHeader {
    pub path: &'static str,
    pub package_name: &'static str,
    pub header_position: LicenseHeaderPosition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JavaSourceLicenseCorrection {
    pub source: JavaSourceLicenseHeader,
    pub required_header_marker: &'static str,
    pub removed_header_markers: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FileLicenseHeader {
    pub path: &'static str,
    pub comment_style: LicenseCommentStyle,
    pub header_position: FileLicenseHeaderPosition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeletedGeneratedTestArtifact {
    pub path: &'static str,
    pub reason: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LicenseHeaderPosition {
    AfterPackageDeclaration,
    BeforePackageDeclaration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LicenseCommentStyle {
    Html,
    Xml,
    Shell,
    WindowsBatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FileLicenseHeaderPosition {
    FileStart,
    AfterXmlDeclaration,
    AfterShebang,
}

pub const APACHE_RAT_EXCLUDES: &[&str] = &[];

pub const APACHE_RAT_MODULE_CONFIGS: &[ApacheRatModuleConfig] = &[
    ApacheRatModuleConfig {
        module_path: "dao/fjall.rya",
        configured_in_plugin_management: true,
        excludes: &["**/*.ntriples", "**/*.trig"],
    },
    ApacheRatModuleConfig {
        module_path: "extras/indexing",
        configured_in_plugin_management: true,
        excludes: &["**/*.ttl", "**/resources/META-INF/services/**"],
    },
    ApacheRatModuleConfig {
        module_path: "extras/rya.prospector",
        configured_in_plugin_management: true,
        excludes: &["**/resources/META-INF/services/**"],
    },
    ApacheRatModuleConfig {
        module_path: "extras/tinkerpop.rya",
        configured_in_plugin_management: true,
        excludes: &["blueprints.log"],
    },
    ApacheRatModuleConfig {
        module_path: "osgi",
        configured_in_plugin_management: true,
        excludes: &[
            "**/resources/META-INF/services/**",
            "sesame-runtime-osgi/openrdf-sesame-osgi.bnd",
        ],
    },
    ApacheRatModuleConfig {
        module_path: "sail",
        configured_in_plugin_management: true,
        excludes: &[
            "**/*.trig",
            "**/*.owl",
            "**/*.nt",
            "**/resources/META-INF/services/**",
        ],
    },
    ApacheRatModuleConfig {
        module_path: "web/web.rya",
        configured_in_plugin_management: true,
        excludes: &["**/*.trig", "**/*.nt", "**/*.data"],
    },
];

impl ApacheRatModuleConfig {
    pub fn plugin_group_id(&self) -> &'static str {
        "org.apache.rat"
    }

    pub fn plugin_artifact_id(&self) -> &'static str {
        "apache-rat-plugin"
    }
}

pub const TINKERPOP_LOG4J_PATH: &str =
    "extras/tinkerpop.rya/src/test/java/com/tinkerpop/blueprints/impls/sail/log4j.properties";

pub const ASF_SOURCE_LICENSE_MARKER: &str = "Licensed to the Apache Software Foundation (ASF)";
pub const LEGACY_RYA_MAVEN_LICENSE_MARKERS: &[&str] = &[
    "#%L",
    "mvm.rya.rya.sail.impl",
    "Copyright (C) 2014 Rya",
    "#L%",
];

pub const RYA_7_SAIL_LICENSE_CORRECTIONS: &[JavaSourceLicenseCorrection] = &[
    JavaSourceLicenseCorrection {
        source: JavaSourceLicenseHeader {
            path: "extras/indexing/src/main/java/mvm/rya/sail/config/RyaFjallSailConfig.java",
            package_name: "mvm.rya.sail.config",
            header_position: LicenseHeaderPosition::AfterPackageDeclaration,
        },
        required_header_marker: ASF_SOURCE_LICENSE_MARKER,
        removed_header_markers: LEGACY_RYA_MAVEN_LICENSE_MARKERS,
    },
    JavaSourceLicenseCorrection {
        source: JavaSourceLicenseHeader {
            path: "extras/indexing/src/main/java/mvm/rya/sail/config/RyaFjallSailFactory.java",
            package_name: "mvm.rya.sail.config",
            header_position: LicenseHeaderPosition::AfterPackageDeclaration,
        },
        required_header_marker: ASF_SOURCE_LICENSE_MARKER,
        removed_header_markers: LEGACY_RYA_MAVEN_LICENSE_MARKERS,
    },
];

pub const RYA_32_PCJ_LICENSE_HEADERS: &[JavaSourceLicenseHeader] = &[
    JavaSourceLicenseHeader {
        path: "extras/indexing/src/main/java/mvm/rya/indexing/external/tupleSet/FjallPcjSerializer.java",
        package_name: "mvm.rya.indexing.external.tupleSet",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/indexing/src/main/java/mvm/rya/indexing/external/tupleSet/PcjTables.java",
        package_name: "mvm.rya.indexing.external.tupleSet",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/external/PcjIntegrationTestingUtil.java",
        package_name: "mvm.rya.indexing.external",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/external/tupleSet/FjallIndexSetTest.java",
        package_name: "mvm.rya.indexing.external.tupleSet",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/external/tupleSet/FjallPcjSerialzerTest.java",
        package_name: "mvm.rya.indexing.external.tupleSet",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/external/tupleSet/PcjTablesIntegrationTests.java",
        package_name: "mvm.rya.indexing.external.tupleSet",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/external/tupleSet/PcjTablesTests.java",
        package_name: "mvm.rya.indexing.external.tupleSet",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
];

pub const COMMIT_58_INDEXING_TEST_LICENSE_HEADERS: &[JavaSourceLicenseHeader] = &[
    JavaSourceLicenseHeader {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/external/tupleSet/FjallIndexSetColumnVisibilityTest.java",
        package_name: "mvm.rya.indexing.external.tupleSet",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/pcj/matching/FlattenedOptionalTest.java",
        package_name: "mvm.rya.indexing.pcj.matching",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/pcj/matching/JoinSegmentPCJMatcherTest.java",
        package_name: "mvm.rya.indexing.pcj.matching",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/pcj/matching/JoinSegmentTest.java",
        package_name: "mvm.rya.indexing.pcj.matching",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/pcj/matching/OptionalJoinSegmentPCJMatcherTest.java",
        package_name: "mvm.rya.indexing.pcj.matching",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/pcj/matching/OptionalJoinSegmentTest.java",
        package_name: "mvm.rya.indexing.pcj.matching",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/pcj/matching/PCJNodeConsolidatorTest.java",
        package_name: "mvm.rya.indexing.pcj.matching",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/pcj/matching/PCJOptimizerTest.java",
        package_name: "mvm.rya.indexing.pcj.matching",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/indexing/src/test/java/mvm/rya/indexing/pcj/matching/PCJOptimizerUtilitesTest.java",
        package_name: "mvm.rya.indexing.pcj.matching",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
];

pub const COMMIT_79_API_SOURCE_LICENSE_HEADERS: &[JavaSourceLicenseHeader] =
    &[JavaSourceLicenseHeader {
        path: "common/rya.api/src/main/java/mvm/rya/api/instance/RyaDetailsToConfiguration.java",
        package_name: "mvm.rya.api.instance",
        header_position: LicenseHeaderPosition::BeforePackageDeclaration,
    }];

pub const COMMIT_74_RYA_MERGER_JAVA_LICENSE_HEADERS: &[JavaSourceLicenseHeader] = &[
    JavaSourceLicenseHeader {
        path: "extras/rya.merger/src/test/java/mvm/rya/fjall/mr/merge/CopyToolTest.java",
        package_name: "mvm.rya.fjall.mr.merge",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/rya.merger/src/test/java/mvm/rya/fjall/mr/merge/MergeToolTest.java",
        package_name: "mvm.rya.fjall.mr.merge",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/rya.merger/src/test/java/mvm/rya/fjall/mr/merge/demo/CopyToolDemo.java",
        package_name: "mvm.rya.fjall.mr.merge.demo",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/rya.merger/src/test/java/mvm/rya/fjall/mr/merge/demo/MergeToolDemo.java",
        package_name: "mvm.rya.fjall.mr.merge.demo",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/rya.merger/src/test/java/mvm/rya/fjall/mr/merge/demo/util/DemoUtilities.java",
        package_name: "mvm.rya.fjall.mr.merge.demo.util",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/rya.merger/src/test/java/mvm/rya/fjall/mr/merge/driver/FjallDualInstanceDriver.java",
        package_name: "mvm.rya.fjall.mr.merge.driver",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/rya.merger/src/test/java/mvm/rya/fjall/mr/merge/driver/MiniFjallClusterDriver.java",
        package_name: "mvm.rya.fjall.mr.merge.driver",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
    JavaSourceLicenseHeader {
        path: "extras/rya.merger/src/test/java/mvm/rya/fjall/mr/merge/util/TestUtils.java",
        package_name: "mvm.rya.fjall.mr.merge.util",
        header_position: LicenseHeaderPosition::AfterPackageDeclaration,
    },
];

pub const COMMIT_74_RYA_MERGER_FILE_LICENSE_HEADERS: &[FileLicenseHeader] = &[
    FileLicenseHeader {
        path: "extras/rya.merger/README.md",
        comment_style: LicenseCommentStyle::Html,
        header_position: FileLicenseHeaderPosition::FileStart,
    },
    FileLicenseHeader {
        path: "extras/rya.merger/config/configuration.xml",
        comment_style: LicenseCommentStyle::Xml,
        header_position: FileLicenseHeaderPosition::AfterXmlDeclaration,
    },
    FileLicenseHeader {
        path: "extras/rya.merger/config/copy_tool_configuration.xml",
        comment_style: LicenseCommentStyle::Xml,
        header_position: FileLicenseHeaderPosition::AfterXmlDeclaration,
    },
    FileLicenseHeader {
        path: "extras/rya.merger/config/copy_tool_log4j.xml",
        comment_style: LicenseCommentStyle::Xml,
        header_position: FileLicenseHeaderPosition::AfterXmlDeclaration,
    },
    FileLicenseHeader {
        path: "extras/rya.merger/config/log4j.xml",
        comment_style: LicenseCommentStyle::Xml,
        header_position: FileLicenseHeaderPosition::AfterXmlDeclaration,
    },
    FileLicenseHeader {
        path: "extras/rya.merger/startup_scripts/copy_tool.bat",
        comment_style: LicenseCommentStyle::WindowsBatch,
        header_position: FileLicenseHeaderPosition::FileStart,
    },
    FileLicenseHeader {
        path: "extras/rya.merger/startup_scripts/copy_tool.sh",
        comment_style: LicenseCommentStyle::Shell,
        header_position: FileLicenseHeaderPosition::AfterShebang,
    },
    FileLicenseHeader {
        path: "extras/rya.merger/startup_scripts/hadoop_copy_tool.bat",
        comment_style: LicenseCommentStyle::WindowsBatch,
        header_position: FileLicenseHeaderPosition::FileStart,
    },
    FileLicenseHeader {
        path: "extras/rya.merger/startup_scripts/hadoop_copy_tool.sh",
        comment_style: LicenseCommentStyle::Shell,
        header_position: FileLicenseHeaderPosition::AfterShebang,
    },
    FileLicenseHeader {
        path: "extras/rya.merger/startup_scripts/hadoop_merge_tool.bat",
        comment_style: LicenseCommentStyle::WindowsBatch,
        header_position: FileLicenseHeaderPosition::FileStart,
    },
    FileLicenseHeader {
        path: "extras/rya.merger/startup_scripts/hadoop_merge_tool.sh",
        comment_style: LicenseCommentStyle::Shell,
        header_position: FileLicenseHeaderPosition::AfterShebang,
    },
    FileLicenseHeader {
        path: "extras/rya.merger/startup_scripts/merge_tool.bat",
        comment_style: LicenseCommentStyle::WindowsBatch,
        header_position: FileLicenseHeaderPosition::FileStart,
    },
    FileLicenseHeader {
        path: "extras/rya.merger/startup_scripts/merge_tool.sh",
        comment_style: LicenseCommentStyle::Shell,
        header_position: FileLicenseHeaderPosition::AfterShebang,
    },
];

pub const COMMIT_74_REMOVED_GENERATED_TEST_ARTIFACTS: &[DeletedGeneratedTestArtifact] = &[
    DeletedGeneratedTestArtifact {
        path: "extras/rya.merger/resources/test/merge_tool_file_input/ct_spo/files/._SUCCESS.crc",
        reason: "generated Hadoop checksum marker without a license header",
    },
    DeletedGeneratedTestArtifact {
        path: "extras/rya.merger/resources/test/merge_tool_file_input/ct_spo/files/_SUCCESS",
        reason: "generated Hadoop success marker without a license header",
    },
];

pub fn apache_rat_check_config() -> ApacheRatCheckConfig {
    ApacheRatCheckConfig {
        plugin_group_id: "org.apache.rat",
        plugin_artifact_id: "apache-rat-plugin",
        active_in_default_build: true,
        execution_id: "check-licenses",
        goal: "check",
        excludes: APACHE_RAT_EXCLUDES,
    }
}

pub fn apache_rat_eclipse_lifecycle_mapping() -> EclipseLifecycleMapping {
    EclipseLifecycleMapping {
        plugin_group_id: "org.eclipse.m2e",
        plugin_artifact_id: "lifecycle-mapping",
        plugin_version: "1.0.0",
        affects_maven_build: false,
        ignored_execution: PluginExecutionIgnore {
            plugin_group_id: "org.apache.rat",
            plugin_artifact_id: "apache-rat-plugin",
            version_range: "[0.11,)",
            goal: "check",
            action: "ignore",
        },
    }
}

pub fn eclipse_mapping_ignores_execution(
    group_id: &str,
    artifact_id: &str,
    version: &str,
    goal: &str,
) -> bool {
    let mapping = apache_rat_eclipse_lifecycle_mapping();
    let ignored = mapping.ignored_execution;

    !mapping.affects_maven_build
        && group_id == ignored.plugin_group_id
        && artifact_id == ignored.plugin_artifact_id
        && goal == ignored.goal
        && ignored.action == "ignore"
        && version_satisfies_lower_bound(version, "0.11")
}

pub fn is_apache_rat_excluded(path: &str) -> bool {
    APACHE_RAT_EXCLUDES
        .iter()
        .any(|pattern| rat_pattern_matches(pattern, path))
}

pub fn apache_rat_module_configs() -> &'static [ApacheRatModuleConfig] {
    APACHE_RAT_MODULE_CONFIGS
}

pub fn apache_rat_module_config(module_path: &str) -> Option<ApacheRatModuleConfig> {
    let normalized = normalize_path(module_path);

    APACHE_RAT_MODULE_CONFIGS
        .iter()
        .copied()
        .find(|config| config.module_path == normalized)
}

pub fn is_apache_rat_excluded_for_module(module_path: &str, path: &str) -> bool {
    apache_rat_module_config(module_path).is_some_and(|config| {
        config
            .excludes
            .iter()
            .any(|pattern| rat_pattern_matches(pattern, path))
    })
}

pub fn rya_7_sail_license_corrections() -> &'static [JavaSourceLicenseCorrection] {
    RYA_7_SAIL_LICENSE_CORRECTIONS
}

pub fn rya_7_license_correction_for_path(path: &str) -> Option<JavaSourceLicenseCorrection> {
    let normalized = path.replace('\\', "/");

    RYA_7_SAIL_LICENSE_CORRECTIONS
        .iter()
        .copied()
        .find(|correction| correction.source.path == normalized)
}

pub fn rya_32_pcj_license_headers() -> &'static [JavaSourceLicenseHeader] {
    RYA_32_PCJ_LICENSE_HEADERS
}

pub fn rya_32_license_header_for_path(path: &str) -> Option<JavaSourceLicenseHeader> {
    let normalized = path.replace('\\', "/");

    RYA_32_PCJ_LICENSE_HEADERS
        .iter()
        .copied()
        .find(|header| header.path == normalized)
}

pub fn commit_58_indexing_test_license_headers() -> &'static [JavaSourceLicenseHeader] {
    COMMIT_58_INDEXING_TEST_LICENSE_HEADERS
}

pub fn commit_58_license_header_for_path(path: &str) -> Option<JavaSourceLicenseHeader> {
    let normalized = path.replace('\\', "/");

    COMMIT_58_INDEXING_TEST_LICENSE_HEADERS
        .iter()
        .copied()
        .find(|header| header.path == normalized)
}

pub fn commit_79_api_source_license_headers() -> &'static [JavaSourceLicenseHeader] {
    COMMIT_79_API_SOURCE_LICENSE_HEADERS
}

pub fn commit_79_api_source_license_header_for_path(path: &str) -> Option<JavaSourceLicenseHeader> {
    let normalized = path.replace('\\', "/");

    COMMIT_79_API_SOURCE_LICENSE_HEADERS
        .iter()
        .copied()
        .find(|header| header.path == normalized)
}

pub fn commit_74_rya_merger_java_license_headers() -> &'static [JavaSourceLicenseHeader] {
    COMMIT_74_RYA_MERGER_JAVA_LICENSE_HEADERS
}

pub fn commit_74_rya_merger_java_license_header_for_path(
    path: &str,
) -> Option<JavaSourceLicenseHeader> {
    let normalized = path.replace('\\', "/");

    COMMIT_74_RYA_MERGER_JAVA_LICENSE_HEADERS
        .iter()
        .copied()
        .find(|header| header.path == normalized)
}

pub fn commit_74_rya_merger_file_license_headers() -> &'static [FileLicenseHeader] {
    COMMIT_74_RYA_MERGER_FILE_LICENSE_HEADERS
}

pub fn commit_74_rya_merger_file_license_header_for_path(path: &str) -> Option<FileLicenseHeader> {
    let normalized = path.replace('\\', "/");

    COMMIT_74_RYA_MERGER_FILE_LICENSE_HEADERS
        .iter()
        .copied()
        .find(|header| header.path == normalized)
}

pub fn commit_74_removed_generated_test_artifacts() -> &'static [DeletedGeneratedTestArtifact] {
    COMMIT_74_REMOVED_GENERATED_TEST_ARTIFACTS
}

pub fn commit_74_removed_generated_test_artifact_for_path(
    path: &str,
) -> Option<DeletedGeneratedTestArtifact> {
    let normalized = path.replace('\\', "/");

    COMMIT_74_REMOVED_GENERATED_TEST_ARTIFACTS
        .iter()
        .copied()
        .find(|artifact| artifact.path == normalized)
}

pub fn tinkerpop_log4j_override_removed() -> bool {
    true
}

pub fn tinkerpop_rat_excludes_blueprints_log(path: &str) -> bool {
    is_apache_rat_excluded_for_module("extras/tinkerpop.rya", path)
}

fn version_satisfies_lower_bound(version: &str, lower_bound: &str) -> bool {
    match (
        parse_dotted_version(version),
        parse_dotted_version(lower_bound),
    ) {
        (Some(version), Some(lower_bound)) => version >= lower_bound,
        _ => false,
    }
}

fn parse_dotted_version(version: &str) -> Option<Vec<u64>> {
    let base = version.split_once('-').map_or(version, |(base, _)| base);
    let parts = base
        .split('.')
        .map(str::parse)
        .collect::<Result<Vec<u64>, _>>()
        .ok()?;

    if parts.is_empty() {
        return None;
    }

    Some(parts)
}

fn normalize_path(path: &str) -> String {
    path.trim_start_matches("./").replace('\\', "/")
}

fn rat_pattern_matches(pattern: &str, path: &str) -> bool {
    let normalized = normalize_path(path);
    let lower = normalized.to_ascii_lowercase();

    match pattern {
        "**/*.ntriples" => lower.ends_with(".ntriples"),
        "**/*.trig" => lower.ends_with(".trig"),
        "**/*.ttl" => lower.ends_with(".ttl"),
        "**/*.owl" => lower.ends_with(".owl"),
        "**/*.nt" => lower.ends_with(".nt"),
        "**/*.data" => lower.ends_with(".data"),
        "**/resources/META-INF/services/**" => {
            normalized.contains("/resources/META-INF/services/")
                || normalized.starts_with("resources/META-INF/services/")
        }
        "blueprints.log" => {
            normalized == "blueprints.log" || normalized.ends_with("/blueprints.log")
        }
        "sesame-runtime-osgi/openrdf-sesame-osgi.bnd" => {
            normalized == "sesame-runtime-osgi/openrdf-sesame-osgi.bnd"
        }
        _ => normalized == *pattern,
    }
}

#[cfg(test)]
#[path = "tests/release_audit_tests.rs"]
mod tests;
