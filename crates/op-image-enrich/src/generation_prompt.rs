use crate::ImageSearchTarget;

/// Subjects at least this heavy already describe a scene. Latin words count
/// one each, CJK characters one per two (a Chinese word is ~2 characters).
const DESCRIBED_SCENE_WEIGHT: usize = 8;

/// Keep authored prompts/queries first; replace only a node-name fallback
/// with the nearest heading. Page context only rescues a bare subject such
/// as a project name: text-capable models paint its headings and node names
/// into the picture whatever the prompt says, so a subject that already
/// describes a scene is sent alone. Without context, preserve the original
/// bytes.
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
    if describes_scene(prompt) {
        return prompt.to_string();
    }
    format!("{prompt}. Context: this image appears in {context}. Match the page's subject and art direction. Use this context only for subject and mood; do not write its words into the image.")
}

fn describes_scene(subject: &str) -> bool {
    let words = subject
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|word| word.chars().any(|c| c.is_ascii_alphabetic()))
        .count();
    let cjk = subject.chars().filter(|c| is_cjk(*c)).count();
    words + cjk.div_ceil(2) >= DESCRIBED_SCENE_WEIGHT
}

fn is_cjk(c: char) -> bool {
    matches!(c, '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}' | '\u{f900}'..='\u{faff}')
}
