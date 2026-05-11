pub const RYA_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MAVEN_MIN_VERSION: &str = "3.0.4";
pub const JAVA_SOURCE_VERSION: &str = "1.7";
pub const JAVA_TARGET_VERSION: &str = "1.7";
pub const FJALL_VERSION: &str = "1.6.4";
pub const GEOMESA_VERSION: &str = "1.2.0";
pub const GEOMESA_RUNTIME_GROUP_ID: &str = "org.locationtech.geomesa";
pub const GEOMESA_FJALL_RUNTIME_ARTIFACT_ID: &str = "geomesa-fjall-distributed-runtime";
pub const LEGACY_GEOMESA_RUNTIME_ARTIFACT_ID: &str = "geomesa-distributed-runtime";
pub const ZOOKEEPER_VERSION: &str = "3.4.6";
pub const ZOOKEEPER_GROUP_ID: &str = "org.apache.zookeeper";
pub const ZOOKEEPER_ARTIFACT_ID: &str = "zookeeper";
pub const RYA_MAPREDUCE_MODULE: &str = "mapreduce";
pub const RYA_MAPREDUCE_ARTIFACT_ID: &str = "rya.mapreduce";
pub const RYA_MERGER_MODULE: &str = "extras/rya.merger";
pub const RYA_MERGER_ARTIFACT_ID: &str = "rya.merger";
pub const FJALL_RYA_MODULE: &str = "dao/fjall.rya";
pub const FJALL_RYA_ARTIFACT_ID: &str = "fjall.rya";
pub const ANIMAL_SNIFFER_GROUP_ID: &str = "org.codehaus.mojo";
pub const ANIMAL_SNIFFER_ARTIFACT_ID: &str = "animal-sniffer-maven-plugin";
pub const ANIMAL_SNIFFER_VERSION: &str = "1.15";
pub const ANIMAL_SNIFFER_SIGNATURE_GROUP_ID: &str = "org.codehaus.mojo.signature";
pub const ANIMAL_SNIFFER_SIGNATURE_ARTIFACT_ID: &str = "java18";
pub const ANIMAL_SNIFFER_SIGNATURE_VERSION: &str = "1.0";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MavenRepository {
    pub id: &'static str,
    pub url: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MavenDependency {
    pub group_id: &'static str,
    pub artifact_id: &'static str,
    pub version: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MavenScopedDependency {
    pub group_id: &'static str,
    pub artifact_id: &'static str,
    pub version: &'static str,
    pub scope: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MavenClassifiedScopedDependency {
    pub group_id: &'static str,
    pub artifact_id: &'static str,
    pub version: &'static str,
    pub classifier: Option<&'static str>,
    pub scope: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AssemblyDependencySet {
    pub output_directory: &'static str,
    pub includes: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MavenPluginExecution {
    pub group_id: &'static str,
    pub artifact_id: &'static str,
    pub version: &'static str,
    pub goals: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MavenRatConfiguration {
    pub excludes: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MavenShadeProfile {
    pub id: &'static str,
    pub transformer: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManagedMavenPlugin {
    pub group_id: &'static str,
    pub artifact_id: &'static str,
    pub version: &'static str,
    pub signature: Option<MavenDependency>,
    pub execution_phase: Option<&'static str>,
    pub goals: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnabledMavenPlugin {
    pub group_id: &'static str,
    pub artifact_id: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MavenModule {
    pub path: &'static str,
    pub artifact_id: &'static str,
    pub name: &'static str,
}

const INDEXING_EXAMPLE_FJALL_EXT_INCLUDES: &[&str] = &[
    "org.apache.rya:rya.indexing:*:fjall-server",
    "org.locationtech.geomesa:geomesa-fjall-distributed-runtime:*",
];

const FJALL_RYA_FAILSAFE_GOALS: &[&str] = &["integration-test", "verify"];
const ANIMAL_SNIFFER_GOALS: &[&str] = &["check"];
const FJALL_RYA_RAT_EXCLUDES: &[&str] = &["**/*.ntriples", "**/*.trig"];

const LOCATIONTECH_REPOSITORIES: &[MavenRepository] = &[
    MavenRepository {
        id: "LocationTech - SNAPSHOT",
        url: "https://repo.locationtech.org/content/repositories/snapshots/",
    },
    MavenRepository {
        id: "LocationTech - RELEASE",
        url: "https://repo.locationtech.org/content/repositories/releases/",
    },
    MavenRepository {
        id: "LocationTech - Third Party",
        url: "https://repo.locationtech.org/content/repositories/thirdparty/",
    },
];

const TOP_LEVEL_MODULES: &[&str] = &[
    "common",
    "dao",
    "extras",
    RYA_MAPREDUCE_MODULE,
    "osgi",
    "pig",
    "sail",
    "spark",
    "test",
    "web",
];

const RYA_MAPREDUCE_DEPENDENCIES: &[MavenDependency] = &[
    MavenDependency {
        group_id: "org.apache.rya",
        artifact_id: "rya.api",
        version: RYA_VERSION,
    },
    MavenDependency {
        group_id: "org.apache.rya",
        artifact_id: "fjall.rya",
        version: RYA_VERSION,
    },
    MavenDependency {
        group_id: "org.apache.rya",
        artifact_id: "rya.indexing",
        version: RYA_VERSION,
    },
    MavenDependency {
        group_id: "org.apache.fjall",
        artifact_id: "fjall-core",
        version: FJALL_VERSION,
    },
    MavenDependency {
        group_id: "commons-lang",
        artifact_id: "commons-lang",
        version: "${commons.lang.version}",
    },
    MavenDependency {
        group_id: "org.openrdf.sesame",
        artifact_id: "sesame-rio-ntriples",
        version: "${openrdf.sesame.version}",
    },
    MavenDependency {
        group_id: "org.openrdf.sesame",
        artifact_id: "sesame-rio-nquads",
        version: "${openrdf.sesame.version}",
    },
];

const FJALL_RYA_RUNTIME_DEPENDENCIES: &[MavenDependency] = &[
    MavenDependency {
        group_id: "org.apache.rya",
        artifact_id: "rya.api",
        version: RYA_VERSION,
    },
    MavenDependency {
        group_id: "org.apache.fjall",
        artifact_id: "fjall-core",
        version: FJALL_VERSION,
    },
    MavenDependency {
        group_id: "org.openrdf.sesame",
        artifact_id: "sesame-rio-ntriples",
        version: "${openrdf.sesame.version}",
    },
    MavenDependency {
        group_id: "org.openrdf.sesame",
        artifact_id: "sesame-rio-nquads",
        version: "${openrdf.sesame.version}",
    },
    MavenDependency {
        group_id: "org.openrdf.sesame",
        artifact_id: "sesame-queryalgebra-evaluation",
        version: "${openrdf.sesame.version}",
    },
];

const FJALL_RYA_TEST_DEPENDENCIES: &[MavenClassifiedScopedDependency] = &[
    MavenClassifiedScopedDependency {
        group_id: "org.openrdf.sesame",
        artifact_id: "sesame-rio-trig",
        version: "${openrdf.sesame.version}",
        classifier: None,
        scope: "test",
    },
    MavenClassifiedScopedDependency {
        group_id: "junit",
        artifact_id: "junit",
        version: "${junit.version}",
        classifier: None,
        scope: "test",
    },
    MavenClassifiedScopedDependency {
        group_id: "org.apache.mrunit",
        artifact_id: "mrunit",
        version: "1.1.0",
        classifier: Some("hadoop2"),
        scope: "test",
    },
    MavenClassifiedScopedDependency {
        group_id: "org.apache.fjall",
        artifact_id: "fjall-minicluster",
        version: FJALL_VERSION,
        classifier: None,
        scope: "test",
    },
];

pub fn rya_version() -> &'static str {
    RYA_VERSION
}

pub fn maven_min_version() -> &'static str {
    MAVEN_MIN_VERSION
}

pub fn java_source_version() -> &'static str {
    JAVA_SOURCE_VERSION
}

pub fn java_target_version() -> &'static str {
    JAVA_TARGET_VERSION
}

pub fn geomesa_version() -> &'static str {
    GEOMESA_VERSION
}

pub fn fjall_version() -> &'static str {
    FJALL_VERSION
}

pub fn zookeeper_version() -> &'static str {
    ZOOKEEPER_VERSION
}

pub fn locationtech_repositories() -> &'static [MavenRepository] {
    LOCATIONTECH_REPOSITORIES
}

pub fn top_level_modules() -> &'static [&'static str] {
    TOP_LEVEL_MODULES
}

pub fn rya_mapreduce_module() -> MavenModule {
    MavenModule {
        path: RYA_MAPREDUCE_MODULE,
        artifact_id: RYA_MAPREDUCE_ARTIFACT_ID,
        name: "Apache Rya MapReduce Tools",
    }
}

pub fn rya_mapreduce_dependency() -> MavenDependency {
    MavenDependency {
        group_id: "org.apache.rya",
        artifact_id: RYA_MAPREDUCE_ARTIFACT_ID,
        version: RYA_VERSION,
    }
}

pub fn rya_mapreduce_dependencies() -> &'static [MavenDependency] {
    RYA_MAPREDUCE_DEPENDENCIES
}

