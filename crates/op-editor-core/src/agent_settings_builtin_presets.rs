//! Built-in API-key provider presets, mirrored from the TS settings UI.

use crate::agent_settings::BuiltinAgentKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinAgentPresetKey {
    Anthropic,
    OpenAi,
    OpenRouter,
    DeepSeek,
    Gemini,
    MiniMax,
    Zhipu,
    GlmCoding,
    Kimi,
    Bailian,
    BailianCoding,
    Doubao,
    ArkCoding,
    Xiaomi,
    ModelScope,
    StepFun,
    StepFunCoding,
    Nvidia,
    Custom,
}

impl BuiltinAgentPresetKey {
    pub const fn as_str(self) -> &'static str {
        match self {
            BuiltinAgentPresetKey::Anthropic => "anthropic",
            BuiltinAgentPresetKey::OpenAi => "openai",
            BuiltinAgentPresetKey::OpenRouter => "openrouter",
            BuiltinAgentPresetKey::DeepSeek => "deepseek",
            BuiltinAgentPresetKey::Gemini => "gemini",
            BuiltinAgentPresetKey::MiniMax => "minimax",
            BuiltinAgentPresetKey::Zhipu => "zhipu",
            BuiltinAgentPresetKey::GlmCoding => "glm-coding",
            BuiltinAgentPresetKey::Kimi => "kimi",
            BuiltinAgentPresetKey::Bailian => "bailian",
            BuiltinAgentPresetKey::BailianCoding => "bailian-coding",
            BuiltinAgentPresetKey::Doubao => "doubao",
            BuiltinAgentPresetKey::ArkCoding => "ark-coding",
            BuiltinAgentPresetKey::Xiaomi => "xiaomi",
            BuiltinAgentPresetKey::ModelScope => "modelscope",
            BuiltinAgentPresetKey::StepFun => "stepfun",
            BuiltinAgentPresetKey::StepFunCoding => "stepfun-coding",
            BuiltinAgentPresetKey::Nvidia => "nvidia",
            BuiltinAgentPresetKey::Custom => "custom",
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(value: &str) -> Option<Self> {
        Some(match value {
            "anthropic" => BuiltinAgentPresetKey::Anthropic,
            "openai" => BuiltinAgentPresetKey::OpenAi,
            "openrouter" => BuiltinAgentPresetKey::OpenRouter,
            "deepseek" => BuiltinAgentPresetKey::DeepSeek,
            "gemini" => BuiltinAgentPresetKey::Gemini,
            "minimax" => BuiltinAgentPresetKey::MiniMax,
            "zhipu" => BuiltinAgentPresetKey::Zhipu,
            "glm-coding" => BuiltinAgentPresetKey::GlmCoding,
            "kimi" => BuiltinAgentPresetKey::Kimi,
            "bailian" => BuiltinAgentPresetKey::Bailian,
            "bailian-coding" => BuiltinAgentPresetKey::BailianCoding,
            "doubao" => BuiltinAgentPresetKey::Doubao,
            "ark-coding" => BuiltinAgentPresetKey::ArkCoding,
            "xiaomi" => BuiltinAgentPresetKey::Xiaomi,
            "modelscope" => BuiltinAgentPresetKey::ModelScope,
            "stepfun" => BuiltinAgentPresetKey::StepFun,
            "stepfun-coding" => BuiltinAgentPresetKey::StepFunCoding,
            "nvidia" => BuiltinAgentPresetKey::Nvidia,
            "custom" => BuiltinAgentPresetKey::Custom,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltinAgentPreset {
    pub key: BuiltinAgentPresetKey,
    pub display_name: &'static str,
    pub kind: BuiltinAgentKind,
    pub model: &'static str,
    pub base_url: &'static str,
    pub alt_kind: Option<BuiltinAgentKind>,
    pub alt_base_url: Option<&'static str>,
}

impl BuiltinAgentPreset {
    pub fn base_url_for_kind(self, kind: BuiltinAgentKind) -> Option<&'static str> {
        if kind == self.kind {
            Some(self.base_url)
        } else if Some(kind) == self.alt_kind {
            self.alt_base_url
        } else {
            None
        }
    }
}

pub const BUILTIN_AGENT_PRESETS: [BuiltinAgentPreset; 19] = [
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::Anthropic,
        display_name: "Anthropic",
        kind: BuiltinAgentKind::Anthropic,
        model: "claude-sonnet-4-6-20250916",
        base_url: "https://api.anthropic.com",
        alt_kind: None,
        alt_base_url: None,
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::OpenAi,
        display_name: "OpenAI",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "gpt-5.4",
        base_url: "https://api.openai.com/v1",
        alt_kind: None,
        alt_base_url: None,
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::OpenRouter,
        display_name: "OpenRouter",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "anthropic/claude-sonnet-4.6",
        base_url: "https://openrouter.ai/api/v1",
        alt_kind: Some(BuiltinAgentKind::Anthropic),
        alt_base_url: Some("https://openrouter.ai/api"),
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::DeepSeek,
        display_name: "DeepSeek",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "deepseek-v4-pro",
        base_url: "https://api.deepseek.com/v1",
        alt_kind: Some(BuiltinAgentKind::Anthropic),
        alt_base_url: Some("https://api.deepseek.com/anthropic"),
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::Gemini,
        display_name: "Google Gemini",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "gemini-3-flash-preview",
        base_url: "https://generativelanguage.googleapis.com/v1beta/openai",
        alt_kind: None,
        alt_base_url: None,
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::MiniMax,
        display_name: "MiniMax",
        kind: BuiltinAgentKind::Anthropic,
        model: "MiniMax-M2.7",
        base_url: "https://api.minimaxi.com/anthropic",
        alt_kind: Some(BuiltinAgentKind::OpenAiCompat),
        alt_base_url: Some("https://api.minimaxi.com/v1"),
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::Zhipu,
        display_name: "智谱 (Zhipu)",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "glm-5",
        base_url: "https://open.bigmodel.cn/api/paas/v4",
        alt_kind: Some(BuiltinAgentKind::Anthropic),
        alt_base_url: Some("https://open.bigmodel.cn/api/anthropic"),
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::GlmCoding,
        display_name: "GLM Coding Plan",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "glm-4.7",
        base_url: "https://open.bigmodel.cn/api/coding/paas/v4",
        alt_kind: Some(BuiltinAgentKind::Anthropic),
        alt_base_url: Some("https://open.bigmodel.cn/api/anthropic"),
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::Kimi,
        display_name: "Kimi (Moonshot)",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "kimi-k2.5",
        base_url: "https://api.moonshot.cn/v1",
        alt_kind: Some(BuiltinAgentKind::Anthropic),
        alt_base_url: Some("https://api.moonshot.cn/anthropic"),
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::Bailian,
        display_name: "Bailian (DashScope)",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "qwen-plus",
        base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        alt_kind: Some(BuiltinAgentKind::Anthropic),
        alt_base_url: Some("https://dashscope.aliyuncs.com/apps/anthropic"),
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::BailianCoding,
        display_name: "Bailian Coding Plan",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "qwen3-coder-plus",
        base_url: "https://coding.dashscope.aliyuncs.com/v1",
        alt_kind: Some(BuiltinAgentKind::Anthropic),
        alt_base_url: Some("https://coding.dashscope.aliyuncs.com/apps/anthropic"),
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::Doubao,
        display_name: "DouBao Seed",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "doubao-seed-2.0-pro",
        base_url: "https://ark.cn-beijing.volces.com/api/v3",
        alt_kind: Some(BuiltinAgentKind::Anthropic),
        alt_base_url: Some("https://ark.cn-beijing.volces.com/api/coding"),
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::ArkCoding,
        display_name: "Ark Coding Plan",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "ark-code-latest",
        base_url: "https://ark.cn-beijing.volces.com/api/coding/v3",
        alt_kind: Some(BuiltinAgentKind::Anthropic),
        alt_base_url: Some("https://ark.cn-beijing.volces.com/api/coding"),
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::Xiaomi,
        display_name: "Xiaomi MiMo",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "mimo-v2-pro",
        base_url: "https://api.xiaomimimo.com/v1",
        alt_kind: None,
        alt_base_url: None,
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::ModelScope,
        display_name: "ModelScope",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "qwen-plus",
        base_url: "https://api-inference.modelscope.cn/v1",
        alt_kind: Some(BuiltinAgentKind::Anthropic),
        alt_base_url: Some("https://api-inference.modelscope.cn"),
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::StepFun,
        display_name: "StepFun",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "step-3.5-flash",
        base_url: "https://api.stepfun.com/v1",
        alt_kind: None,
        alt_base_url: None,
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::StepFunCoding,
        display_name: "StepFun Coding Plan",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "step-3-coding",
        base_url: "https://api.stepfun.com/step_plan/v1",
        alt_kind: None,
        alt_base_url: None,
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::Nvidia,
        display_name: "NVIDIA NIM",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "nvidia/llama-3.1-nemotron-70b-instruct",
        base_url: "https://integrate.api.nvidia.com/v1",
        alt_kind: None,
        alt_base_url: None,
    },
    BuiltinAgentPreset {
        key: BuiltinAgentPresetKey::Custom,
        display_name: "Custom",
        kind: BuiltinAgentKind::OpenAiCompat,
        model: "model-name",
        base_url: "",
        alt_kind: Some(BuiltinAgentKind::Anthropic),
        alt_base_url: None,
    },
];

pub fn builtin_agent_preset(key: BuiltinAgentPresetKey) -> BuiltinAgentPreset {
    BUILTIN_AGENT_PRESETS
        .iter()
        .copied()
        .find(|preset| preset.key == key)
        .unwrap_or(BUILTIN_AGENT_PRESETS[0])
}

pub fn infer_builtin_agent_preset(kind: BuiltinAgentKind, base_url: &str) -> BuiltinAgentPresetKey {
    let normalized = base_url.trim().trim_end_matches('/');
    BUILTIN_AGENT_PRESETS
        .iter()
        .find(|preset| {
            preset.base_url.trim_end_matches('/') == normalized
                || preset
                    .alt_base_url
                    .is_some_and(|url| url.trim_end_matches('/') == normalized)
        })
        .map(|preset| preset.key)
        .unwrap_or(match kind {
            BuiltinAgentKind::Anthropic => BuiltinAgentPresetKey::Anthropic,
            BuiltinAgentKind::OpenAiCompat => BuiltinAgentPresetKey::Custom,
        })
}
