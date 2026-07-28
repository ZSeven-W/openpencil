//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `es_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Buscar imágenes…",
        "imagePanel.searching" => "Buscando…",
        "imagePanel.noResults" => "Sin resultados",
        "imagePanel.searchPrompt" => "Busca imágenes",
        "imagePanel.sourceNotice" => {
            "Imágenes de {{source}}. Licencia libre — verifica la licencia antes de usar."
        }
        "imagePanel.genNotConfigured" => "La generación de imágenes no está configurada",
        "imagePanel.openSettings" => "Abrir ajustes",
        "imagePanel.promptPlaceholder" => "Describe la imagen…",
        "providerProbe.connectedViaCli" => "Conectado a través de la CLI de {{name}}",
        "providerProbe.cliExitedWithError" => "La CLI de {{name}} finalizó con un error",
        "providerProbe.cliNoVersionOutput" => "La CLI de {{name}} no devolvió información de versión",
        "providerProbe.modelQueryFailed" => "La consulta de modelos de {{name}} falló o superó el tiempo de espera",
        "providerProbe.modelQueryFailedRunLogin" => "La consulta de modelos de {{name}} falló. Ejecuta {{command}} una vez para autenticarte.",
        "providerProbe.modelQueryNeedsAuth" => "La consulta de modelos de {{name}} requiere autenticación. Ejecuta {{command}} una vez para iniciar sesión.",
        "providerProbe.unrecognizedModelCatalog" => "{{name}} devolvió un catálogo de modelos no reconocido",
        _ => return super::es_collab::lookup(key),
    })
}
