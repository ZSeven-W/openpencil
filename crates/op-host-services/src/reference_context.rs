//! Reference-page structure and style context for new design turns.

use jian_ops_schema::node::PenNode;
use op_ai::chat_provider::ChatAttachment;
use op_editor_core::EditorState;
use op_orchestrator::{
    detect_reference_intent, AbortFlag, LlmClient, ReferenceIntent, VisionCallRequest,
    VisionLlmClient, VisionResponse,
};

use crate::import_html_url::import_page_from_url;

pub use crate::reference_context_error::ReferenceContextError;

/// The safe subset of a referenced page that reaches the planner.
#[derive(Debug, Clone)]
pub struct ReferenceContext {
    pub source_host: String,
    pub skeleton: Option<op_orchestrator::ReferenceSkeleton>,
    pub design_md: jian_ops_schema::DesignMdSpec,
}

/// Build reference context from already-imported nodes.
///
/// The temporary state is intentionally separate from every live editor state:
/// the imported page can only be read by the design.md extractor and the
/// content-free skeleton builder.
pub async fn reference_context_from_nodes(
    llm: &dyn LlmClient,
    nodes: Vec<PenNode>,
    source: &str,
    user_prompt: &str,
    model: Option<String>,
    provider: Option<String>,
    abort: &AbortFlag,
) -> Result<ReferenceContext, ReferenceContextError> {
    let mut temp = EditorState::new();
    *temp.active_children_mut() = nodes;
    let skeleton = op_orchestrator::ReferenceSkeleton::from_state(&temp, source)
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
        skeleton: Some(skeleton),
        design_md,
    })
}

const SCREENSHOT_SKELETON_INSTRUCTION: &str = r#"After the design.md, output a line `<<<SKELETON>>>` followed by ONE JSON object of exactly this shape (camelCase, matches `ReferenceSkeleton` serde): `{"source":"screenshot","width":1440,"sections":[{"role":"navbar","heightRatio":0.05,"childCount":3,"layout":"horizontal","hasImage":false}],"navKind":"topBar","heroKind":"split","columnRhythm":[3]}`. Use only these section roles: navbar, hero, section, features, pricing, testimonial, cta, footer, bottom-tab-bar. Never include any text, brand, or copy from the screenshot."#;

/// Extract a screenshot's reusable design system and content-free structure.
pub fn reference_context_from_image(
    vision: &dyn VisionLlmClient,
    image: &ChatAttachment,
    user_prompt: &str,
    model: Option<String>,
    provider: Option<String>,
) -> Result<ReferenceContext, ReferenceContextError> {
    if !image.media_type.starts_with("image/") || image.data.is_empty() {
        return Err(ReferenceContextError::NotAnImage);
    }

    use base64::Engine as _;
    let response = vision.validate(VisionCallRequest {
        system: format!(
            "{}\n\n{}",
            crate::design_md_llm::design_md_system_prompt(),
            SCREENSHOT_SKELETON_INSTRUCTION
        ),
        message: format!(
            "Extract the design system and the section skeleton of this screenshot for a NEW product: {user_prompt}"
        ),
        image_base64: base64::engine::general_purpose::STANDARD.encode(&image.data),
        model,
        provider,
        timeout: crate::design_md_llm::DESIGN_MD_TIMEOUT,
    });
    let output = match response {
        VisionResponse::Text(output) => output,
        VisionResponse::Skipped { reason } => {
            return Err(ReferenceContextError::DesignMd(
                crate::design_md_llm::DesignMdError::Llm(
                    reason.unwrap_or_else(|| "vision reference extraction was skipped".into()),
                ),
            ));
        }
    };

    let (design_md_text, skeleton_text) = output
        .split_once("<<<SKELETON>>>")
        .map_or((output.as_str(), None), |(design_md, skeleton)| {
            (design_md, Some(skeleton))
        });
    let design_md = crate::design_md_llm::parse_design_md_text(design_md_text)?;
    let skeleton = skeleton_text.and_then(|text| {
        match serde_json::from_str::<op_orchestrator::ReferenceSkeleton>(strip_code_fence(text)) {
            Ok(skeleton) => Some(skeleton),
            Err(error) => {
                tracing::debug!(%error, "screenshot reference skeleton was not valid JSON");
                None
            }
        }
    });

    Ok(ReferenceContext {
        source_host: "screenshot".to_string(),
        skeleton,
        design_md,
    })
}

/// Return the first image attachment, preserving request order.
/// Models routinely wrap the skeleton JSON in a ```json fence; strip one so
/// the fence alone never costs the skeleton.
fn strip_code_fence(text: &str) -> &str {
    let text = text.trim();
    let text = text
        .strip_prefix("```json")
        .or_else(|| text.strip_prefix("```"))
        .unwrap_or(text);
    text.strip_suffix("```").unwrap_or(text).trim()
}

pub fn first_image_attachment(attachments: &[ChatAttachment]) -> Option<&ChatAttachment> {
    attachments
        .iter()
        .find(|attachment| attachment.media_type.starts_with("image/"))
}

/// Resolve a prompt's optional URL reference, import it through the existing
/// SSRF-screened importer, and derive isolated planning context.
pub async fn resolve_reference_context(
    llm: &dyn LlmClient,
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
