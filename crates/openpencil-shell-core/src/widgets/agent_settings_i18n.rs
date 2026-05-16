//! Hand-maintained EN/ZH lookup for the settings modal.
//!
//! The repo's main locale tables (`crates/op-i18n/src/i18n/{en,zh_cn,…}.rs`)
//! are auto-generated from `apps/web/src/i18n/locales/*.ts` via
//! `tools/convert-locales.py` and carry a "do NOT hand-edit"
//! header. The settings modal strings haven't been added to the
//! TS source yet, so injecting them here keeps the chrome
//! translatable without touching generated files. When the TS
//! locale tables grow `settings.*` keys, this file can collapse
//! into per-locale `lookup` calls.

use op_editor_core::editor_ui_state::{EditorUiState, Locale};

/// Pick the English or simplified-Chinese variant of `key` for the
/// editor's active locale; locales other than zh_cn/zh_tw fall
/// back to English. Unknown keys return `key` itself so missing
/// strings stay visible in dev builds.
pub fn t(ui: &EditorUiState, key: &'static str) -> &'static str {
    let (en, zh) = match key {
        "settings.title" => ("Settings", "设置"),
        "settings.tab.agents" => ("Agents", "Agents"),
        "settings.tab.mcp" => ("MCP", "MCP"),
        "settings.tab.images" => ("Images", "Images"),
        "settings.tab.system" => ("System", "系统"),
        "settings.agents.builtin" => ("Built-in providers", "内置服务商"),
        "settings.agents.builtinSubtitle" => (
            "Configure API keys directly — no CLI tools needed.",
            "直接配置 API 密钥 — 无需 CLI 工具。",
        ),
        "settings.agents.builtinEmpty" => (
            "No built-in providers configured.",
            "尚未配置内置服务商。",
        ),
        "settings.agents.addProvider" => ("+ Add provider", "+ 添加服务商"),
        "settings.agents.acp" => ("ACP Agent", "ACP Agent"),
        "settings.agents.acpSubtitle" => (
            "Connect external ACP-compatible Agents.",
            "连接外部 ACP 兼容的 Agent。",
        ),
        "settings.agents.acpEmpty" => ("No ACP Agents configured.", "尚未配置 ACP Agent。"),
        "settings.agents.addAcp" => ("+ Add Agent", "+ 添加 Agent"),
        "settings.agents.title" => ("Agents", "Agents"),
        "settings.agents.connect" => ("Connect", "连接"),
        "settings.agents.disconnect" => ("Disconnect", "断开连接"),
        "settings.agents.claudeHint" => (
            "You can set additional environment variables in ~/.claude/settings.json.",
            "你可以在 ~/.claude/settings.json 中设置额外的环境变量。",
        ),
        "settings.mcp.server" => ("MCP Server", "MCP 服务器"),
        "settings.mcp.running" => ("Running", "运行中"),
        "settings.mcp.stopped" => ("Stopped", "已停止"),
        "settings.mcp.port" => ("Port", "端口"),
        "settings.mcp.start" => ("Start", "启动"),
        "settings.mcp.stop" => ("Stop", "停止"),
        "settings.mcp.terminalIntegrations" => (
            "Terminal MCP integrations",
            "终端中的 MCP 集成",
        ),
        "settings.mcp.terminalSubtitle1" => (
            "MCP integrations take effect after restarting the terminal.",
            "MCP 集成将在重启终端后生效。",
        ),
        "settings.mcp.terminalSubtitle2" => (
            "After upgrading OpenPencil, please reinstall MCP integrations for compatibility.",
            "升级 OpenPencil 版本后，请重新安装 MCP 集成以确保兼容性。",
        ),
        "settings.images.search" => ("Image Search", "图片搜索"),
        "settings.images.ready" => ("Ready", "已就绪"),
        "settings.images.notConfigured" => ("Not configured", "尚未配置"),
        "settings.images.advanced" => ("Advanced", "高级"),
        "settings.images.oauthLabel" => (
            "Openverse OAuth (optional, for higher rate limits)",
            "Openverse OAuth（可选，用于提升速率限制）",
        ),
        "settings.images.clientId" => ("Client ID", "客户端 ID"),
        "settings.images.clientSecret" => ("Client Secret", "客户端密钥"),
        "settings.images.clientIdPlaceholder" => ("your-client-id", "your-client-id"),
        "settings.images.clientSecretPlaceholder" => ("your-client-secret", "your-client-secret"),
        "settings.images.registerLink" => ("Register at Openverse", "在 Openverse 注册"),
        "settings.images.test" => ("Test", "测试"),
        "settings.images.generation" => ("Image Generation", "图片生成"),
        "settings.images.add" => ("+ Add", "+ 添加"),
        "settings.images.empty" => (
            "No configurations yet. Click \"Add\" to create one.",
            "尚无配置。点击\"添加\"创建一个。",
        ),
        "settings.provider.claudeCode" => ("Claude model", "Claude 模型"),
        "settings.provider.codexCli" => ("OpenAI models", "OpenAI 模型"),
        "settings.provider.openCode" => ("75+ LLM providers", "75+ LLM 提供商"),
        "settings.provider.githubCopilot" => ("GitHub Copilot models", "GitHub Copilot 模型"),
        "settings.provider.geminiCli" => ("Google Gemini models", "Google Gemini 模型"),
        "settings.system.title" => ("System", "系统"),
        "settings.system.autoUpdate" => ("Auto-update", "自动更新"),
        "settings.system.autoUpdateUnavailable" => (
            "Not yet wired in this build — please install updates manually.",
            "当前版本尚未接入自动更新，请手动安装更新。",
        ),
        "settings.system.upToDate" => ("Up to date", "已是最新"),
        "settings.system.upToDateDescription" => (
            "Running the latest installed build. No update channel is wired in this build — the next release ships through your package manager / .dmg / .exe.",
            "当前已是最新已安装版本。本版本未接入自动更新通道，下一次更新请通过你的包管理器或安装包获取。",
        ),
        "settings.system.checkForUpdates" => ("Check for updates", "检查更新"),
        _ => return key,
    };
    match ui.locale {
        Locale::ZhCn | Locale::ZhTw => zh,
        _ => en,
    }
}
