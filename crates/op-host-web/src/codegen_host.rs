//! Code-panel requests owned by the embedding conversation, without a second AI provider.

use std::cell::RefCell;
use std::rc::Rc;

use op_editor_core::codegen::{CodegenPhase, Framework};
use op_editor_core::{EditorState, PenNodeExt};

use crate::repaint_ctx::RepaintContext;

#[derive(Clone, Debug)]
struct Pending {
    id: String,
    identity: (u64, u64, u64, u64),
    framework: Framework,
    node_ids: Vec<String>,
}

thread_local! {
    static NEXT_ID: RefCell<u64> = const { RefCell::new(0) };
    static PENDING: RefCell<Option<Pending>> = const { RefCell::new(None) };
}

fn identity(epoch: u64, state: &EditorState) -> (u64, u64, u64, u64) {
    (
        epoch,
        state.document_generation(),
        state.document_revision(),
        state.codegen.document_reset_epoch(),
    )
}

fn matches(pending: &Pending, id: &str, epoch: u64, state: &EditorState) -> bool {
    pending.id == id
        && pending.identity == identity(epoch, state)
        && pending.framework == state.codegen.framework
        && state.editor_ui.host_owns_chat
}

fn discard() -> Option<Pending> {
    PENDING.with(|pending| pending.borrow_mut().take())
}

pub(crate) fn cancel() {
    if let Some(pending) = discard() {
        crate::vscode_bridge::post_host_event(
            &serde_json::json!({
                "type": "op-bridge/codegen-cancel", "requestId": pending.id
            })
            .to_string(),
        );
    }
}

pub(crate) fn launch<C: RepaintContext + 'static>(inner: &Rc<RefCell<C>>) {
    cancel();
    let mut context = inner.borrow_mut();
    let state = context.host().editor_state();
    let node_ids = if state.selection.is_empty() {
        state
            .active_children()
            .iter()
            .map(|node| node.id_str().to_string())
            .collect::<Vec<_>>()
    } else {
        state
            .selection
            .set
            .iter()
            .map(|id| id.as_str().to_string())
            .collect()
    };
    if node_ids.is_empty() {
        let cg = &mut context.host_mut().editor_state_mut().codegen;
        cg.error = Some("Select nodes to generate code".into());
        cg.phase = CodegenPhase::Error;
        context.host_mut().mark_editor_state_dirty();
        let _ = context.repaint();
        return;
    }
    let id = NEXT_ID.with(|next| {
        let mut next = next.borrow_mut();
        *next += 1;
        format!("host-{}-{next}", js_sys::Date::now() as u64)
    });
    let page_id = state
        .doc
        .pages
        .as_ref()
        .and_then(|pages| pages.get(state.ui.active_page_index))
        .map_or("0", |page| page.id.as_str());
    let event = serde_json::json!({ "type": "op-bridge/codegen-request", "requestId": id,
        "framework": state.codegen.framework.as_wire(), "pageId": page_id, "nodeIds": node_ids })
    .to_string();
    PENDING.with(|pending| {
        *pending.borrow_mut() = Some(Pending {
            id: id.clone(),
            identity: identity(context.host().document_epoch(), state),
            framework: state.codegen.framework,
            node_ids,
        })
    });
    context.host_mut().editor_state_mut().codegen.host_notice = None;
    drop(context);
    crate::vscode_bridge::post_host_event(&event);
    // A missing/old host cannot leave the panel spinning indefinitely.
    let inner = inner.clone();
    let callback = wasm_bindgen::closure::Closure::once_into_js(move || {
        if inner.borrow().host().editor_state().codegen.phase == CodegenPhase::Generating {
            receive(
                &inner,
                &id,
                None,
                None,
                Some("Canvas host did not answer. Reopen the canvas and try again.".into()),
            );
        }
    });
    if let Some(window) = web_sys::window() {
        use wasm_bindgen::JsCast;
        let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
            callback.unchecked_ref(),
            25_000,
        );
    }
}

pub(crate) fn receive<C: RepaintContext + 'static>(
    inner: &Rc<RefCell<C>>,
    id: &str,
    code: Option<String>,
    notice: Option<String>,
    error: Option<String>,
) {
    let Some(pending) = PENDING.with(|pending| pending.borrow().clone()) else {
        return;
    };
    let mut context = inner.borrow_mut();
    let state = context.host().editor_state();
    if pending.id != id {
        return;
    }
    if !matches(&pending, id, context.host().document_epoch(), state) {
        // A changed design cannot consume a stale result or stay in Generating.
        if pending.framework == state.codegen.framework
            && state.codegen.phase == CodegenPhase::Generating
        {
            let cg = &mut context.host_mut().editor_state_mut().codegen;
            cg.error =
                Some("The design changed. Generate code again for the updated design.".into());
            cg.phase = CodegenPhase::Error;
            context.host_mut().mark_editor_state_dirty();
            let _ = context.repaint();
        }
        cancel();
        return;
    }
    let result_identity = (context.host().document_epoch(), state.document_generation());
    let cg = &mut context.host_mut().editor_state_mut().codegen;
    if let Some(error) = error {
        cg.error = Some(error);
        cg.phase = CodegenPhase::Error;
    } else {
        if let Some(code) = code {
            if code.trim().is_empty() || code.len() > 2 * 1024 * 1024 {
                cg.error = Some("The canvas host returned invalid source code.".into());
                cg.phase = CodegenPhase::Error;
                context.host_mut().mark_editor_state_dirty();
                let _ = context.repaint();
                return;
            }
            crate::codegen_web::cache_completed_result(
                result_identity,
                pending.framework,
                code.clone(),
                vec![],
            );
            cg.code = code;
            cg.code_scroll.offset = 0.0;
            cg.code_selection = None;
            cg.selection_snapshot = pending.node_ids;
            cg.assets.clear();
            cg.degraded = false;
            cg.error = None;
            cg.phase = CodegenPhase::Complete;
        }
        if let Some(notice) = notice {
            cg.host_notice = Some(notice);
        }
    }
    context.host_mut().mark_editor_state_dirty();
    let _ = context.repaint();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_discards_the_host_request() {
        let state = EditorState::default();
        PENDING.with(|slot| {
            *slot.borrow_mut() = Some(Pending {
                id: "host-1".into(),
                identity: identity(1, &state),
                framework: state.codegen.framework,
                node_ids: vec![],
            })
        });
        discard();
        assert!(PENDING.with(|slot| slot.borrow().is_none()));
    }

    #[test]
    fn response_is_bound_to_host_request_document_and_framework() {
        let mut state = EditorState::default();
        state.editor_ui.host_owns_chat = true;
        let pending = Pending {
            id: "host-1".into(),
            identity: identity(7, &state),
            framework: state.codegen.framework,
            node_ids: vec!["n1".into()],
        };
        assert!(matches(&pending, "host-1", 7, &state));
        assert!(!matches(&pending, "host-2", 7, &state));
        assert!(!matches(&pending, "host-1", 8, &state));
        state.codegen.reset_for_document_replacement();
        assert!(!matches(&pending, "host-1", 7, &state));
        let pending = Pending {
            identity: identity(7, &state),
            ..pending
        };
        state.mark_document_changed();
        assert!(!matches(&pending, "host-1", 7, &state));
        let pending = Pending {
            identity: identity(7, &state),
            ..pending
        };
        state.codegen.framework = Framework::Vue;
        assert!(!matches(&pending, "host-1", 7, &state));
        state.codegen.framework = pending.framework;
        state.editor_ui.host_owns_chat = false;
        assert!(!matches(&pending, "host-1", 7, &state));
    }
}
