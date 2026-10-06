//! Optional Studio Home prompt construction for task-faithful benchmarks.

use op_editor_core::{EditorState, HomeFamily};
use std::fmt;

#[derive(Debug)]
pub(crate) enum HomePromptError {
    UnknownFamily(String),
    EmptyBrief,
}

impl fmt::Display for HomePromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownFamily(value) => write!(f, "unknown smoke Home family {value:?}"),
            Self::EmptyBrief => f.write_str("Home brief is empty"),
        }
    }
}

pub(crate) fn from_env(prompt: String) -> Result<String, HomePromptError> {
    prepare(
        prompt,
        std::env::var("OPENPENCIL_SMOKE_HOME_FAMILY")
            .ok()
            .as_deref(),
    )
}

fn prepare(prompt: String, family: Option<&str>) -> Result<String, HomePromptError> {
    let Some(family) = family else {
        return Ok(prompt);
    };
    let family =
        parse_family(family).ok_or_else(|| HomePromptError::UnknownFamily(family.into()))?;
    let mut state = EditorState::new();
    state.editor_ui.home.task = family;
    state.editor_ui.home.set_draft(prompt);
    state
        .editor_ui
        .home
        .generation_prompt()
        .ok_or(HomePromptError::EmptyBrief)
}

fn parse_family(id: &str) -> Option<HomeFamily> {
    HomeFamily::from_id(id).or_else(|| (id == "cards").then_some(HomeFamily::KnowledgeCards))
}

pub(crate) fn record_purpose(state: &mut EditorState) {
    state.editor_ui.home.work_family = std::env::var("OPENPENCIL_SMOKE_HOME_FAMILY")
        .ok()
        .as_deref()
        .and_then(parse_family);
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_orchestrator::{detect_design_type, DesignType};

    #[test]
    fn home_card_contract_wins_over_app_words_in_the_article() {
        let prompt = prepare(
            "把文章转成4张卡：App界面从一页开始，需要再加多页。".into(),
            Some("cards"),
        )
        .unwrap();
        assert_eq!(detect_design_type(&prompt).type_, DesignType::Card);
        assert!(prompt.contains("App界面从一页开始"));
    }

    #[test]
    fn unknown_family_is_not_silently_treated_as_a_raw_brief() {
        assert!(prepare("brief".into(), Some("not-a-family")).is_err());
        assert_eq!(prepare("raw brief".into(), None).unwrap(), "raw brief");
    }
}
