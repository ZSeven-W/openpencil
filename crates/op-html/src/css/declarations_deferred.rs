use super::{expand_declaration, Declaration};
use crate::css::cascade_shared::is_ident;
use std::collections::BTreeSet;

pub(super) fn is_shorthand_with_var(name: &str, value: &str) -> bool {
    is_supported_shorthand(name) && contains_var_function(value)
}

pub(super) fn push_deferred(
    out: &mut Vec<Declaration>,
    shorthand: &str,
    value: &str,
    important: bool,
) {
    let mut placeholders = Vec::new();
    expand_declaration(&mut placeholders, shorthand, "initial", important);
    let mut seen = BTreeSet::new();
    for mut declaration in placeholders {
        if !seen.insert(declaration.name.clone()) {
            continue;
        }
        declaration.value = value.trim().to_string();
        declaration.deferred_shorthand = Some(shorthand.to_string());
        out.push(declaration);
    }
}

pub(crate) fn resolve_deferred_shorthand(
    shorthand: &str,
    target: &str,
    value: &str,
) -> Option<String> {
    let mut declarations = Vec::new();
    expand_declaration(&mut declarations, shorthand, value, false);
    declarations
        .into_iter()
        .rev()
        .find(|declaration| declaration.name == target)
        .map(|declaration| declaration.value)
}

fn is_supported_shorthand(name: &str) -> bool {
    matches!(
        name,
        "margin"
            | "padding"
            | "inset"
            | "margin-block"
            | "padding-block"
            | "inset-block"
            | "margin-inline"
            | "padding-inline"
            | "inset-inline"
            | "border"
            | "outline"
            | "border-top"
            | "border-right"
            | "border-bottom"
            | "border-left"
            | "border-block"
            | "border-inline"
            | "border-width"
            | "border-style"
            | "border-color"
            | "border-block-width"
            | "border-block-style"
            | "border-block-color"
            | "border-inline-width"
            | "border-inline-style"
            | "border-inline-color"
            | "border-radius"
            | "background"
            | "font"
            | "flex"
            | "flex-flow"
            | "gap"
            | "overflow"
            | "place-items"
            | "place-content"
            | "place-self"
            | "text-decoration"
            | "columns"
    )
}

pub(super) fn contains_var_function(value: &str) -> bool {
    let bytes = value.as_bytes();
    let (mut quote, mut escaped) = (None, false);
    let mut at = 0;
    while at + 4 <= bytes.len() {
        let byte = bytes[at];
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == active {
                quote = None;
            }
        } else if matches!(byte, b'\'' | b'"') {
            quote = Some(byte);
        } else if bytes[at..at + 4].eq_ignore_ascii_case(b"var(")
            && value[..at]
                .chars()
                .next_back()
                .is_none_or(|ch| !is_ident(ch) && ch != '\\')
        {
            return true;
        }
        at += 1;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_function_tokens_but_not_strings_or_identifiers() {
        assert!(contains_var_function("1px var(--edge)"));
        assert!(!contains_var_function(r#"url("var(--edge)")"#));
        assert!(!contains_var_function("notvar(--edge)"));
    }
}
