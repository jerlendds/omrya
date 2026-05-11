#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManualBuildConfig {
    pub packaging: &'static str,
    pub site_plugin: &'static str,
    pub markdown_module: &'static str,
    pub input_encoding: &'static str,
    pub output_encoding: &'static str,
    pub retired_web_stack: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManualPage {
    pub title: &'static str,
    pub markdown_file: &'static str,
    pub html_file: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ManualSection {
    pub name: &'static str,
    pub pages: &'static [ManualPage],
}

pub const MANUAL_MARKDOWN_ROOT: &str = "rya/extras/rya.manual/src/site/markdown";
pub const MANUAL_INDEX_PATH: &str = "rya/extras/rya.manual/src/site/markdown/index.md";
pub const MANUAL_LINK_REWRITE_SCRIPT: &str =
    "rya/extras/rya.manual/src/site/resources/js/fixmarkdownlinks.js";
pub const APACHE_RYA_WEBSITE_URL: &str = "http://rya.incubator.apache.org/";
pub const MAPREDUCE_MANUAL_TITLE: &str = "MapReduce Interface";
pub const MAPREDUCE_MANUAL_MARKDOWN: &str = "mapreduce.md";
pub const MAPREDUCE_MANUAL_HTML: &str = "mapreduce.html";
pub const MAPREDUCE_LOAD_JAR: &str = "target/rya.mapreduce-3.2.10-SNAPSHOT-shaded.jar";
pub const MAPREDUCE_LOAD_TOOL_CLASS: &str = "mvm.rya.fjall.mr.RdfFileInputTool";

const RYA_PAGES: &[ManualPage] = &[
    page("Overview", "overview.md", "overview.html"),
    page("Quick Start", "quickstart.md", "quickstart.html"),
    page("Load Data", "loaddata.md", "loaddata.html"),
    page("Query Data", "querydata.md", "querydata.html"),
    page("Evaluation Table", "eval.md", "eval.html"),
    page(
        "Pre-computed Joins",
        "loadPrecomputedJoin.md",
        "loadPrecomputedJoin.html",
    ),
    page("Inferencing", "infer.md", "infer.html"),
    page(
        MAPREDUCE_MANUAL_TITLE,
        MAPREDUCE_MANUAL_MARKDOWN,
        MAPREDUCE_MANUAL_HTML,
    ),
];

const SAMPLE_PAGES: &[ManualPage] = &[
    page(
        "Typical First Steps",
        "sm-firststeps.md",
        "sm-firststeps.html",
    ),
    page(
        "Simple Add/Query/Remove Statements",
        "sm-simpleaqr.md",
        "sm-simpleaqr.html",
    ),
    page("Sparql query", "sm-sparqlquery.md", "sm-sparqlquery.html"),
    page("Adding Authentication", "sm-addauth.md", "sm-addauth.html"),
    page("Inferencing", "sm-infer.md", "sm-infer.html"),
    page("Named Graph", "sm-namedgraph.md", "sm-namedgraph.html"),
    page("Update data", "sm-updatedata.md", "sm-updatedata.html"),
    page("Alx", "alx.md", "alx.html"),
];

const DEVELOPMENT_PAGES: &[ManualPage] = &[page(
    "Building From Source",
    "build-source.md",
    "build-source.html",
)];

pub const MANUAL_SECTIONS: &[ManualSection] = &[
    ManualSection {
        name: "Rya",
        pages: RYA_PAGES,
    },
    ManualSection {
        name: "Samples",
        pages: SAMPLE_PAGES,
    },
    ManualSection {
        name: "Development",
        pages: DEVELOPMENT_PAGES,
    },
];

const fn page(
    title: &'static str,
    markdown_file: &'static str,
    html_file: &'static str,
) -> ManualPage {
    ManualPage {
        title,
        markdown_file,
        html_file,
    }
}

pub fn manual_build_config() -> ManualBuildConfig {
    ManualBuildConfig {
        packaging: "jar",
        site_plugin: "maven-site-plugin",
        markdown_module: "doxia-module-markdown",
        input_encoding: "UTF-8",
        output_encoding: "UTF-8",
        retired_web_stack: &[
            "war",
            "maven-scalate-plugin_2.11",
            "TemplateEngineFilter",
            "jetty-maven-plugin",
            "tomcat-maven-plugin",
            "scalate-wikitext_2.11",
        ],
    }
}

pub fn manual_pages() -> impl Iterator<Item = &'static ManualPage> {
    MANUAL_SECTIONS
        .iter()
        .flat_map(|section| section.pages.iter())
}

pub fn find_page_by_markdown(markdown_file: &str) -> Option<&'static ManualPage> {
    manual_pages().find(|page| page.markdown_file == markdown_file)
}

pub fn rewrite_markdown_href(href: &str) -> String {
    href.strip_suffix(".md")
        .map_or_else(|| href.to_string(), |prefix| format!("{prefix}.html"))
}

#[cfg(test)]
#[path = "tests/manual_tests.rs"]
mod tests;