pub fn rya_merger_module() -> MavenModule {
    MavenModule {
        path: RYA_MERGER_MODULE,
        artifact_id: RYA_MERGER_ARTIFACT_ID,
        name: "Apache Rya Merger Tools",
    }
}

pub fn rya_merger_dependency() -> MavenDependency {
    MavenDependency {
        group_id: "org.apache.rya",
        artifact_id: RYA_MERGER_ARTIFACT_ID,
        version: RYA_VERSION,
    }
}

pub fn fjall_rya_module() -> MavenModule {
    MavenModule {
        path: FJALL_RYA_MODULE,
        artifact_id: FJALL_RYA_ARTIFACT_ID,
        name: "Apache Rya Fjall DAO",
    }
}

pub fn fjall_rya_runtime_dependencies() -> &'static [MavenDependency] {
    FJALL_RYA_RUNTIME_DEPENDENCIES
}

pub fn fjall_rya_test_dependencies() -> &'static [MavenClassifiedScopedDependency] {
    FJALL_RYA_TEST_DEPENDENCIES
}

pub fn geomesa_fjall_runtime_dependency() -> MavenDependency {
    MavenDependency {
        group_id: GEOMESA_RUNTIME_GROUP_ID,
        artifact_id: GEOMESA_FJALL_RUNTIME_ARTIFACT_ID,
        version: GEOMESA_VERSION,
    }
}

