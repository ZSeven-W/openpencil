---
name: interactivity
description: State, bindings, and events so generated widgets are functional
phase: [generation]
trigger:
  keywords: [interactive, interactivity, clickable, functional, prototype, stateful, motion, animation, animated, mount, inView, transition, 交互, 可交互, 原型, 可点击, 可操作, 动效, 动画]
priority: 25
budget: 2000
category: domain
---

INTERACTIVITY (state + bindings + events):

Apply only to requested interactive behavior; static/mockup-only instructions take
priority. Preserve the design, content and layout. Do not add business services.
A toast confirms an action; it does not implement selection, quantity or totals.

MOTION — use a 200–500 ms `mount` fade-up for first content; `inView` reveals
may stagger repeated cards with `delayMs`. Use Material 3 easing: `standard`,
`emphasized`, `emphasizedDecelerate`, `emphasizedAccelerate`. Keep lifecycle
motion separate from scroll timelines.

STATE — declare reactive values on the owning node or document root. Each entry needs
`type` (int / float / number / string / bool / array / object / date) and an
intentional `default` for initially visible values. Static content/fill does not
initialize bindings. Null is valid for an intentionally empty selection.
`persist:true` is optional, only when restart persistence is requested.
Use `$app.*` for cross-section/page state. `$state.*` is an alias of `$app.*`,
not private node-local state. Use distinct keys for independent items/groups.

BINDINGS — declarative reads: map supported property names to runtime expression strings:
- Read-only: `bindings:{content:"'Count: ' + to_str($app.count)"}`.
- Input two-way: `bindings:{'bind:value':'$app.email'}`. Input events can write
  `$event.value` with `onChange`; `onSubmit` can confirm submission.
- Selection: bind the chip's `fill` and its label's `fill` to the SAME selected
  key that its `events.onTap` writes. `selected` and `fontWeight` are not binding
  targets. Constants, unused state and toast-only handlers do not change selection.
- Quantity: bind the quantity label AND every requested dependent row amount,
  subtotal and final total to those same quantity keys. Do not hardcode a total
  after making the counter interactive. Do not maintain a second stale total.
- Keep static content/fill equal to the default-state rendering for exports.
  Static `fill`/`stroke.color` are literal colors/design variables; expressions
  belong only in `bindings`. For a transient panel use authored `visible:false`
  plus `bindings.visible`; opacity:0 alone still intercepts input/reserves space.

EVENTS — action handlers:
`events` maps camelCase hooks to arrays of single-key action objects.
Gesture: `onTap`, `onDoubleTap`, `onLongPress`; drag: `onPanStart`, `onPanUpdate`,
`onPanEnd`; input: `onChange`, `onSubmit`, `onFocus`, `onBlur`; scroll:
`onScroll`, `onReachEnd`.

| Action | Body | Effect |
|---|---|---|
| `set` | `{ "<path>": "<expr>" }` | Write state; values are expression STRINGS |
| `toggle` | `"<path>"` | Flip a bool |
| `toast` | `"<message expr>"` | Transient feedback |
| `push` | `"\"/details\""` | Drill into a screen; allows back |
| `replace` | `"\"/profile\""` | Switch sibling screen/tab |
| `pop` | `null` | Return to previous screen |
| `if` | `{ "expr":"<condition>", "then":[...], "else":[...] }` | Branch; else optional |

The following event snippets belong to the corresponding button/input nodes:
```json
"events": {
  "onTap": [{"set":{"$app.count":"$app.count + 1"}}],
  "onChange": [{"set":{"$app.name":"$event.value"}}],
  "onSubmit": [{"toast":"'Submitted'"}]
}
```
```json
{"events":{"onTap":[{ "replace": "\"/profile\"" }]}}
```

EXPRESSION LANGUAGE — builder scripts are JavaScript; saved action/binding
expressions are Jian. Use `to_str(value)`, never `String(value)` or `.toFixed(2)`
inside a Jian expression. Arithmetic, comparison, ternary, concatenation and
backtick templates are supported. String literals need inner quotes:
`{"set":{"$app.capacity":"'450ml'"}}`, `{"toast":"'Added to cart'"}`.
In the outer JavaScript builder `JSON.stringify(value)` encodes a literal string.
`set` targets need real scopes (`$app.count`, not bare `count`); bool/number RHS
also need expression strings (`"true"`, `"1"`, never raw true/1).

