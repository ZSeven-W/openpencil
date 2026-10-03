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
    let family = match family {
        "app" => HomeFamily::AppUi,
        "web" => HomeFamily::Web,
        "presentation" => HomeFamily::Presentation,
        "cards" => HomeFamily::KnowledgeCards,
        "tutorial" => HomeFamily::ScreenshotTutorial,
        "infographic" => HomeFamily::Infographic,
        "poster" => HomeFamily::EventPoster,
        value => return Err(HomePromptError::UnknownFamily(value.into())),
    };
    let mut state = EditorState::new();
    state.editor_ui.home.task = family;
    state.editor_ui.home.set_draft(prompt);
    state
        .editor_ui
        .home
        .generation_prompt()
        .ok_or(HomePromptError::EmptyBrief)
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
