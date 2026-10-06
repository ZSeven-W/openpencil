//! Work identity comes from declared intent. Geometry only picks reading posture.

use crate::scene_template_catalog::TemplateScene;
use crate::{EditorState, EditorUiState, HomeFamily, PenNodeExt};

pub fn declared_family(ui: &EditorUiState) -> Option<HomeFamily> {
    ui.home
        .work_family
        .or_else(|| ui.home.recipe.as_ref().map(|recipe| recipe.family))
        .or_else(|| {
            ui.scenario.map(|scene| match scene {
                TemplateScene::App => HomeFamily::AppUi,
                TemplateScene::Web => HomeFamily::Web,
                TemplateScene::Slides => HomeFamily::Presentation,
                TemplateScene::Tutorial => HomeFamily::ScreenshotTutorial,
                TemplateScene::Infographic => HomeFamily::Infographic,
                TemplateScene::Card | TemplateScene::Carousel | TemplateScene::Comparison => {
                    HomeFamily::KnowledgeCards
                }
            })
        })
}

/// Reopen the same document on desktop and touch, without inventing a task.
pub fn open_for_reading(state: &mut EditorState, now_ms: u64) {
    let declared = declared_family(&state.editor_ui);
    let family = declared.unwrap_or_else(|| {
        let sizes: Vec<_> = state
            .active_children()
            .iter()
            .filter(|node| matches!(node, jian_ops_schema::node::PenNode::Frame(_)))
            .filter_map(|node| Some((node.width_px()?, node.height_px()?)))
            .collect();
        crate::infer_reading_family(&sizes)
    });
    state.editor_ui.workspace.open_for_reading(family, now_ms);
    state.editor_ui.workspace.family_known = declared.is_some();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tall_work() -> EditorState {
        let doc = jian_ops_schema::load_str(r#"{"version":"1.0.0","children":[{"type":"frame","id":"poster","width":1080,"height":2160,"children":[]}]}"#).unwrap().value;
        EditorState::from_document(doc)
    }

    #[test]
    fn declared_task_and_template_override_dimension_guesses() {
        let mut state = tall_work();
        state.editor_ui.home.work_family = Some(HomeFamily::EventPoster);
        open_for_reading(&mut state, 1);
        assert_eq!(state.editor_ui.workspace.family, HomeFamily::EventPoster);
        assert!(state.editor_ui.workspace.family_known);
        state.editor_ui.home.work_family = None;
        state.editor_ui.scenario = Some(TemplateScene::Infographic);
        open_for_reading(&mut state, 2);
        assert_eq!(state.editor_ui.workspace.family, HomeFamily::Infographic);
        assert!(state.editor_ui.workspace.family_known);
    }

    #[test]
    fn legacy_geometry_selects_posture_without_claiming_a_design_type() {
        let mut state = tall_work();
        let before = state.doc.clone();
        open_for_reading(&mut state, 1);
        assert_eq!(
            state.editor_ui.workspace.view,
            crate::WorkspaceView::LongPage
        );
        assert!(!state.editor_ui.workspace.family_known);
        assert!(state.editor_ui.home.work_family.is_none());
        assert_eq!(state.doc, before);
    }
}