WORKED SELECTION — syntax example only, not supplied product data or styling.
Adapt values, keys, colors and dimensions to the requested design. Each group
owns a different key. Both options share it; clicking an already selected option
may leave it unchanged. Native radio_group/select may use bind:value instead.

```js
const group = I(null,{type:'frame',name:'Specification',layout:'horizontal',gap:8,
  state:{finish:{type:'string',default:'cream'}}});
for (const [value,label] of [['cream','Cream'],['green','Green']]) {
  const active = '$app.finish == ' + JSON.stringify(value);
  const chip = I(group,{type:'frame',name:'Finish '+label,layout:'horizontal',
    width:88,height:36,cornerRadius:18,fill:value==='cream'?'#172A24':'#F1F4F2',
    bindings:{fill:active+" ? '#172A24' : '#F1F4F2'"},
    events:{onTap:[{set:{'$app.finish':JSON.stringify(value)}}]}});
  I(chip,{type:'text',name:'Finish label '+label,content:label,
    fill:value==='cream'?'#FFFFFF':'#172A24',
    bindings:{fill:active+" ? '#FFFFFF' : '#172A24'"}});
}
```

WORKED CART — two independently adjustable rows, each with its own quantity;
all visible money reads the same quantities. These fictional numbers demonstrate
syntax only. Use supplied/shared entity prices, requested discount/tax/shipping
rules and initial quantities; do not invent fees, offers or a minimum quantity.
This example's minimum is explicitly one. No real checkout/payment is performed.

```js
const cart = I(null,{type:'frame',name:'Cart',layout:'vertical',gap:12,
  state:{qtyA:{type:'int',default:1},qtyB:{type:'int',default:1}}});
const items = [['A',24],['B',36]];
for (const [key,price] of items) {
  const path = '$app.qty'+key;
  const row = I(cart,{type:'frame',name:'Item '+key,layout:'horizontal',gap:8});
  for (const [name,label,expr] of [
    ['minus','−',path+' > 1 ? '+path+' - 1 : 1'],['plus','+',path+' + 1']]) {
    const button = I(row,{type:'frame',name:key+' '+name,width:32,height:32,
      events:{onTap:[{set:{[path]:expr}}]}});
    I(button,{type:'text',content:label});
  }
  I(row,{type:'text',name:key+' quantity',content:'1',bindings:{content:'to_str('+path+')'}});
  I(row,{type:'text',name:key+' amount',content:'¥'+price,
    bindings:{content:"'¥' + to_str("+price+' * '+path+')'}});
}
const subtotal = '24 * $app.qtyA + 36 * $app.qtyB';
I(cart,{type:'text',name:'Subtotal',content:'¥60',bindings:{content:"'¥' + to_str("+subtotal+')'}});
I(cart,{type:'text',name:'Total',content:'¥60',bindings:{content:"'¥' + to_str("+subtotal+')'}});
```

MULTI-SCREEN NAVIGATION — only top-level frames carry `screen` route strings:
exactly ONE entry `"screen":"/"`; other frames have unique paths. The orchestrator
supplies exact routes; reuse them. A nested `screen` is ignored; the `route` field is schema-only and does not
navigate. Use `push` on detail-opening cards, `replace` on tabs/sidebar, `pop`
on back controls: `{ "pop": null }`. Routes are expression strings including literal quotes:
`events:{onTap:[{push:'"/details"'}]}`, never `{push:'/details'}`.

PLACEMENT / CORRECTNESS:
- Declare state at the lowest common ancestor; document-root state for shared
  sections. Declare `StateEntry.type`, not `kind`/`dataType`.
- Bind the visible node's property, not a wrapper. Put tap actions on the actual
  control; its label/icon can bubble to it. Do not put separate handlers on labels
  that intercept the intended control.
- `set`, `toast`, `push`, etc. are separate single-key objects in the action array.
- Before returning, trace two different option taps, then plus and minus for EACH
  cart row. All requested visible summaries must agree with the resulting state.
  Keep unrelated items/selections unchanged. Preserve initial exported appearance.
