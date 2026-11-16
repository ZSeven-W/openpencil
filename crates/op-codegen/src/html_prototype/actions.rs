use super::{expression, HtmlPrototype};
use jian_core::motion::screen_transition::ScreenTransitionSpec;
use jian_core::state::{path::StatePath, Scope, Segment};
use jian_core::{action::policy::PreviewActionPolicy, motion::MotionControl};
use jian_ops_schema::events::ActionList;
use serde_json::{json, Value};

fn path(value: &str) -> Option<Value> {
    let path = StatePath::parse(value).ok()?;
    let [Segment::Key(key)] = path.segments.as_slice() else {
        return None;
    };
    match path.scope {
        Scope::App | Scope::Page | Scope::SelfNode | Scope::Vars => {
            Some(json!([path.scope.as_prefix(), key]))
        }
        _ => None,
    }
}

fn writable_scope(scope: &str) -> bool {
    matches!(scope, "$app" | "$page" | "$self" | "$vars")
}

fn writes(body: &Value) -> Option<Vec<Value>> {
    let object = body.as_object()?;
    if let (Some(target), Some(value)) = (object.get("target"), object.get("value")) {
        return Some(vec![json!([
            path(target.as_str()?)?,
            expression::from_value(value)?
        ])]);
    }
    object
        .iter()
        .map(|(target, value)| Some(json!([path(target)?, expression::from_value(value)?])))
        .collect()
}

fn transition(body: &Value) -> Option<Option<ScreenTransitionSpec>> {
    match body.get("transition").filter(|value| !value.is_null()) {
        Some(value) => serde_json::from_value(value.clone()).ok().map(Some),
        None => Some(None),
    }
}

impl HtmlPrototype {
    pub(super) fn compile_actions(&mut self, actions: &ActionList) -> Vec<Value> {
        self.compile_chain(actions.iter().filter_map(|action| action.single()))
    }

    fn compile_chain<'a>(
        &mut self,
        actions: impl Iterator<Item = (&'a str, &'a Value)>,
    ) -> Vec<Value> {
        actions
            .filter_map(|(verb, body)| self.compile_action(verb, body))
            .collect()
    }

    fn compile_list(&mut self, value: &Value) -> Option<Vec<Value>> {
        Some(self.compile_chain(value.as_array()?.iter().filter_map(single)))
    }

    fn compile_action(&mut self, verb: &str, body: &Value) -> Option<Value> {
        // Match Play's local action policy before lowering authored action data.
        if !PreviewActionPolicy::ALLOWED.contains(&verb) {
            return None;
        }
        Some(match verb {
            "set" => json!({"op": "set", "writes": writes(body)?}),
            "toggle" | "delete" => json!({"op": verb, "target": path(body.as_str()?)?}),
            "reset" if body.as_str().is_some_and(writable_scope) => {
                json!({"op": "reset_state", "scope": body})
            }
            "if" => {
                let test = expression::from_value(body.get("expr")?)?;
                let then = self.compile_list(body.get("then")?)?;
                let otherwise = body
                    .get("else")
                    .and_then(|v| self.compile_list(v))
                    .unwrap_or_default();
                json!({"op": "if", "test": test, "then": then, "else": otherwise})
            }
            "parallel" => {
                let branches: Vec<_> = body
                    .as_array()?
                    .iter()
                    .map(|branch| {
                        if branch.is_array() {
                            self.compile_list(branch).unwrap_or_default()
                        } else {
                            self.compile_chain(single(branch).into_iter())
                        }
                    })
                    .collect();
                json!({"op": "parallel", "branches": branches})
            }
            "delay" => json!({"op": "delay", "ms": body.get("ms")?.as_u64()?}),
            "play_motion" => {
                let control: MotionControl = serde_json::from_value(body.clone()).ok()?;
                json!({"op": "motion", "target": control.target, "animation": control.animation, "mode": control.mode})
            }
            "show" | "hide" | "toggle_visibility" | "focus" => {
                let target = body
                    .as_str()
                    .or_else(|| body.get("target")?.as_str())?
                    .trim();
                if target.is_empty() {
                    return None;
                }
                json!({"op": verb, "target": target})
            }
            "blur" | "dismiss_keyboard" => json!({"op": "blur"}),
            "push" | "replace" | "reset" if self.entry.is_some() => {
                if verb == "reset" && body.as_str().is_none_or(|value| value.starts_with('$')) {
                    return None;
                }
                let authored = if body.is_string() {
                    body
                } else {
                    body.get("path")?
                };
                let path = expression::from_value(authored)?;
                let effect = transition(body)?
                    .as_ref()
                    .map(|spec| self.profiles_for(Some(spec)));
                self.navigates = true;
                json!({"op": verb, "path": path, "effect": effect})
            }
            "pop" if self.entry.is_some() => {
                let effect = transition(body)?
                    .as_ref()
                    .map(|spec| self.profiles_for(Some(spec)));
                self.navigates = true;
                json!({"op": "pop", "effect": effect})
            }
            _ => return None,
        })
    }
}

fn single(value: &Value) -> Option<(&str, &Value)> {
    let object = value.as_object()?;
    if object.len() != 1 {
        return None;
    }
    object
        .iter()
        .next()
        .map(|(key, value)| (key.as_str(), value))
}