pub fn zookeeper_dependency() -> MavenDependency {
    MavenDependency {
        group_id: ZOOKEEPER_GROUP_ID,
        artifact_id: ZOOKEEPER_ARTIFACT_ID,
        version: ZOOKEEPER_VERSION,
    }
}

pub fn indexing_fjall_minicluster_test_dependency() -> MavenScopedDependency {
    MavenScopedDependency {
        group_id: "org.apache.fjall",
        artifact_id: "fjall-minicluster",
        version: FJALL_VERSION,
        scope: "test",
    }
}

pub fn indexing_example_fjall_ext_dependency_set() -> AssemblyDependencySet {
    AssemblyDependencySet {
        output_directory: "fjall/lib/ext",
        includes: INDEXING_EXAMPLE_FJALL_EXT_INCLUDES,
    }
}

pub fn fjall_rya_failsafe_plugin() -> MavenPluginExecution {
    MavenPluginExecution {
        group_id: "org.apache.maven.plugins",
        artifact_id: "maven-failsafe-plugin",
        version: "${maven-failsafe-plugin.version}",
        goals: FJALL_RYA_FAILSAFE_GOALS,
    }
}

pub fn fjall_rya_rat_configuration() -> MavenRatConfiguration {
    MavenRatConfiguration {
        excludes: FJALL_RYA_RAT_EXCLUDES,
    }
}

pub fn fjall_rya_mr_shade_profile() -> MavenShadeProfile {
    MavenShadeProfile {
        id: "mr",
        transformer: "org.apache.maven.plugins.shade.resource.ServicesResourceTransformer",
    }
}

pub fn animal_sniffer_managed_plugin() -> ManagedMavenPlugin {
    ManagedMavenPlugin {
        group_id: ANIMAL_SNIFFER_GROUP_ID,
        artifact_id: ANIMAL_SNIFFER_ARTIFACT_ID,
        version: ANIMAL_SNIFFER_VERSION,
        signature: Some(MavenDependency {
            group_id: ANIMAL_SNIFFER_SIGNATURE_GROUP_ID,
            artifact_id: ANIMAL_SNIFFER_SIGNATURE_ARTIFACT_ID,
            version: ANIMAL_SNIFFER_SIGNATURE_VERSION,
        }),
        execution_phase: Some("test"),
        goals: ANIMAL_SNIFFER_GOALS,
    }
}

pub fn animal_sniffer_enabled_plugin() -> EnabledMavenPlugin {
    EnabledMavenPlugin {
        group_id: ANIMAL_SNIFFER_GROUP_ID,
        artifact_id: ANIMAL_SNIFFER_ARTIFACT_ID,
    }
}

pub fn is_legacy_geomesa_runtime_artifact(artifact_id: &str) -> bool {
    artifact_id == LEGACY_GEOMESA_RUNTIME_ARTIFACT_ID
}

pub fn is_snapshot_version(version: &str) -> bool {
    version.ends_with("-SNAPSHOT")
}

pub fn is_supported_maven_version(version: &str) -> bool {
    match (parse_version(version), parse_version(MAVEN_MIN_VERSION)) {
        (Some(version), Some(minimum)) => version >= minimum,
        _ => false,
    }
}

fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let base = version.split_once('-').map_or(version, |(base, _)| base);
    let mut parts = base.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;

    if parts.next().is_some() {
        return None;
    }

    Some((major, minor, patch))
}

#[cfg(test)]
#[path = "tests/build_info_tests.rs"]
mod tests;
