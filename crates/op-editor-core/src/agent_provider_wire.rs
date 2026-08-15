use crate::AgentProvider;

impl AgentProvider {
    /// Stable identifier used at the browser/daemon boundary.
    pub const fn wire_id(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::CodexCli => "codex-cli",
            Self::OpenCode => "open-code",
            Self::GithubCopilot => "github-copilot",
            Self::Antigravity => "antigravity",
            Self::GrokBuild => "grok-build",
            Self::DeepSeekHarness => "deepseek-harness",
        }
    }

    /// Parse a browser/daemon provider identifier.
    pub fn from_wire_id(value: &str) -> Option<Self> {
        match value.trim() {
            "claude-code" => Some(Self::ClaudeCode),
            "codex-cli" => Some(Self::CodexCli),
            "open-code" => Some(Self::OpenCode),
            "github-copilot" => Some(Self::GithubCopilot),
            "antigravity" => Some(Self::Antigravity),
            "grok-build" => Some(Self::GrokBuild),
            "deepseek-harness" => Some(Self::DeepSeekHarness),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_wire_ids_round_trip() {
        for provider in AgentProvider::ALL {
            assert_eq!(
                AgentProvider::from_wire_id(provider.wire_id()),
                Some(provider)
            );
        }
        assert_eq!(AgentProvider::from_wire_id("default"), None);
        assert_eq!(AgentProvider::from_wire_id("gemini-cli"), None);
    }
}
