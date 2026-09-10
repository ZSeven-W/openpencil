---
name: interactivity
description: State, bindings, and events so generated widgets are functional
phase: [generation]
trigger:
  keywords: [interactive, interactivity, clickable, functional, prototype, stateful, motion, animation, animated, mount, inView, transition, 交互, 可交互, 原型, 可点击, 动效, 动画]
priority: 25
budget: 2000
category: domain
---

INTERACTIVITY (state + bindings + events):

MOTION — use a 200–500 ms `mount` fade-up for a hero or first content, and `inView` fade-up reveals for repeated cards with staggered `delayMs` (during `delayMs` the node shows keyframe 0); use the Material 3 easing tokens `standard`, `emphasized`, `emphasizedDecelerate`, or `emphasizedAccelerate`, and keep lifecycle motion separate from a scroll timeline.

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
two or more independently-generated sections must read or write (e.g. a
counter button in one section and a display label in another both reach
`$app.count` without coupling their node trees). Per-section private values
may use `$state.*`. When in doubt, prefer `$app.*`.

BINDINGS — declarative reads (`bind:value`, `content`):

`bindings` maps a property name to an expression string, keeping node
properties in sync with state automatically.

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

| Action | Body | Effect |
|---|---|---|
| `set` | `{ "<path>": "<expr>" }` map of assignments | Write one or more state variables |
| `toggle` | `"<path>"` — the bool variable to flip | Toggle a bool state variable |
| `toast` | `"<message expr>"` string or template literal | Show a transient notification |
| `push` | `"\"<route path>\""` — a JSON string whose VALUE is itself `"<route path>"`, quotes included | Drill into a screen (back/pop can return) |
| `replace` | `"\"<route path>\""` — same quote-literal shape as `push` | Switch to a sibling screen (tab bar / sidebar — no back entry) |
| `pop` | `null` — no body | Return to the previous screen |
| `if` | `{ "expr": "<condition>", "then": [...], "else": [...] }` | Conditional branch (`else` optional) |

`push` / `replace` bodies compile as a Tier-1 EXPRESSION, not a literal path — an unquoted `/stats` lexes as a division token and fails to compile. Always wrap the route path in an extra pair of escaped quotes: `{ "push": "\"/stats\"" }`, never `{ "push": "/stats" }`.

Examples (grounded in `full-jian-extensions.op` + `form.op`):

```json
"events": {
  "onTap": [
    { "set": { "$app.count": "$app.count + 1" } },
    {
      "if": {
        "expr": "$app.count >= $app.target",
        "then": [{ "toast": "Done!" }, { "push": "\"/stats\"" }]
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

MULTI-SCREEN NAVIGATION (App Mode preview) — `screen` marker + tap-to-switch:

A document with 2+ screens becomes a tappable, navigable app in Preview once
its top-level screen frames carry a `screen` route path and its nav elements
bind `push` / `replace` / `pop` as above. Mark exactly ONE top-level frame
`"screen": "/"` (the entry); every other screen gets a unique `/slug`, unique
across the whole document:

```json
{ "type": "frame", "id": "home", "name": "Home", "screen": "/" }
{ "type": "frame", "id": "profile", "name": "Profile", "screen": "/profile" }
```

Bind a bottom-tab-bar / sidebar item with `replace` (lateral move between
sibling screens); bind a card/row that opens a detail screen with `push`
(the user expects to come back FROM it); bind a header back-arrow with `pop`:

```json
{ "events": { "onTap": [ { "replace": "\"/profile\"" } ] } }
```

```json
{ "events": { "onTap": [ { "pop": null } ] } }
```

`screen` is valid ONLY on a top-level frame — a nested frame's `screen` value
is ignored by the routing projection. Never write a `route` field instead of
`events.onTap`: `route` is schema-only surface metadata the tap dispatcher
does not read, so a node with `route` but no `events.onTap` does nothing
when tapped.

PLACEMENT RULES:

- Declare `state` on the **lowest common ancestor** node that all bindings /
  event handlers need — the document root for cross-section designs.
- `bindings` lives on the node whose property is driven (e.g. the `text`
  node whose `content` reflects a counter, or the `text_input` node whose
  `value` binds a form field).
- `events` lives on the interactive node (button frame, input node, list
  item) — NOT on a wrapper layout frame with no tap/change semantics.
- Input nodes (`text_input`) use `bind:value` for two-way sync and
  `onChange` to write `$event.value` back to state — don't manually echo
  `$event.value` into a display node, use a `bindings.content` expression.

CORRECTNESS CHECKLIST:

- Every `StateEntry` MUST have a `type` key (not `kind`, not `dataType`).
- `onTap` / `onChange` / `onSubmit` are camelCase exactly as shown.
- An `Action` object has exactly one key — `{ "set": ... }` not
  `{ "set": ..., "toast": ... }` (that would be two actions — put them in
  separate array elements).
- `set` body is an object mapping variable paths to expression strings.
- `toggle` body is a single string (the variable path), not an object.
- `push` / `replace` bodies are the quote-literal string `"\"<path>\""`, not a
  bare path string and not an object — a bare `/path` fails to compile.
- `pop` body is `null`, never a path.
- `if` body has an `expr` string plus `then` array; `else` is optional.
- Expression strings are plain JSON strings — no special encoding needed.
- `screen` is a plain string on a top-level frame, never `route` — `route`
  is not consumed by the tap dispatcher.
