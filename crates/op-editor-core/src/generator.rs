//! Canvas generator nodes.
//!
//! A generator is an ordinary Frame that also stores a small sandboxed
//! design program plus named parameters. Running the program produces a
//! subtree that is MATERIALIZED as the frame's ordinary `children`, so any
//! `.op` reader (web viewer SDK, codegen, older builds) renders the output
//! without running code — the program is only needed to *change* it.
//!
//! ## Storage (no schema change)
//!
//! The spec rides in the frame's `explain` string as
//! `"op:generator " + JSON` ([`GENERATOR_EXPLAIN_PREFIX`]). `explain` is a
//! free-form string on every node base that renderers and codegen ignore,
//! and it already carries prefixed machine metadata elsewhere (the image
//! enrichment fallback marker, the HTML importer's z-index hint), so the
//! spec round-trips through every reader untouched. A pre-existing human
//! `explain` is preserved inside the spec (`originalExplain`) and restored
//! on detach.
//!
//! ## Ownership
//!
//! Generated children are OWNED by the generator: regenerating replaces
//! the whole child list, so a manual edit to a generated child survives
//! only until the next regenerate. The spec stores a hash of the output it
//! last wrote (`outputHash`); the property panel compares it with the live
//! children and warns when they were edited by hand. To keep hand edits,
//! detach — that drops the program and leaves plain children behind.
//!
//! ## Determinism
//!
//! Child ids are derived from the generator id plus the child's tree path
//! (`{gid}_g{i}`, then `{parent}_{j}` below), never from an allocator, so
//! the same program + params produce byte-identical children. The runtime
//! (installed by the host, see [`install_generator_runner`]) seeds
//! `Math.random` from the generator id and freezes the clock.

use std::sync::OnceLock;

use jian_ops_schema::node::PenNode;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::pen_node_ext::PenNodeExt;

#[path = "generator_materialize.rs"]
mod materialize;
#[path = "generator_remote.rs"]
mod remote;
#[path = "generator_starters.rs"]
pub mod starters;
#[path = "generator_state.rs"]
mod state;

pub use crate::generator_error::GeneratorError;
pub use materialize::{materialize_children, output_hash};
pub use remote::{
    generator_run_reply_body, parse_generator_run_reply, GeneratorPending, GeneratorRunReply,
    GeneratorRunWireRequest, GENERATOR_RUN_ROUTE,
};
pub use starters::{GeneratorStarter, GENERATOR_STARTERS};
pub use state::GeneratorPanelError;

/// Prefix marking a generator spec inside a node's `explain` string.
pub const GENERATOR_EXPLAIN_PREFIX: &str = "op:generator ";
/// Current spec format version.
pub const GENERATOR_SPEC_VERSION: u32 = 1;
/// The only engine this build knows: the sandboxed JavaScript recorder
/// shared with `batch_design { script }`.
pub const GENERATOR_ENGINE_JS: &str = "js";
/// Cap on declared parameters.
pub const MAX_GENERATOR_PARAMS: usize = 16;
/// Cap on the program source.
pub const MAX_PROGRAM_BYTES: usize = 64 * 1024;
/// Cap on the number of nodes one generator may own.
pub const MAX_GENERATED_NODES: usize = 2_000;
/// Cap on the nesting depth of the generated subtree.
pub const MAX_GENERATED_DEPTH: usize = 24;
/// Cap on a text parameter's length.
pub const MAX_TEXT_PARAM_CHARS: usize = 4_096;

/// The value kinds a parameter can hold — each maps to one property-panel
/// editor (number / text / color field, boolean checkbox).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GeneratorParamKind {
    Number,
    Text,
    Color,
    Boolean,
}

/// One named parameter the program reads as `params.<name>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratorParam {
    pub name: String,
    /// Display label for the property panel; falls back to `name`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    pub kind: GeneratorParamKind,
    pub value: Value,
}

impl GeneratorParam {
    pub fn display_label(&self) -> &str {
        self.label
            .as_deref()
            .filter(|label| !label.trim().is_empty())
            .unwrap_or(&self.name)
    }

    /// Validate `value` against the declared kind.
    pub fn validate(&self) -> Result<(), GeneratorError> {
        check_value(&self.name, self.kind, &self.value)
    }

