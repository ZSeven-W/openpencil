//! Tiny chrome-string translation layer (Step 6).
//!
//! Stable English keys → per-locale display strings. Mirrors the
//! TS app's i18next setup at a fraction of the surface — chrome
//! has ~20 visible strings, hardcoding a per-locale table beats
//! pulling a runtime parser.
//!
//! Add a key by appending to all locale tables. Unknown keys
//! fall through to the key itself so missing translations are
//! visually obvious.

use crate::document::Locale;

/// Look up `key` in the table for `locale`. Returns `key` itself
/// (visible "missing translation" hint) when there's no entry.
/// Lifetime tied to `key` so unknown keys can be returned without
/// promoting them to `'static`.
pub fn translate<'a>(locale: Locale, key: &'a str) -> &'a str {
    let lookup = match locale {
        Locale::ZhCn => zh_cn(key),
        Locale::EnUs => en_us(key),
    };
    lookup.unwrap_or(key)
}

fn zh_cn(key: &str) -> Option<&'static str> {
    Some(match key {
        "topbar.untitled" => "未命名",
        "topbar.agent_count" => "agent",
        "layer_panel.pages" => "页面",
        "layer_panel.layers" => "图层",
        "property_panel.tab_design" => "设计",
        "property_panel.tab_code" => "代码",
        "property_panel.create_component" => "创建组件",
        "property_panel.position" => "位置",
        "property_panel.flex_layout" => "弹性布局",
        "property_panel.size" => "尺寸",
        "property_panel.fill_width" => "填充宽度",
        "property_panel.fill_height" => "填充高度",
        "property_panel.fit_width" => "适应宽度",
        "property_panel.fit_height" => "适应高度",
        "property_panel.clip_content" => "裁剪内容",
        "property_panel.layer" => "图层",
        "property_panel.opacity" => "不透明度",
        "property_panel.fill" => "填充",
        "property_panel.solid" => "纯色",
        "property_panel.stroke" => "描边",
        "property_panel.effects" => "效果",
        "property_panel.export" => "导出",
        "chat.new_chat" => "New Chat",
        "chat.start_with_ai" => "用 AI 开始设计",
        "chat.input_placeholder" => "用 Agent 设计…",
        _ => return None,
    })
}

fn en_us(key: &str) -> Option<&'static str> {
    Some(match key {
        "topbar.untitled" => "Untitled",
        "topbar.agent_count" => "agent",
        "layer_panel.pages" => "Pages",
        "layer_panel.layers" => "Layers",
        "property_panel.tab_design" => "Design",
        "property_panel.tab_code" => "Code",
        "property_panel.create_component" => "Create component",
        "property_panel.position" => "Position",
        "property_panel.flex_layout" => "Layout",
        "property_panel.size" => "Size",
        "property_panel.fill_width" => "Fill width",
        "property_panel.fill_height" => "Fill height",
        "property_panel.fit_width" => "Fit width",
        "property_panel.fit_height" => "Fit height",
        "property_panel.clip_content" => "Clip content",
        "property_panel.layer" => "Layer",
        "property_panel.opacity" => "Opacity",
        "property_panel.fill" => "Fill",
        "property_panel.solid" => "Solid",
        "property_panel.stroke" => "Stroke",
        "property_panel.effects" => "Effects",
        "property_panel.export" => "Export",
        "chat.new_chat" => "New Chat",
        "chat.start_with_ai" => "Start designing with AI",
        "chat.input_placeholder" => "Design with Agent…",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zh_cn_returns_chinese_chrome_strings() {
        assert_eq!(translate(Locale::ZhCn, "layer_panel.pages"), "页面");
        assert_eq!(translate(Locale::ZhCn, "property_panel.position"), "位置");
    }

    #[test]
    fn en_us_returns_english_chrome_strings() {
        assert_eq!(translate(Locale::EnUs, "layer_panel.pages"), "Pages");
        assert_eq!(translate(Locale::EnUs, "property_panel.position"), "Position");
    }

    #[test]
    fn unknown_key_falls_through_to_key() {
        assert_eq!(
            translate(Locale::ZhCn, "this.key.does.not.exist"),
            "this.key.does.not.exist"
        );
    }
}
