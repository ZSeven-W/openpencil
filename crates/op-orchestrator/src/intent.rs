//! 意图分类 —— design vs chat 路由。
//!
//! 行为忠实 TS `ai-service.ts` 的路由:有明确设计意图 → Design,
//! 否则 Chat。S3a 用轻量关键词启发式(可单测);LLM 分类留后续。

use crate::types::Intent;

/// 命中任一即判为设计意图。中英双语,覆盖动词 + 设计名词。
const DESIGN_KEYWORDS: &[&str] = &[
    // 英文动词
    "design", "create", "build", "make a", "make me", "generate", "draw",
    "mock up", "mockup", "wireframe", "prototype", "lay out",
    // 英文设计名词
    "landing page", "dashboard", "pricing card", "login page", "sign up",
    "ui for", "screen for", "app screen", "web page",
    // 中文动词
    "设计", "生成", "做一个", "做个", "创建", "画一个", "画个", "搞一个",
    "帮我做", "来个",
    // 中文设计名词
    "页面", "界面", "仪表盘", "落地页", "原型", "登录页", "注册页",
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
