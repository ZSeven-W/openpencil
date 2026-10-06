//! 意图分类 —— design vs chat 路由。
//!
//! 行为忠实 TS `ai-service.ts` 的路由:有明确设计意图 → Design,
//! 否则 Chat。S3a 用轻量关键词启发式(可单测);LLM 分类留后续。

use crate::types::Intent;

/// 命中任一即判为设计意图。中英双语,覆盖动词 + 设计名词。
const DESIGN_KEYWORDS: &[&str] = &[
    // 英文动词
    "design",
    "create",
    "build",
    "make a",
    "make me",
    "generate",
    "draw",
    "mock up",
    "mockup",
    "wireframe",
    "prototype",
    "lay out",
    // 英文设计名词
    "landing page",
    "dashboard",
    "pricing card",
    "login page",
    "sign up",
    "ui for",
    "screen for",
    "app screen",
    "web page",
    // 英文设计名词 — 产品/应用类（名词短语请求，无创建动词，如
    // "Luxury webapp for managing barbershop clients"）。设计工具语境下
    // 这些词几乎总是"造一个 X"，误判成 chat 的代价（弱 chat-agent-loop
    // 只探索不设计）远高于偶发误判成 design 的代价。
    "webapp",
    "web app",
    "website",
    "web site",
    "app for",
    "mobile app",
    "admin panel",
    "saas",
    // 中文动词
    "设计",
    "生成",
    "做一个",
    "做个",
    "创建",
    "画一个",
    "画个",
    "画出",
    "搞一个",
    "帮我做",
    "来个",
    // 中文设计名词
    "页面",
    "界面",
    "仪表盘",
    "落地页",
    "原型",
    "登录页",
    "注册页",
    "发现页",
    "订单页",
    "我的页",
    "个人页",
    "搜索页",
    "详情页",
    "分类页",
    "网站",
    "网页",
    "小程序",
    "后台",
    // 中文动词 —— 量词形式。Studio 首页的七类任务包装语都用"做一份/
    // 一套/一张/一篇"起头，而旧表只有"做一个/做个"，于是演示文稿、图文
    // 卡片、截图教程、信息图、活动海报五类全部落到 chat。
    "做一份",
    "做一套",
    "做一张",
    "做一篇",
    "做份",
    "做套",
    "做张",
    // 中文设计名词 —— 交付物形态。与上面的页面类名词同理：在设计工具
    // 语境里这些词几乎总是"造一个 X"。
    "演示文稿",
    "幻灯片",
    "ppt",
    "slides",
    "图文卡片",
    "卡片",
    "海报",
    "信息图",
    "长图",
    "截图教程",
    "排版",
];

/// 把用户消息分类为 [`Intent::Design`] 或 [`Intent::Chat`]。
///
/// 命中任一设计关键词即 Design;否则 Chat。大小写不敏感。
pub fn classify_intent(message: &str) -> Intent {
    let lower = message.to_lowercase();
    if DESIGN_KEYWORDS.iter().any(|k| lower.contains(k)) {
        Intent::Design
    } else {
        Intent::Chat
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn design_verbs_classify_as_design() {
        for p in [
            "design a login page",
            "生成一个仪表盘",
            "做一个落地页",
            "create a pricing card",
            "Generate a dashboard for sales",
            "帮我做个注册页",
            "继续画出发现页",
            "继续做订单页",
            // 名词短语（无创建动词）— 产品/应用类请求
            "Luxury webapp for managing barbershop clients",
            "a SaaS website for invoicing",
            "mobile app for habit tracking",
            "admin panel for orders",
            "一个理发店管理网站",
            "记账小程序",
        ] {
            assert_eq!(classify_intent(p), Intent::Design, "{p}");
        }
    }

    /// Every Studio Home task wrapper must classify as Design. The
    /// deliverable is already chosen by the task card, so a wrapper that
    /// reads as chat silently answers the user in prose instead of
    /// drawing (measured 2026-09-13 on 演示文稿).
    #[test]
    fn every_studio_home_task_wrapper_classifies_as_design() {
        for p in [
            "请设计一套可编辑的高保真手机 App 界面（mobile app，375×812）。",
            "请设计一个完整的纵向滚动网站页面（landing page，1440 宽）。",
            "请做一份 5 页的 PPT 演示文稿（slides，16:9）。封面、正文与结束页风格统一。",
            "请做一套图文卡片（card，竖版 3:4，多页轮播）。保持统一排版与视觉系统。",
            "请做一篇截图教程图文（card，竖版，截图配上步骤说明）。",
            "请做一张数据对比信息图长图（card，竖版图文长图）。",
            "请做一套活动海报（card，主海报竖版 + 社交方图）。",
        ] {
            assert_eq!(classify_intent(p), Intent::Design, "{p}");
        }
    }

    #[test]
    fn questions_classify_as_chat() {
        for p in [
            "what is a frame?",
            "这个怎么用",
            "解释一下布局",
            "thanks, that looks good",
        ] {
            assert_eq!(classify_intent(p), Intent::Chat, "{p}");
        }
    }
}
