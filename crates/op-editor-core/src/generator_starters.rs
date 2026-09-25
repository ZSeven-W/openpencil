//! Built-in generator starters, insertable from the toolbar shape menu
//! and by id through the `create_generator` MCP tool.
//!
//! Each starter is a frame template plus a JS program written against the
//! generator runtime's API: `params.<name>` for inputs, `root` as the
//! insert target, `I(parent, node)` returning a binding for nested
//! inserts. Dates go through `Date.UTC` / `getUTC*` so the month grid does
//! not depend on the host time zone.

use jian_ops_schema::node::PenNode;
use serde_json::{json, Value};

use super::{GeneratorParam, GeneratorParamKind, GeneratorSpec};

/// One built-in starter.
#[derive(Debug, Clone, Copy)]
pub struct GeneratorStarter {
    /// Stable id (`create_generator { starter }`, stored in the spec).
    pub id: &'static str,
    /// i18n key of the menu label.
    pub label_key: &'static str,
    /// Default layer name.
    pub name: &'static str,
    program: &'static str,
    frame: fn() -> Value,
    params: fn() -> Vec<GeneratorParam>,
}

/// Starters are identified by id (their fn pointers have no stable identity).
impl PartialEq for GeneratorStarter {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for GeneratorStarter {}

impl GeneratorStarter {
    /// The starter's spec with default parameters.
    pub fn spec(&self) -> GeneratorSpec {
        let mut spec = GeneratorSpec::new(self.program, (self.params)());
        spec.starter = Some(self.id.to_string());
        spec
    }

    /// The generator frame (no children, placeholder id) at `(x, y)`.
    pub fn frame(&self, x: f64, y: f64) -> PenNode {
        let mut value = (self.frame)();
        value["id"] = json!("generator");
        value["name"] = json!(self.name);
        value["x"] = json!(x);
        value["y"] = json!(y);
        serde_json::from_value(value).expect("starter frame template is a valid frame")
    }

    /// Look a starter up by id.
    pub fn by_id(id: &str) -> Option<&'static GeneratorStarter> {
        GENERATOR_STARTERS.iter().find(|starter| starter.id == id)
    }
}

/// Every built-in starter, in menu order.
pub static GENERATOR_STARTERS: [GeneratorStarter; 2] = [
    GeneratorStarter {
        id: "card-grid",
        label_key: "generator.starterCardGrid",
        name: "Card grid",
        program: CARD_GRID_PROGRAM,
        frame: card_grid_frame,
        params: card_grid_params,
    },
    GeneratorStarter {
        id: "month-calendar",
        label_key: "generator.starterCalendar",
        name: "Month calendar",
        program: CALENDAR_PROGRAM,
        frame: calendar_frame,
        params: calendar_params,
    },
];

fn param(name: &str, label: &str, kind: GeneratorParamKind, value: Value) -> GeneratorParam {
    GeneratorParam {
        name: name.to_string(),
        label: Some(label.to_string()),
        kind,
        value,
    }
}

fn surface_frame(fill: &str) -> Value {
    json!({
        "type": "frame",
        "layout": "vertical",
        "gap": 12,
        "padding": 24,
        "width": "fit_content",
        "height": "fit_content",
        "cornerRadius": 16,
        "fill": [{"type": "solid", "color": fill}],
        "stroke": {"thickness": 1, "fill": [{"type": "solid", "color": "#E5E7EB"}]}
    })
}

fn card_grid_frame() -> Value {
    surface_frame("#F9FAFB")
}

fn card_grid_params() -> Vec<GeneratorParam> {
    use GeneratorParamKind::*;
    vec![
        param("title", "Title", Text, json!("Quarterly metrics")),
        param(
            "rows",
            "Rows (label: value; …)",
            Text,
            json!(
                "Revenue: $48k; Users: 12,480; Churn: 2.1%; NPS: 61; Tickets: 312; Uptime: 99.98%"
            ),
        ),
        param("columns", "Columns", Number, json!(3)),
        param("accent", "Accent", Color, json!("#6366F1")),
        param("showValues", "Show values", Boolean, json!(true)),
    ]
}

