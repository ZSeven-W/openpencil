//! Calendar TS element alias builders.

#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{json, Value};

use crate::ToolOutcome;

static NEXT_CALENDAR_ELEMENT_ID: AtomicU64 = AtomicU64::new(1);

pub(crate) fn calendar_alias_node_value(
    tool: &str,
    args: &BTreeMap<String, String>,
) -> Result<Option<Value>, ToolOutcome> {
    let value = match tool {
        "add_calendar_grid_v0" => build_calendar_grid(args, false)?,
        "add_calendar_grid_v1" => build_calendar_grid(args, true)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn build_calendar_grid(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let days_in_month = number_arg(args, "days_in_month", 30.0)
        .floor()
        .clamp(1.0, 31.0) as i64;
    let start_offset = number_arg(args, "start_day_offset", 0.0)
        .floor()
        .clamp(0.0, 6.0) as i64;
    let today = optional_i64_arg(args, "today");
    let selected = optional_i64_arg(args, "selected_day");
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = calendar_colors(theme, theme_aware);

    let mut rows = vec![calendar_header_row(colors.header_text)];
    let total_cells = ((start_offset + days_in_month + 6) / 7) * 7;
    let mut day_counter = 1_i64;
    for row in 0..(total_cells / 7) {
        let mut cells = Vec::new();
        for col in 0..7 {
            let idx = row * 7 + col;
            if idx < start_offset || day_counter > days_in_month {
                cells.push(json!({
                    "id": next_id("calendar_empty"),
                    "type": "frame",
                    "name": "Empty",
                    "role": "calendar-day-empty",
                    "width": 40,
                    "height": 40,
                }));
                continue;
            }
            let day = day_counter;
            let is_selected = selected == Some(day);
            let is_today = !is_selected && today == Some(day);
            cells.push(calendar_day_cell(day, is_selected, is_today, &colors));
            day_counter += 1;
        }
        rows.push(json!({
            "id": next_id("calendar_week"),
            "type": "frame",
            "name": format!("Week {}", row + 1),
            "role": "calendar-week",
            "width": "fit_content",
            "height": 40,
            "layout": "horizontal",
            "gap": 0,
            "children": cells,
        }));
    }

    Ok(json!({
        "id": next_id("calendar"),
        "type": "frame",
        "name": "Calendar",
        "role": "calendar-grid",
        "width": "fit_content",
        "height": "fit_content",
        "layout": "vertical",
        "gap": 0,
        "children": rows,
    }))
}

struct CalendarColors {
    header_text: &'static str,
    day_text: &'static str,
    selected_bg: &'static str,
    selected_text: &'static str,
    today_bg: &'static str,
}

fn calendar_colors(theme: &str, theme_aware: bool) -> CalendarColors {
    if !theme_aware || theme == "light" {
        return CalendarColors {
            header_text: "#6B7280",
            day_text: "#111827",
            selected_bg: "#2563EB",
            selected_text: "#FFFFFF",
            today_bg: "#DBEAFE",
        };
    }
    if theme == "system" {
        return CalendarColors {
            header_text: "$color-text-muted",
            day_text: "$color-text-primary",
            selected_bg: "$color-accent",
            selected_text: "$color-surface",
            today_bg: "$color-info-bg",
        };
    }
    CalendarColors {
        header_text: "#94A3B8",
        day_text: "#F1F5F9",
        selected_bg: "#60A5FA",
        selected_text: "#1E293B",
        today_bg: "#1E3A8A",
    }
}

fn calendar_header_row(header_fill: &str) -> Value {
    let weekdays = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    let children: Vec<Value> = weekdays
        .iter()
        .map(|day| {
            json!({
                "id": next_id("calendar_header"),
                "type": "frame",
                "name": format!("Header {day}"),
                "role": "calendar-header",
                "width": 40,
                "height": 40,
                "layout": "horizontal",
                "alignItems": "center",
                "justifyContent": "center",
                "children": [{
                    "id": next_id("calendar_header_label"),
                    "type": "text",
                    "name": "Label",
                    "content": day,
                    "fontSize": 12,
                    "fontWeight": 500,
                    "fill": [{ "type": "solid", "color": header_fill }],
                }],
            })
        })
        .collect();
    json!({
        "id": next_id("calendar_header_row"),
        "type": "frame",
        "name": "Weekday Header",
        "role": "calendar-header-row",
        "width": "fit_content",
        "height": 40,
        "layout": "horizontal",
        "gap": 0,
        "children": children,
    })
}

fn calendar_day_cell(
    day: i64,
    is_selected: bool,
    is_today: bool,
    colors: &CalendarColors,
) -> Value {
    let mut cell = json!({
        "id": next_id("calendar_day"),
        "type": "frame",
        "name": format!("Day {day}"),
        "role": if is_selected {
            "calendar-day-selected"
        } else if is_today {
            "calendar-day-today"
        } else {
            "calendar-day"
        },
        "width": 40,
        "height": 40,
        "cornerRadius": 20,
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "center",
        "children": [{
            "id": next_id("calendar_day_number"),
            "type": "text",
            "name": "Number",
            "content": day.to_string(),
            "fontSize": 14,
            "fontWeight": if is_selected || is_today { 600 } else { 400 },
            "fill": [{
                "type": "solid",
                "color": if is_selected { colors.selected_text } else { colors.day_text },
            }],
        }],
    });
    if is_selected {
        cell["fill"] = json!([{ "type": "solid", "color": colors.selected_bg }]);
    } else if is_today {
        cell["fill"] = json!([{ "type": "solid", "color": colors.today_bg }]);
    }
    cell
}

fn number_arg(args: &BTreeMap<String, String>, key: &str, default: f64) -> f64 {
    args.get(key)
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(default)
}

fn optional_i64_arg(args: &BTreeMap<String, String>, key: &str) -> Option<i64> {
    args.get(key)
        .and_then(|value| value.parse::<f64>().ok())
        .map(|value| value.floor() as i64)
}

fn next_id(prefix: &str) -> String {
    let n = NEXT_CALENDAR_ELEMENT_ID.fetch_add(1, Ordering::Relaxed);
    format!("__op_calendar_{prefix}_{n}")
}
