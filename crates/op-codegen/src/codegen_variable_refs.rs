//! `$variable` references in the markup targets. The node tree names
//! design variables as `$token` strings, which no target language
//! understands, so a generator emits from a prepared copy:
//!
//! - Targets without CSS custom properties (React Native, Flutter,
//!   SwiftUI, Compose) get every reference resolved against the
//!   document's default theme — the same resolver the canvas paints
//!   through, so the export matches what the editor shows.
//! - CSS targets (HTML, Vue, Svelte, inline-style React) get colour
//!   references as `var(--token, default)`, so the `CssVariables` blocks
//!   can switch themes, while the fallback keeps the default-theme
//!   colour wherever none applies: a themed-only variable is declared
//!   only under its `[data-…]` / `.dark` selectors, and the React export
//!   ships no stylesheet at all.

use std::borrow::Cow;
use std::collections::BTreeMap;

use jian_ops_schema::variable::{VariableKind, VariableScalar, VariableValue};
use jian_ops_schema::PenDocument;
use op_editor_core::variables_resolve::{
    document_has_tokens, effective_theme, resolve_document_for_canvas, resolve_variable_ref,
};

use crate::css_custom_property;

/// The document with every `$token` replaced by its default-theme value.
pub(crate) fn resolved_doc(doc: &PenDocument) -> Cow<'_, PenDocument> {
    if !document_has_tokens(doc) {
        return Cow::Borrowed(doc);
    }
    Cow::Owned(resolve_document_for_canvas(doc, &BTreeMap::new()))
}

/// The document for a CSS target's markup: a reference to a colour
/// variable the document defines becomes `var(--token, default)`, any
/// other reference its default-theme value.
pub(crate) fn css_markup_doc(doc: &PenDocument) -> Cow<'_, PenDocument> {
    if !document_has_tokens(doc) {
        return Cow::Borrowed(doc);
    }
    let theme = effective_theme(doc, &BTreeMap::new());
    let mut prepared = doc.clone();
    // Rewriting the definitions rather than the nodes lets the canvas
    // resolver carry the `var()` into every colour slot it knows (fills,
    // strokes, gradient stops, shadows) and resolve everything else.
    for (name, def) in prepared.variables.iter_mut().flatten() {
        if !matches!(def.kind, VariableKind::Color) {
            continue;
        }
        let property = css_custom_property(name);
        let reference = format!("${name}");
        let value = match resolve_variable_ref(&reference, doc.variables.as_ref(), &theme) {
            Some(VariableScalar::Str(fallback)) => format!("var({property}, {fallback})"),
            _ => format!("var({property})"),
        };
        def.value = VariableValue::Scalar(VariableScalar::Str(value));
    }
    Cow::Owned(resolve_document_for_canvas(&prepared, &BTreeMap::new()))
}
