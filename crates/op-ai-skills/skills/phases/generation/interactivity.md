---
name: interactivity
description: State, bindings, and events so generated widgets are functional
phase: [generation]
trigger:
  keywords: [interactive, interactivity, clickable, functional, prototype, stateful, 交互, 可交互, 原型, 可点击]
priority: 25
budget: 1800
category: domain
---

INTERACTIVITY (state + bindings + events):

STATE — declare reactive values on the node that owns them (`StateSchema`):

Attach a `state` object to a node or to the document root. Each key is a
variable name; the value is a `StateEntry` with a required `type` field and
an optional `default`.

Primitive types: `int` / `float` / `number` / `string` / `bool` / `array`
/ `object` / `date`.

Examples (grounded in `counter.op` + `form.op` + `full-jian-extensions.op`):

```json
"state": {
  "count": { "type": "int",    "default": 0  },
  "name":  { "type": "string", "default": "" },
  "email": { "type": "string", "default": "" }
}
```

Add `"persist": true` to survive app restarts:

```json
"state": { "token": { "type": "string", "default": "", "persist": true } }
```

CROSS-SECTION SHARED STATE — use `$app.*` (document-root state) for values
that two or more independently-generated sections must read or write. A
counter button in one section and a display label in another section both
reach `$app.count` without coupling their node trees. Per-section private
values may use `$state.*`. When in doubt, prefer `$app.*`.

BINDINGS — declarative reads (`bind:value`, `content`):

`bindings` is a map from property name to an expression string. Use it to
keep node properties in sync with state automatically.

- Read-only bind: `"bindings": { "content": "\"Count: \" + $app.count" }`
  (the text node displays the live count, grounded in `full-jian-extensions.op:36`)
- Two-way bind on an input: `"bindings": { "bind:value": "$app.email" }`
  (`bind:value` writes back to the state variable when the user types)
- Template literal (backtick syntax also accepted by the expression parser):
  `"bindings": { "content": "\`Count: ${$app.count}\`" }`

EVENTS — action handlers:

`events` is an `EventHandlers` object. Each field maps to an `ActionList`
(array of `Action` objects). An `Action` is exactly one key:

```
{ "<action_name>": <body> }
```

Supported event hook keys (camelCase, `#[serde(rename_all = "camelCase")]`):

- **Gesture:** `onTap` / `onDoubleTap` / `onLongPress`
- **Drag:** `onPanStart` / `onPanUpdate` / `onPanEnd`
- **Input-node only:** `onChange` / `onSubmit` / `onFocus` / `onBlur`
- **Scroll:** `onScroll` / `onReachEnd`

Action vocabulary (body shape per action):

| Action   | Body                                                             | Effect                                   |
|----------|------------------------------------------------------------------|------------------------------------------|
| `set`    | `{ "<path>": "<expr>" }` map of assignments                     | Write one or more state variables         |
| `toggle` | `"<path>"` — the bool variable to flip                           | Toggle a bool state variable              |
| `toast`  | `"<message expr>"` string or template literal                    | Show a transient notification             |
| `push`   | `"<route path>"` string                                          | Navigate to a named route                 |
| `if`     | `{ "expr": "<condition>", "then": [...], "else": [...] }`        | Conditional action branch (`else` optional) |

Examples (grounded in `full-jian-extensions.op` + `form.op`):

```json
"events": {
  "onTap": [
    { "set": { "$app.count": "$app.count + 1" } },
    {
      "if": {
        "expr": "$app.count >= $app.target",
        "then": [{ "toast": "Done!" }, { "push": "/stats" }]
      }
    }
  ]
}
```

```json
"events": {
  "onChange": [{ "set": { "$app.name": "$event.value" } }],
  "onSubmit": [{ "toast": "`Submitted ${$app.name} <${$app.email}>`" }]
}
```

EXPRESSION LANGUAGE:

- `$app.<key>` — document-root state (cross-section, shared across pages)
- `$state.<key>` — local state on the node that declares it (private)
- `$event.value` — the originating event's payload (e.g. the text typed in
  an `onChange` from an input node)
- Arithmetic: `$app.count + 1`, `$app.total * 0.1`
- Comparison: `$app.count >= $app.target`, `$app.name != ""`
- String concatenation: `"Hello " + $app.name`
- Template literals (backtick, resolved by the expression parser):
  `` `Count: ${$app.count}` ``

PLACEMENT RULES:

- Declare `state` on the **lowest common ancestor** node that all bindings /
  event handlers need. For cross-section designs, declare on the document root.
- `bindings` lives on the node whose property is driven (e.g. the `text`
  node whose `content` reflects a counter, or the `text_input` node whose
  `value` binds a form field).
- `events` lives on the interactive node (button frame, input node, list
  item) — NOT on a wrapper layout frame that has no tap/change semantics.
- Input nodes (`text_input`) use `bind:value` for two-way sync and
  `onChange` to write `$event.value` back to state. Do not manually echo
  `$event.value` into a display node — use a `bindings.content` expression
  instead.

CORRECTNESS CHECKLIST:

- Every `StateEntry` MUST have a `type` key (not `kind`, not `dataType`).
- `onTap` / `onChange` / `onSubmit` are camelCase exactly as shown.
- An `Action` object has exactly one key — `{ "set": ... }` not
  `{ "set": ..., "toast": ... }` (that would be two actions — put them in
  separate array elements).
- `set` body is an object mapping variable paths to expression strings.
- `toggle` body is a single string (the variable path), not an object.
- `push` body is a route path string, not an object.
- `if` body has an `expr` string plus `then` array; `else` is optional.
- Expression strings are plain JSON strings — no special encoding needed.
