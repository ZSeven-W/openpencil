use std::io::{self, Write};
use std::sync::Arc;

use op_ai::chat_provider::{ChatAttachment, ChatProvider};
use op_orchestrator::{DesignRequest, DocSink};

use super::events::write_thinking_event;

pub(crate) fn resolve_image_reference(
    attachments: &[ChatAttachment],
    provider_arc: &Arc<dyn ChatProvider>,
    prompt: &str,
    model: Option<String>,
    provider: Option<String>,
) -> Result<
    Option<crate::reference_context::ReferenceContext>,
    crate::reference_context::ReferenceContextError,
> {
    if !op_orchestrator::has_reference_trigger(prompt) {
        return Ok(None);
    }
    let Some(image) = crate::reference_context::first_image_attachment(attachments) else {
        return Ok(None);
    };
    let vision = crate::validation_providers::ChatVisionLlmClient::new(provider_arc.clone())
        .with_model(model.clone());
    crate::reference_context::reference_context_from_image(&vision, image, prompt, model, provider)
        .map(Some)
}

pub(crate) fn apply_reference_context(
    request: &mut DesignRequest,
    sink: &mut dyn DocSink,
    context: crate::reference_context::ReferenceContext,
) {
    request.reference_skeleton = context.skeleton;
    request.design_md = Some(context.design_md.clone());
    let _ = sink.apply(op_editor_core::EditorCommand::SetDesignMd {
        spec: Box::new(context.design_md),
    });
}

pub(crate) fn apply_image_reference<W: Write>(
    request: &mut DesignRequest,
    sink: &mut dyn DocSink,
    out: &mut W,
    result: Result<
        Option<crate::reference_context::ReferenceContext>,
        crate::reference_context::ReferenceContextError,
    >,
) -> io::Result<()> {
    match result {
        Ok(Some(context)) => apply_reference_context(request, sink, context),
        Ok(None) => {}
        Err(error) => {
            let notice = format!("reference screenshot could not be used: {error}");
            eprintln!("[web-chat] {notice}");
            write_thinking_event(out, &format!("\n{notice}"))?;
        }
    }
    Ok(())
}
