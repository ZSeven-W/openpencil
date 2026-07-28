//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `zh_cn_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "搜索图片…",
        "imagePanel.searching" => "搜索中…",
        "imagePanel.noResults" => "未找到结果",
        "imagePanel.searchPrompt" => "搜索图片",
        "imagePanel.sourceNotice" => "图片来自 {{source}}。自由许可 — 使用前请核实许可协议。",
        "imagePanel.genNotConfigured" => "图片生成未配置",
        "imagePanel.openSettings" => "打开设置",
        "imagePanel.promptPlaceholder" => "描述要生成的图片…",
        "providerProbe.connectedViaCli" => "已通过 {{name}} CLI 连接",
        "providerProbe.cliExitedWithError" => "{{name}} CLI 退出并报错",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI 未输出版本信息",
        "providerProbe.modelQueryFailed" => "{{name}} 模型查询失败或超时",
        "providerProbe.modelQueryFailedRunLogin" => {
            "{{name}} 模型查询失败。请先运行 {{command}} 完成认证。"
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "{{name}} 模型查询需要认证。请先运行 {{command}} 登录。"
        }
        "providerProbe.unrecognizedModelCatalog" => "{{name}} 返回了无法识别的模型列表",
        "providerProbe.connectedAs" => "已以 @{{login}}{{method}} 身份连接",
        "providerProbe.connectedViaGithub" => "已通过 GitHub 连接",
        "importProgress.figmaTitle" => "正在解析 Figma 文件…",
        "importProgress.htmlTitle" => "正在解析 HTML 和页面资源…",
        "importProgress.htmlSubtitle" => "正在读取样式和图片，请稍候",
        "importProgress.largeFileSubtitle" => "大型文件需要几秒钟，请稍候",
        "account.signedOutHint" => "登录后即可同步你的设置与偏好",
        "code.noUsableCode" => "AI 未返回可用代码。请重试，或切换 AI 模型后再试。",
        "code.previousResultKept" => "上次生成的代码仍已保留",
        _ => return super::zh_cn_collab::lookup(key),
    })
}
