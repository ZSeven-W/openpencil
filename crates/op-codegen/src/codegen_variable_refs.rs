//! `$variable` references in the markup targets. The node tree names
//! design variables as `$token` strings, which no target language
//! understands, so a generator emits from a prepared copy. Targets
//! without CSS custom properties (React Native, Flutter, SwiftUI,
//! Compose) get every reference resolved against the document's
//! default theme — the same resolver the canvas paints through, so the
//! export matches what the editor shows.

use std::borrow::Cow;
use std::collections::BTreeMap;

use jian_ops_schema::PenDocument;
use op_editor_core::variables_resolve::{document_has_tokens, resolve_document_for_canvas};

/// The document with every `$token` replaced by its default-theme value.
pub(crate) fn resolved_doc(doc: &PenDocument) -> Cow<'_, PenDocument> {
    if !document_has_tokens(doc) {
        return Cow::Borrowed(doc);
    }
    Cow::Owned(resolve_document_for_canvas(doc, &BTreeMap::new()))
}
