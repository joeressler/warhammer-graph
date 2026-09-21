//! Build a petgraph bundle from a Wahapedia corpus v1 directory.

mod attrs;
mod build;
mod bundle;
mod corpus;
mod error;
mod html;
mod model;
mod textutil;

pub use attrs::Attrs;
pub use bundle::{build_bundle, validate_bundle, Summary};
pub use error::GraphError;
pub use model::{BundleManifest, EdgeRecord, GraphEdge, GraphNode, Passage};

const BUILD_EXAMPLE: &str = "wh-graph build --corpus ./corpus --out ./bundle";
const VALIDATE_EXAMPLE: &str = "wh-graph validate --bundle ./bundle";

pub fn render_summary(
    summary: &Summary,
    output: &str,
    example: &str,
) -> Result<String, GraphError> {
    match output {
        "text" => Ok(format!(
            "bundle: {}\nformat_version: {}\nnodes: {}\nedges: {}\npassages: {}\ncorpus_fingerprint: {}\n",
            summary.bundle,
            summary.format_version,
            summary.nodes,
            summary.edges,
            summary.passages,
            summary.corpus_fingerprint
        )),
        "json" => {
            #[derive(serde::Serialize)]
            struct SummaryJson<'a> {
                bundle: &'a str,
                format_version: u32,
                nodes: usize,
                edges: usize,
                passages: usize,
                corpus_fingerprint: &'a str,
            }
            let payload = SummaryJson {
                bundle: &summary.bundle,
                format_version: summary.format_version,
                nodes: summary.nodes,
                edges: summary.edges,
                passages: summary.passages,
                corpus_fingerprint: &summary.corpus_fingerprint,
            };
            let mut text = serde_json::to_string(&payload).map_err(|err| {
                GraphError::Read(format!("could not render JSON output: {err}"))
            })?;
            text.push('\n');
            Ok(text)
        }
        _ => Err(GraphError::Usage(format!("bad flag: --output\n{example}"))),
    }
}

pub fn build_example() -> &'static str {
    BUILD_EXAMPLE
}

pub fn validate_example() -> &'static str {
    VALIDATE_EXAMPLE
}