    /// Parse a raw property-panel draft into a value of this parameter's
    /// kind. Numbers accept a leading sign and a decimal point; colors
    /// accept `#RGB` / `#RRGGBB` / `#RRGGBBAA` and canonicalize to upper
    /// case; booleans accept `true/false/1/0/yes/no/on/off`.
    pub fn parse_input(&self, raw: &str) -> Result<Value, GeneratorError> {
        let raw = raw.trim();
        let invalid = |reason: &str| GeneratorError::InvalidParamValue {
            name: self.name.clone(),
            reason: reason.to_string(),
        };
        let value = match self.kind {
            GeneratorParamKind::Number => {
                let number: f64 = raw.parse().map_err(|_| invalid("expected a number"))?;
                serde_json::Number::from_f64(number)
                    .map(Value::Number)
                    .ok_or_else(|| invalid("expected a finite number"))?
            }
            GeneratorParamKind::Text => Value::String(raw.to_string()),
            GeneratorParamKind::Color => {
                Value::String(canonical_color(raw).ok_or_else(|| invalid("expected #RRGGBB"))?)
            }
            GeneratorParamKind::Boolean => match raw.to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Value::Bool(true),
                "false" | "0" | "no" | "off" => Value::Bool(false),
                _ => return Err(invalid("expected true or false")),
            },
        };
        check_value(&self.name, self.kind, &value)?;
        Ok(value)
    }

    /// The value formatted for a property-panel text field.
    pub fn display_value(&self) -> String {
        match &self.value {
            Value::String(text) => text.clone(),
            Value::Bool(flag) => flag.to_string(),
            Value::Number(number) => match number.as_f64() {
                Some(v) if v.fract() == 0.0 && v.abs() < 1e15 => format!("{}", v as i64),
                Some(v) => format!("{v}"),
                None => number.to_string(),
            },
            other => other.to_string(),
        }
    }
}

/// The program + parameters stored on a generator frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratorSpec {
    pub v: u32,
    pub engine: String,
    pub program: String,
    #[serde(default)]
    pub params: Vec<GeneratorParam>,
    /// Built-in starter id this generator was created from, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starter: Option<String>,
    /// The node's human `explain` before it became a generator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_explain: Option<String>,
    /// [`output_hash`] of the children this spec last wrote.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_hash: Option<String>,
}

impl GeneratorSpec {
    /// A fresh JS spec.
    pub fn new(program: impl Into<String>, params: Vec<GeneratorParam>) -> Self {
        Self {
            v: GENERATOR_SPEC_VERSION,
            engine: GENERATOR_ENGINE_JS.to_string(),
            program: program.into(),
            params,
            starter: None,
            original_explain: None,
            output_hash: None,
        }
    }

    /// Structural validation: engine, program size, parameter names,
    /// uniqueness, count, and each value against its kind.
    pub fn validate(&self) -> Result<(), GeneratorError> {
        if self.engine != GENERATOR_ENGINE_JS {
            return Err(GeneratorError::UnsupportedEngine(self.engine.clone()));
        }
        if self.program.len() > MAX_PROGRAM_BYTES {
            return Err(GeneratorError::ProgramTooLarge {
                bytes: self.program.len(),
                max: MAX_PROGRAM_BYTES,
            });
        }
        if self.program.trim().is_empty() {
            return Err(GeneratorError::InvalidSpec("program is empty".into()));
        }
        if self.params.len() > MAX_GENERATOR_PARAMS {
            return Err(GeneratorError::InvalidSpec(format!(
                "{} parameters (max {MAX_GENERATOR_PARAMS})",
                self.params.len()
            )));
        }
        for (index, param) in self.params.iter().enumerate() {
            if !is_identifier(&param.name) {
                return Err(GeneratorError::InvalidSpec(format!(
                    "parameter name {:?} must be a JavaScript identifier",
                    param.name
                )));
            }
            if self.params[..index].iter().any(|p| p.name == param.name) {
                return Err(GeneratorError::InvalidSpec(format!(
                    "duplicate parameter {}",
                    param.name
                )));
            }
            param.validate()?;
        }
        Ok(())
    }

    /// Parameters as the `params` object the program reads.
    pub fn params_object(&self) -> Value {
        let mut map = serde_json::Map::new();
        for param in &self.params {
            map.insert(param.name.clone(), param.value.clone());
        }
        Value::Object(map)
    }

    /// Overwrite named parameter values, validating each against its kind.
    /// Unknown names are refused rather than silently added: the program
    /// declares its inputs, callers only tune them.
    pub fn apply_param_values(
        &mut self,
        values: &serde_json::Map<String, Value>,
    ) -> Result<(), GeneratorError> {
        for (name, value) in values {
            let param = self
                .params
                .iter_mut()
                .find(|param| &param.name == name)
                .ok_or_else(|| GeneratorError::UnknownParam(name.clone()))?;
            let value = coerce_value(param.kind, value);
            check_value(&param.name, param.kind, &value)?;
            param.value = value;
        }
        Ok(())
    }

    /// The `explain` string that stores this spec.
    pub fn to_explain(&self) -> String {
        let json = serde_json::to_string(self).expect("generator spec serializes");
        format!("{GENERATOR_EXPLAIN_PREFIX}{json}")
    }

