//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `de_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Bilder suchen…",
        "imagePanel.searching" => "Suche läuft…",
        "imagePanel.noResults" => "Keine Ergebnisse",
        "imagePanel.searchPrompt" => "Nach Bildern suchen",
        "imagePanel.sourceNotice" => {
            "Bilder von {{source}}. Frei lizenziert — Lizenz vor Verwendung prüfen."
        }
        "imagePanel.genNotConfigured" => "Bildgenerierung nicht konfiguriert",
        "imagePanel.openSettings" => "Einstellungen öffnen",
        "imagePanel.promptPlaceholder" => "Beschreibe das Bild…",
        "providerProbe.connectedViaCli" => "Über {{name}}-CLI verbunden",
        "providerProbe.cliExitedWithError" => "{{name}}-CLI wurde mit einem Fehler beendet",
        "providerProbe.cliNoVersionOutput" => "{{name}}-CLI hat keine Versionsausgabe geliefert",
        "providerProbe.modelQueryFailed" => "Modellabfrage für {{name}} fehlgeschlagen oder abgelaufen",
        "providerProbe.modelQueryFailedRunLogin" => "Modellabfrage für {{name}} fehlgeschlagen. Führe {{command}} einmal aus, um dich zu authentifizieren.",
        "providerProbe.modelQueryNeedsAuth" => "Die Modellabfrage für {{name}} erfordert eine Authentifizierung. Führe {{command}} einmal aus, um dich anzumelden.",
        "providerProbe.unrecognizedModelCatalog" => "{{name}} hat einen unbekannten Modellkatalog zurückgegeben",
        _ => return super::de_collab::lookup(key),
    })
}
