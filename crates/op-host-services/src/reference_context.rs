//! Reference-page structure and style context for new design turns.

use jian_ops_schema::node::PenNode;
use op_editor_core::EditorState;
use op_orchestrator::{detect_reference_intent, AbortFlag, LlmClient, ReferenceIntent};

use crate::import_html_url::import_page_from_url;

pub use crate::reference_context_error::ReferenceContextError;

/// The safe subset of a referenced page that reaches the planner.
#[derive(Debug, Clone)]
pub struct ReferenceContext {
    pub source_host: String,
    pub skeleton: op_orchestrator::ReferenceSkeleton,
    pub design_md: jian_ops_schema::DesignMdSpec,
}

/// Build reference context from already-imported nodes.
///
/// The temporary state is intentionally separate from every live editor state:
/// the imported page can only be read by the design.md extractor and the
/// content-free skeleton builder.
pub async fn reference_context_from_nodes<L: LlmClient + Send + Sync>(
    llm: &L,
    nodes: Vec<PenNode>,
    source: &str,
    user_prompt: &str,
    model: Option<String>,
    provider: Option<String>,
    abort: &AbortFlag,
) -> Result<ReferenceContext, ReferenceContextError> {
    let mut temp = EditorState::new();
    *temp.active_children_mut() = nodes;
    let root = temp
        .active_children()
        .first()
        .ok_or(ReferenceContextError::NoStructure)?;
    let skeleton = op_orchestrator::ReferenceSkeleton::from_root(root, source)
        .ok_or(ReferenceContextError::NoStructure)?;
    let design_md = crate::design_md_llm::generate_design_md_spec(
        llm,
        &temp,
        user_prompt,
        model,
        provider,
        abort,
    )
    .await?;

    Ok(ReferenceContext {
        source_host: skeleton.source.clone(),
        skeleton,
        design_md,
    })
}

/// Resolve a prompt's optional URL reference, import it through the existing
/// SSRF-screened importer, and derive isolated planning context.
pub async fn resolve_reference_context<L: LlmClient + Send + Sync>(
    llm: &L,
    prompt: &str,
    model: Option<String>,
    provider: Option<String>,
    abort: &AbortFlag,
) -> Result<Option<ReferenceContext>, ReferenceContextError> {
    let Some(ReferenceIntent::Url(url)) = detect_reference_intent(prompt) else {
        return Ok(None);
    };
    let page = import_page_from_url(&url, None)?;
    let source_host = page
        .final_url
        .host_str()
        .ok_or(ReferenceContextError::NoStructure)?
        .to_string();
    reference_context_from_nodes(
        llm,
        page.nodes,
        &source_host,
        prompt,
        model,
        provider,
        abort,
    )
    .await
    .map(Some)
}

#[cfg(test)]
#[path = "reference_context_tests.rs"]
mod tests;
