//! Re-export shim: the quality-credential renderer moved to `op-chat-agent`
//! (pure code motion); existing paths stay valid.

pub use op_chat_agent::quality_credential::*;

/// A brief receipt for Chinese editor chrome. Detailed diagnostics remain on
/// the expandable progress row; a check does not imply zero remaining issues.
pub fn quality_credential_line_for_locale(
    quality: &op_ai::chat_provider::QualitySummary,
    locale: op_editor_core::Locale,
) -> Option<String> {
    if !matches!(
        locale,
        op_editor_core::Locale::ZhCn | op_editor_core::Locale::ZhTw
    ) {
        return quality_credential_line(quality, None);
    }
    quality.ran().then(|| {
        format!(
            "\n\n• {}",
            op_i18n::translate_with(
                locale,
                "ai.designProgress.narration.qualityChecked",
                &[("count", &quality.total_repairs().to_string())],
            )
        )
    })
}
