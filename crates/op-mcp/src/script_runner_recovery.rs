//! Syntax recovery must never turn an invalid insertion parent into a
//! successful partial section. Bubble that failure back to the model retry.

use super::{
    balance_brackets, dotted_keys, escape_raw_newlines_in_quoted_strings, eval_to_program,
    repair_truncated_script, truncate_duplicate_script, ScriptError,
};

fn invalid_parent(error: &ScriptError) -> bool {
    error.is_invalid_insertion_parent()
}

fn attempt(source: &str, preamble: Option<&str>) -> Result<Option<String>, ScriptError> {
    match eval_to_program(source, preamble) {
        Ok(program) => Ok(Some(program)),
        Err(error) if invalid_parent(&error) => Err(error),
        Err(_) => Ok(None),
    }
}

pub(super) fn eval_after_initial_failure(
    script: &str,
    first_err: ScriptError,
    preamble: Option<&str>,
) -> Result<String, ScriptError> {
    if invalid_parent(&first_err) {
        return Err(first_err);
    }
    // gemini-3.6-flash writes schema property names with the separator it
    // reads in the docs — `justify.content:` instead of `justifyContent:`.
    // A bare dotted key is a SyntaxError at the first `.`, so QuickJS
    // rejected an otherwise-correct slide before recording one `I(...)`.
    // Normalize FIRST, then let the rest of the ladder work on the repaired
    // source: a script can be both mis-keyed and truncated.
    let script = match dotted_keys::repair_dotted_object_keys(script) {
        Some(repaired) => match attempt(&repaired, preamble)? {
            Some(p) => {
                tracing::warn!(
                    original_len = script.len(),
                    repaired_len = repaired.len(),
                    "script failed as-is; dotted-property-key repair recovered a runnable source"
                );
                return Ok(p);
            }
            None => repaired,
        },
        None => script.to_string(),
    };
    let script = match escape_raw_newlines_in_quoted_strings(&script) {
        Some(repaired) => match attempt(&repaired, preamble)? {
            Some(p) => {
                tracing::warn!(
                    original_len = script.len(),
                    repaired_len = repaired.len(),
                    "script failed as-is; raw newline repair recovered a quoted string"
                );
                return Ok(p);
            }
            None => repaired,
        },
        None => script,
    };
    let script = script.as_str();

    // GLM-5.2 commonly drops the outer `}` when `stroke:{...}` is the final
    // property of an I() object, so QuickJS reaches `)` with `{` still open.
    let balanced = balance_brackets(script);
    if balanced != script {
        if let Some(p) = attempt(&balanced, preamble)? {
            tracing::warn!(
                original_len = script.len(),
                repaired_len = balanced.len(),
                "script failed as-is; bracket balance repair recovered a runnable source"
            );
            return Ok(p);
        }
    }

    // DeepSeek V4 sometimes ECHOES the whole script a second time, glued
    // straight onto the first copy (often mid-line after a trailing
    // comment) - the re-declared `const` bindings then throw "invalid
    // redefinition of lexical identifier" and the whole section is lost.
    // Detect the first declaration recurring and run the first copy alone.
    if let Some(deduped) = truncate_duplicate_script(script) {
        if let Some(p) = attempt(&deduped, preamble)? {
            tracing::warn!(
                original_len = script.len(),
                deduped_len = deduped.len(),
                "script failed as-is; duplicate-echo truncation recovered the first copy"
            );
            return Ok(p);
        }
    }

    match repair_truncated_script(script) {
        Some(repaired) => match attempt(&repaired, preamble)? {
            Some(p) => {
                tracing::warn!(
                    original_len = script.len(),
                    repaired_len = repaired.len(),
                    "script failed as-is; truncation repair salvaged a runnable prefix"
                );
                Ok(p)
            }
            None => Err(first_err),
        },
        None => Err(first_err),
    }
}