    /// Parse a spec out of an `explain` string. `None` when the string is
    /// not a generator marker at all.
    pub fn from_explain(explain: &str) -> Option<Result<Self, GeneratorError>> {
        let json = explain.strip_prefix(GENERATOR_EXPLAIN_PREFIX)?;
        Some(
            serde_json::from_str::<GeneratorSpec>(json)
                .map_err(|error| GeneratorError::InvalidSpec(error.to_string())),
        )
    }
}

/// The spec stored on `node`, if it is a generator. `Some(Err(..))` flags a
/// marker whose payload no longer parses (hand-edited file).
pub fn generator_spec_of(node: &PenNode) -> Option<Result<GeneratorSpec, GeneratorError>> {
    GeneratorSpec::from_explain(node.base().explain.as_deref()?)
}

/// True when `node` carries a generator marker (valid or not).
pub fn is_generator(node: &PenNode) -> bool {
    node.base()
        .explain
        .as_deref()
        .is_some_and(|explain| explain.starts_with(GENERATOR_EXPLAIN_PREFIX))
}

/// Everything a runtime needs to run one generator.
#[derive(Debug, Clone, Copy)]
pub struct GeneratorRequest<'a> {
    /// The generator node's id — the Math.random seed.
    pub generator_id: &'a str,
    pub spec: &'a GeneratorSpec,
    /// The generator frame with its children removed; the program's
    /// `root` insert target.
    pub frame: &'a PenNode,
}

/// A generator runtime: run the program, return the raw children it built
/// (ids are rewritten by [`materialize_children`] afterwards).
pub type GeneratorRunner = fn(&GeneratorRequest<'_>) -> Result<Vec<PenNode>, GeneratorError>;

static RUNNER: OnceLock<GeneratorRunner> = OnceLock::new();

/// Install the process-wide runtime. Hosts that can run programs (desktop,
/// headless services) call this once at startup. The browser bundle
/// installs a daemon-backed runner (see `generator_remote.rs`) only once
/// its daemon advertises the `generators` capability; until then — and
/// for good when the daemon cannot run programs — its editor treats
/// generators as read-only materialized frames.
/// Returns `false` when a runtime was already installed.
pub fn install_generator_runner(runner: GeneratorRunner) -> bool {
    RUNNER.set(runner).is_ok()
}

/// The installed runtime, if any.
pub fn installed_generator_runner() -> Option<GeneratorRunner> {
    RUNNER.get().copied()
}

fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic() || c == '_' || c == '$')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
}

fn canonical_color(raw: &str) -> Option<String> {
    let digits = raw.trim().strip_prefix('#')?;
    if !digits.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let expanded = match digits.len() {
        3 => digits.chars().flat_map(|c| [c, c]).collect::<String>(),
        6 | 8 => digits.to_string(),
        _ => return None,
    };
    Some(format!("#{}", expanded.to_ascii_uppercase()))
}

/// Lenient MCP input: numbers may arrive as numeric strings, booleans as
/// `"true"`, colors in any accepted spelling. Numbers are canonicalized to
/// f64 so `5` and `5.0` store identically (a re-send is then a no-op).
fn coerce_value(kind: GeneratorParamKind, value: &Value) -> Value {
    match (kind, value) {
        (GeneratorParamKind::Number, Value::Number(number)) => number
            .as_f64()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or_else(|| value.clone()),
        (GeneratorParamKind::Number, Value::String(raw)) => raw
            .trim()
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or_else(|| value.clone()),
        (GeneratorParamKind::Boolean, Value::String(raw)) => match raw.trim() {
            "true" => Value::Bool(true),
            "false" => Value::Bool(false),
            _ => value.clone(),
        },
        (GeneratorParamKind::Color, Value::String(raw)) => canonical_color(raw)
            .map(Value::String)
            .unwrap_or_else(|| value.clone()),
        _ => value.clone(),
    }
}

fn check_value(name: &str, kind: GeneratorParamKind, value: &Value) -> Result<(), GeneratorError> {
    let invalid = |reason: &str| GeneratorError::InvalidParamValue {
        name: name.to_string(),
        reason: reason.to_string(),
    };
    match kind {
        GeneratorParamKind::Number => match value.as_f64() {
            Some(v) if v.is_finite() => Ok(()),
            _ => Err(invalid("expected a number")),
        },
        GeneratorParamKind::Text => match value.as_str() {
            Some(text) if text.chars().count() <= MAX_TEXT_PARAM_CHARS => Ok(()),
            Some(_) => Err(invalid("text too long")),
            None => Err(invalid("expected text")),
        },
        GeneratorParamKind::Color => match value.as_str().and_then(canonical_color) {
            Some(canonical) if Some(canonical.as_str()) == value.as_str() => Ok(()),
            _ => Err(invalid("expected #RRGGBB")),
        },
        GeneratorParamKind::Boolean => {
            if value.is_boolean() {
                Ok(())
            } else {
                Err(invalid("expected true or false"))
            }
        }
    }
}

#[cfg(test)]
#[path = "generator_tests.rs"]
mod tests;
