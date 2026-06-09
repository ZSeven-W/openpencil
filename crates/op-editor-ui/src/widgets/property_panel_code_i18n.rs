use op_editor_core::codegen::Framework;
use op_i18n::Locale;

#[derive(Debug, Clone, Copy)]
pub(super) struct CodePanelStrings {
    locale: Locale,
}

impl CodePanelStrings {
    pub(super) fn new(locale: Locale) -> Self {
        Self { locale }
    }

    pub(super) fn title(self) -> &'static str {
        self.t("rightPanel.code")
    }

    pub(super) fn idle_subtitle(self) -> &'static str {
        self.t("code.productionReady")
    }

    pub(super) fn selected_nodes(self, count: usize) -> String {
        self.t("code.selectedNodes")
            .replace("{{count}}", &count.to_string())
    }

    pub(super) fn generate_framework(self, framework: Framework) -> String {
        self.t("code.generateFramework")
            .replace("{{framework}}", framework.display_name())
    }

    pub(super) fn generating_framework(self, framework: Framework) -> String {
        self.t("code.generatingFramework")
            .replace("{{framework}}", framework.display_name())
    }

    pub(super) fn export_ai_bundle(self) -> &'static str {
        self.t("code.exportAiBundle")
    }

    pub(super) fn generation_failed(self) -> &'static str {
        self.t("code.generationFailed")
    }

    pub(super) fn regenerate(self) -> &'static str {
        self.t("code.regenerate")
    }

    pub(super) fn generated_degraded(self) -> &'static str {
        self.t("code.generatedDegraded")
    }

    pub(super) fn includes_assets(self, count: usize) -> String {
        self.t("code.includesAssets")
            .replace("{{count}}", &count.to_string())
    }

    pub(super) fn copy(self) -> &'static str {
        self.t("code.copyAction")
    }

    pub(super) fn save(self) -> &'static str {
        self.t("common.save")
    }

    pub(super) fn bundle(self) -> &'static str {
        self.t("code.bundleAction")
    }

    pub(super) fn regen(self) -> &'static str {
        self.t("code.regenAction")
    }

    pub(super) fn preparing_production(self) -> &'static str {
        self.t("code.preparingProduction")
    }

    pub(super) fn planning(self) -> &'static str {
        self.t("code.planning")
    }

    pub(super) fn assembly(self) -> &'static str {
        self.t("code.assembly")
    }

    pub(super) fn cancel(self) -> &'static str {
        self.t("common.cancel")
    }

    pub(super) fn waiting(self) -> &'static str {
        self.t("code.waiting")
    }

    pub(super) fn running(self) -> &'static str {
        self.t("code.running")
    }

    pub(super) fn done(self) -> &'static str {
        self.t("code.done")
    }

    pub(super) fn issue(self) -> &'static str {
        self.t("code.issue")
    }

    pub(super) fn skipped(self) -> &'static str {
        self.t("code.skipped")
    }

    fn t(self, key: &'static str) -> &'static str {
        op_i18n::translate(self.locale, key)
    }
}
