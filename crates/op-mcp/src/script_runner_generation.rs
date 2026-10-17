//! The fresh generation sandbox cannot address existing document nodes.

pub(super) const PREAMBLE: &str = r#"
__normalizeForegroundContrast = function(obj) { return obj; };
(function() {
  var handles = Object.create(null);
  var authoredIds = Object.create(null);
  var parentBinding = __parentBinding;
  var recordInsert = __record;
  var recordKit = __recordK;
  var kitInsert = K;
  function rememberAuthoredIds(node) {
    if (node === null || typeof node !== "object" || Array.isArray(node)) return;
    if (typeof node.id === "string" && !node.id.startsWith("__op_tmp_")) {
      authoredIds[node.id] = true;
    }
    if (Array.isArray(node.children)) node.children.forEach(rememberAuthoredIds);
  }
  __parentBinding = function(parent) {
    var binding = parentBinding(parent);
    if (parent !== null && !Object.prototype.hasOwnProperty.call(handles, binding)) {
      throw new Error("OP_SCRIPT_PARENT_UNRESOLVED: insertion parent " + JSON.stringify(binding) + " cannot be resolved in this generation script. Node names are not parent handles. Save the value returned by I() or K() and pass that value directly, for example const button = I(section, {...}); I(button, {...}). Use explicit null only for a root node.");
    }
    return binding;
  };
  __record = function(parent, json, original, background) {
    var binding = recordInsert(parent, json, original, background);
    handles[binding] = true;
    rememberAuthoredIds(JSON.parse(json));
    return binding;
  };
  __recordK = function(kit, parent, json) {
    var binding = recordKit(kit, parent, json);
    handles[binding] = true;
    return binding;
  };
  // The K executor resolves authored ids (including inline descendants)
  // through its alias table. I does not; do not extend that contract here.
  globalThis.K = function(kit, parent, overrides) {
    var binding = parentBinding(parent);
    if (parent !== null && Object.prototype.hasOwnProperty.call(authoredIds, binding)) {
      return __recordK(JSON.stringify(String(kit)), binding, JSON.stringify(overrides == null ? {} : overrides));
    }
    return kitInsert(kit, parent, overrides);
  };
})();
"#;

#[cfg(test)]
#[path = "script_runner_generation_tests.rs"]
mod tests;
