//! Content/card TS element alias builders.

#![allow(clippy::result_large_err)]

use std::collections::BTreeMap;

use serde_json::{json, Value};

use crate::element_content_alias_support::{
    bool_arg, card_row_card, chat_colors, comment_colors, content_colors, event_colors,
    event_meta_row, faq_colors, icon_node, next_id, number_arg, object_array_arg, required,
    scroll_wrapper,
};
use crate::ToolOutcome;

pub(crate) fn content_alias_node_value(
    tool: &str,
    args: &BTreeMap<String, String>,
) -> Result<Option<Value>, ToolOutcome> {
    let value = match tool {
        "add_card_row_v0" => build_card_row(args, false)?,
        "add_card_row_v1" => build_card_row(args, true)?,
        "add_comment_v0" => build_comment(args, false)?,
        "add_comment_v1" => build_comment(args, true)?,
        "add_chat_bubble_v0" => build_chat_bubble(args, false)?,
        "add_chat_bubble_v1" => build_chat_bubble(args, true)?,
        "add_code_block_v0" => build_code_block(args, false)?,
        "add_code_block_v1" => build_code_block(args, true)?,
        "add_faq_item_v0" => build_faq_item(args, false)?,
        "add_faq_item_v1" => build_faq_item(args, true)?,
        "add_event_card_v0" => build_event_card(args, false)?,
        "add_event_card_v1" => build_event_card(args, true)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

fn build_card_row(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let items = object_array_arg(args, "items")?;
    let card_width = number_arg(args, "card_width", 140.0);
    let gap = number_arg(args, "gap", 12.0);
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = content_colors(theme, theme_aware);
    let cards: Vec<Value> = items
        .iter()
        .map(|item| card_row_card(item, card_width, &colors))
        .collect();
    Ok(scroll_wrapper("Card Row", gap, cards))
}

fn build_comment(args: &BTreeMap<String, String>, theme_aware: bool) -> Result<Value, ToolOutcome> {
    let author = required(args, "author")?;
    let body = required(args, "body")?;
    let avatar_size = number_arg(args, "avatar_size", 40.0).floor().max(24.0);
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = comment_colors(theme, theme_aware);

    let mut avatar_children = Vec::new();
    if let Some(initial) = args
        .get("avatar_initial")
        .filter(|initial| !initial.is_empty())
    {
        avatar_children.push(json!({
            "id": next_id("comment_avatar_initial"),
            "type": "text",
            "name": "Initial",
            "role": "avatar-initial",
            "content": initial.chars().take(2).collect::<String>().to_uppercase(),
            "fontSize": (avatar_size * 0.4).round().max(10.0),
            "fontWeight": 600,
            "fill": [{ "type": "solid", "color": colors.initial }],
        }));
    }

    let mut header_children = vec![json!({
        "id": next_id("comment_author"),
        "type": "text",
        "name": "Author",
        "role": "comment-author",
        "content": author,
        "fontSize": 14,
        "fontWeight": 600,
    })];
    if let Some(timestamp) = args
        .get("timestamp")
        .filter(|timestamp| !timestamp.is_empty())
    {
        header_children.push(json!({
            "id": next_id("comment_timestamp"),
            "type": "text",
            "name": "Timestamp",
            "role": "comment-timestamp",
            "content": timestamp,
            "fontSize": 12,
            "fontWeight": 400,
            "fill": [{ "type": "solid", "color": colors.timestamp }],
        }));
    }

    Ok(json!({
        "id": next_id("comment"),
        "type": "frame",
        "name": "Comment",
        "role": "comment",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "start",
        "gap": 12,
        "children": [
            {
                "id": next_id("comment_avatar"),
                "type": "frame",
                "name": "Avatar",
                "role": "comment-avatar",
                "width": avatar_size,
                "height": avatar_size,
                "cornerRadius": avatar_size / 2.0,
                "layout": "horizontal",
                "alignItems": "center",
                "justifyContent": "center",
                "fill": [{ "type": "solid", "color": colors.avatar_bg }],
                "children": avatar_children,
            },
            {
                "id": next_id("comment_body_column"),
                "type": "frame",
                "name": "Body Column",
                "role": "comment-body-column",
                "width": "fill_container",
                "height": "fit_content",
                "layout": "vertical",
                "gap": 4,
                "children": [
                    {
                        "id": next_id("comment_header"),
                        "type": "frame",
                        "name": "Header",
                        "role": "comment-header",
                        "width": "fit_content",
                        "height": "fit_content",
                        "layout": "horizontal",
                        "alignItems": "end",
                        "gap": 6,
                        "children": header_children,
                    },
                    {
                        "id": next_id("comment_body"),
                        "type": "text",
                        "name": "Body",
                        "role": "comment-body",
                        "content": body,
                        "fontSize": 14,
                        "fontWeight": 400,
                        "lineHeight": 1.5,
                    },
                ],
            },
        ],
    }))
}

fn build_chat_bubble(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let message = required(args, "message")?;
    let side = args.get("side").map(String::as_str).unwrap_or("left");
    let is_self = side == "right";
    let max_width = number_arg(args, "max_width", 280.0)
        .floor()
        .clamp(160.0, 480.0);
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = chat_colors(
        theme,
        theme_aware,
        args.get("accent_color").map(String::as_str),
    );
    let mut children = Vec::new();
    if !is_self {
        if let Some(author) = args.get("author").filter(|author| !author.is_empty()) {
            children.push(json!({
                "id": next_id("chat_author"),
                "type": "text",
                "name": "Author",
                "role": "chat-bubble-author",
                "content": author,
                "fontSize": 12,
                "fontWeight": 500,
                "fill": [{ "type": "solid", "color": colors.muted }],
            }));
        }
    }
    children.push(json!({
        "id": next_id("chat_surface"),
        "type": "frame",
        "name": "Surface",
        "role": "chat-bubble-surface",
        "width": max_width,
        "height": "fit_content",
        "cornerRadius": 16,
        "layout": "vertical",
        "padding": [10, 14],
        "fill": [{ "type": "solid", "color": if is_self { colors.self_bg.as_ref() } else { colors.other_bg } }],
        "children": [{
            "id": next_id("chat_message"),
            "type": "text",
            "name": "Message",
            "role": "chat-bubble-message",
            "content": message,
            "fontSize": 14,
            "fontWeight": 400,
            "lineHeight": 1.4,
            "width": "fill_container",
            "textGrowth": "fixed-width",
            "fill": [{ "type": "solid", "color": if is_self { "#FFFFFF" } else { colors.other_text } }],
        }],
    }));
    if let Some(timestamp) = args
        .get("timestamp")
        .filter(|timestamp| !timestamp.is_empty())
    {
        children.push(json!({
            "id": next_id("chat_timestamp"),
            "type": "text",
            "name": "Timestamp",
            "role": "chat-bubble-timestamp",
            "content": timestamp,
            "fontSize": 11,
            "fontWeight": 400,
            "fill": [{ "type": "solid", "color": colors.muted }],
        }));
    }

    Ok(json!({
        "id": next_id("chat_bubble"),
        "type": "frame",
        "name": "Chat Bubble",
        "role": if is_self { "chat-bubble-right" } else { "chat-bubble-left" },
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "alignItems": if is_self { "end" } else { "start" },
        "gap": 4,
        "children": children,
    }))
}

fn build_code_block(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let code = required(args, "code")?;
    let language = args.get("language").filter(|language| !language.is_empty());
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let bg = if theme_aware && theme == "system" {
        "$color-surface-2"
    } else if theme_aware && theme != "light" {
        "#334155"
    } else {
        "#F3F4F6"
    };
    Ok(json!({
        "id": next_id("code_block"),
        "type": "frame",
        "name": match language {
            Some(language) => format!("Code Block ({language})"),
            None => "Code Block".to_string(),
        },
        "role": "code-block",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "padding": [12, 16],
        "cornerRadius": 8,
        "fill": [{ "type": "solid", "color": bg }],
        "children": [{
            "id": next_id("code"),
            "type": "text",
            "name": "Code",
            "role": "code",
            "content": code,
            "fontSize": 13,
            "fontWeight": 400,
            "lineHeight": 1.5,
            "width": "fill_container",
            "textGrowth": "fixed-width",
        }],
    }))
}

fn build_faq_item(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let question = required(args, "question")?;
    let expanded = bool_arg(args, "expanded", false);
    let show_divider = bool_arg(args, "show_divider", false);
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = faq_colors(theme, theme_aware);

    let mut question_node = json!({
        "id": next_id("faq_question"),
        "type": "text",
        "name": "Question",
        "role": "faq-question",
        "content": question,
        "fontSize": 15,
        "fontWeight": 600,
        "width": "fill_container",
    });
    if let Some(question) = colors.question {
        question_node["fill"] = json!([{ "type": "solid", "color": question }]);
    }

    let mut children = vec![json!({
        "id": next_id("faq_header"),
        "type": "frame",
        "name": "Header",
        "role": "faq-header",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "space-between",
        "gap": 12,
        "children": [
            question_node,
            icon_node(
                "Chevron",
                Some(if expanded { "faq-chevron-open" } else { "faq-chevron-closed" }),
                if expanded { "chevron-down" } else { "chevron-right" },
                20.0,
                20.0,
                Some(colors.chevron),
            ),
        ],
    })];

    if expanded {
        if let Some(answer) = args.get("answer").filter(|answer| !answer.is_empty()) {
            children.push(json!({
                "id": next_id("faq_answer"),
                "type": "text",
                "name": "Answer",
                "role": "faq-answer",
                "content": answer,
                "fontSize": 14,
                "fontWeight": 400,
                "width": "fill_container",
                "fill": [{ "type": "solid", "color": colors.answer }],
            }));
        }
    }
    if show_divider {
        children.push(json!({
            "id": next_id("faq_divider"),
            "type": "rectangle",
            "name": "Divider",
            "role": "faq-divider",
            "width": "fill_container",
            "height": 1,
            "fill": [{ "type": "solid", "color": colors.divider }],
        }));
    }

    Ok(json!({
        "id": next_id("faq_item"),
        "type": "frame",
        "name": "FAQ Item",
        "role": "faq-item",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "gap": 12,
        "padding": [16, 0],
        "children": children,
    }))
}

fn build_event_card(
    args: &BTreeMap<String, String>,
    theme_aware: bool,
) -> Result<Value, ToolOutcome> {
    let month = required(args, "month")?;
    let day = required(args, "day")?;
    let title = required(args, "title")?;
    let accent = args.get("accent").map(String::as_str).unwrap_or("#2563EB");
    let theme = args.get("theme").map(String::as_str).unwrap_or("light");
    let colors = event_colors(theme, theme_aware);
    let mut stack = vec![json!({
        "id": next_id("event_title"),
        "type": "text",
        "name": "Title",
        "role": "event-card-title",
        "content": title,
        "fontSize": 15,
        "fontWeight": 600,
        "width": "fill_container",
        "textGrowth": "fixed-width",
        "fill": [{ "type": "solid", "color": colors.title }],
    })];
    if let Some(time) = args.get("time").filter(|time| !time.is_empty()) {
        stack.push(event_meta_row(
            "Time Row",
            "event-card-time-icon",
            "clock",
            "Time",
            "event-card-time",
            time,
            colors.meta,
        ));
    }
    if let Some(location) = args.get("location").filter(|location| !location.is_empty()) {
        stack.push(event_meta_row(
            "Location Row",
            "event-card-location-icon",
            "map-pin",
            "Location",
            "event-card-location",
            location,
            colors.meta,
        ));
    }

    let month_strip = json!({
        "id": next_id("event_month_strip"),
        "type": "frame",
        "name": "Month Strip",
        "role": "event-card-month-strip",
        "width": "fill_container",
        "height": "fit_content",
        "fill": [{ "type": "solid", "color": accent }],
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "center",
        "padding": [4, 0],
        "children": [{
            "id": next_id("event_month"),
            "type": "text",
            "name": "Month",
            "role": "event-card-month",
            "content": month,
            "fontSize": 11,
            "fontWeight": 700,
            "letterSpacing": 1,
            "fill": [{ "type": "solid", "color": "#FFFFFF" }],
        }],
    });
    let day_strip = json!({
        "id": next_id("event_day_strip"),
        "type": "frame",
        "name": "Day Strip",
        "role": "event-card-day-strip",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "center",
        "justifyContent": "center",
        "padding": [8, 0],
        "children": [{
            "id": next_id("event_day"),
            "type": "text",
            "name": "Day",
            "role": "event-card-day",
            "content": day,
            "fontSize": 22,
            "fontWeight": 700,
            "fill": [{ "type": "solid", "color": colors.day }],
        }],
    });
    let date_column = json!({
        "id": next_id("event_date"),
        "type": "frame",
        "name": "Date Column",
        "role": "event-card-date",
        "width": 64,
        "height": "fit_content",
        "cornerRadius": 8,
        "fill": [{ "type": "solid", "color": colors.date_bg }],
        "layout": "vertical",
        "alignItems": "center",
        "children": [month_strip, day_strip],
    });
    let text_stack = json!({
        "id": next_id("event_text"),
        "type": "frame",
        "name": "Text Stack",
        "role": "event-card-text",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "vertical",
        "gap": 6,
        "padding": [4, 0],
        "children": stack,
    });

    Ok(json!({
        "id": next_id("event_card"),
        "type": "frame",
        "name": "Event Card",
        "role": "event-card",
        "width": "fill_container",
        "height": "fit_content",
        "layout": "horizontal",
        "alignItems": "start",
        "gap": 14,
        "padding": [12, 12],
        "cornerRadius": 12,
        "fill": [{ "type": "solid", "color": colors.card_bg }],
        "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": colors.border }] },
        "children": [date_column, text_stack],
    }))
}
