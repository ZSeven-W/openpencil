//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `pt_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Pesquisar imagens…",
        "imagePanel.searching" => "Pesquisando…",
        "imagePanel.noResults" => "Nenhum resultado",
        "imagePanel.searchPrompt" => "Pesquise imagens",
        "imagePanel.sourceNotice" => {
            "Imagens de {{source}}. Licença livre — verifique a licença antes de usar."
        }
        "imagePanel.genNotConfigured" => "Geração de imagens não configurada",
        "imagePanel.openSettings" => "Abrir configurações",
        "imagePanel.promptPlaceholder" => "Descreva a imagem…",
        "providerProbe.connectedViaCli" => "Conectado via CLI do {{name}}",
        "providerProbe.cliExitedWithError" => "A CLI do {{name}} terminou com erro",
        "providerProbe.cliNoVersionOutput" => "A CLI do {{name}} não produziu informação de versão",
        "providerProbe.modelQueryFailed" => "A consulta de modelos do {{name}} falhou ou expirou",
        "providerProbe.modelQueryFailedRunLogin" => "A consulta de modelos do {{name}} falhou. Execute {{command}} uma vez para autenticar.",
        "providerProbe.modelQueryNeedsAuth" => "A consulta de modelos do {{name}} exige autenticação. Execute {{command}} uma vez para entrar.",
        "providerProbe.unrecognizedModelCatalog" => "{{name}} devolveu um catálogo de modelos não reconhecido",
        _ => return super::pt_collab::lookup(key),
    })
}