const CARD_GRID_PROGRAM: &str = r##"
var items = String(params.rows).split(";").map(function (s) { return s.trim(); }).filter(function (s) { return s.length > 0; });
var cols = Math.max(1, Math.min(8, Math.round(params.columns)));
if (String(params.title).trim().length > 0) {
  I(root, {type: "text", name: "Title", content: String(params.title), fontFamily: "Inter", fontSize: 20, fontWeight: 700, fill: [{type: "solid", color: "#111827"}]});
}
var row = null;
items.forEach(function (item, i) {
  if (i % cols === 0) {
    row = I(root, {type: "frame", name: "Row " + (i / cols + 1), layout: "horizontal", gap: 12, width: "fit_content", height: "fit_content"});
  }
  var colon = item.indexOf(":");
  var label = colon >= 0 ? item.slice(0, colon).trim() : item;
  var value = colon >= 0 ? item.slice(colon + 1).trim() : "";
  var card = I(row, {type: "frame", name: label, layout: "vertical", gap: 6, padding: 16, width: 168, height: "fit_content", cornerRadius: 12, fill: [{type: "solid", color: "#FFFFFF"}], stroke: {thickness: 1, fill: [{type: "solid", color: "#E5E7EB"}]}});
  I(card, {type: "rectangle", name: "Accent", width: 28, height: 4, cornerRadius: 2, fill: [{type: "solid", color: params.accent}]});
  I(card, {type: "text", name: "Label", content: label, fontFamily: "Inter", fontSize: 13, fontWeight: 500, fill: [{type: "solid", color: "#6B7280"}]});
  if (params.showValues && value.length > 0) {
    I(card, {type: "text", name: "Value", content: value, fontFamily: "Inter", fontSize: 22, fontWeight: 700, fill: [{type: "solid", color: "#111827"}]});
  }
});
"##;

fn calendar_frame() -> Value {
    surface_frame("#FFFFFF")
}

fn calendar_params() -> Vec<GeneratorParam> {
    use GeneratorParamKind::*;
    vec![
        param("year", "Year", Number, json!(2026)),
        param("month", "Month (1-12)", Number, json!(9)),
        param("highlight", "Highlight day (0 = none)", Number, json!(25)),
        param("startMonday", "Week starts Monday", Boolean, json!(true)),
        param("accent", "Accent", Color, json!("#6366F1")),
    ]
}

const CALENDAR_PROGRAM: &str = r##"
var year = Math.round(params.year);
var month = Math.max(1, Math.min(12, Math.round(params.month)));
var names = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
var first = new Date(Date.UTC(year, month - 1, 1)).getUTCDay();
var days = new Date(Date.UTC(year, month, 0)).getUTCDate();
var offset = params.startMonday ? (first + 6) % 7 : first;
var labels = params.startMonday ? ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"] : ["Su", "Mo", "Tu", "We", "Th", "Fr", "Sa"];
I(root, {type: "text", name: "Month", content: names[month - 1] + " " + year, fontFamily: "Inter", fontSize: 18, fontWeight: 700, fill: [{type: "solid", color: "#111827"}]});
function cell(parent, name, text, color, fill) {
  var box = I(parent, {type: "frame", name: name, width: 36, height: 32, layout: "vertical", alignItems: "center", justifyContent: "center", cornerRadius: 8, fill: fill ? [{type: "solid", color: fill}] : []});
  if (text.length > 0) {
    I(box, {type: "text", name: "Label", content: text, fontFamily: "Inter", fontSize: 13, fontWeight: 500, fill: [{type: "solid", color: color}]});
  }
}
var head = I(root, {type: "frame", name: "Weekdays", layout: "horizontal", gap: 4, width: "fit_content", height: "fit_content"});
labels.forEach(function (label) { cell(head, label, label, "#9CA3AF", null); });
var weeks = Math.ceil((offset + days) / 7);
for (var w = 0; w < weeks; w++) {
  var row = I(root, {type: "frame", name: "Week " + (w + 1), layout: "horizontal", gap: 4, width: "fit_content", height: "fit_content"});
  for (var k = 0; k < 7; k++) {
    var day = w * 7 + k - offset + 1;
    var inMonth = day >= 1 && day <= days;
    var marked = inMonth && day === Math.round(params.highlight);
    cell(row, inMonth ? "Day " + day : "Empty", inMonth ? String(day) : "", marked ? "#FFFFFF" : "#111827", marked ? params.accent : null);
  }
}
"##;
