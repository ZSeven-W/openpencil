use crate::ImageSearchTarget;

/// Keep authored prompts/queries first; replace only a node-name fallback
/// with the nearest heading. Without context, preserve the original bytes.
pub fn compose_generation_prompt(target: &ImageSearchTarget) -> String {
    let authored_prompt = target
        .prompt
        .as_deref()
        .filter(|prompt| !prompt.trim().is_empty());
    let Some(context) = target
        .page_context
        .as_deref()
        .filter(|context| !context.trim().is_empty())
    else {
        return authored_prompt.unwrap_or(&target.query).to_string();
    };
    let prompt = authored_prompt
        .or(target.generation_subject.as_deref())
        .unwrap_or(&target.query);
    format!("{prompt}. Context: this image appears in {context}. Match the page's subject and art direction.")
}
