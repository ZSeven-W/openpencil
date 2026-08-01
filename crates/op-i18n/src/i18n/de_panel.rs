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
        "promptCenter.title" => "Prompt-Bibliothek",
        "promptCenter.searchPlaceholder" => "Prompts durchsuchen…",
        "promptCenter.category.all" => "Alle",
        "promptCenter.category.starter" => "Schnellstart",
        "promptCenter.category.mobileApp" => "Mobile App",
        "promptCenter.category.webPage" => "Webseite",
        "promptCenter.category.dashboard" => "Dashboard",
        "promptCenter.category.component" => "Komponente",
        "promptCenter.category.modify" => "Überarbeiten",
        "promptCenter.category.custom" => "Meine",
        "promptCenter.empty" => "Keine passenden Prompts gefunden",
        "promptCenter.saveCurrent" => "Aktuelle Eingabe als Prompt speichern",
        "promptCenter.saveTitlePlaceholder" => "Titel des Prompts",
        "promptCenter.save" => "Speichern",
        "promptCenter.cancel" => "Abbrechen",
        "promptCenter.delete" => "Löschen",
        "promptCenter.screens" => "{{count}} Screens",
        "promptCenter.freeform" => "Freie Form",
        "promptCenter.item.wander.title" => "Wander · Reiseplanung",
        "promptCenter.item.forage.title" => "Forage · Saisonale Rezepte",
        "promptCenter.item.still.title" => "Still · Meditation und Einschlafen",
        "promptCenter.item.hearth.title" => "Hearth · Smart Home",
        "promptCenter.item.meteo.title" => "Meteo · Immersives Wetter",
        "promptCenter.item.marginalia.title" => "Marginalia · Lesen und Anmerkungen",
        "promptCenter.item.lingua.title" => "Lingua · Sprachen lernen",
        "promptCenter.item.daybreak.title" => "Daybreak · Kaffee bestellen",
        "promptCenter.item.verdant.title" => "Verdant · Pflanzenpflege",
        "promptCenter.item.companion.title" => "Companion · Leben mit Haustieren",
        "promptCenter.item.relic.title" => "Relic · Kuratierter Secondhand-Markt",
        "promptCenter.item.nocturne.title" => "Nocturne · Sternbeobachtung",
        "promptCenter.item.marquee.title" => "Marquee · Film-Merkliste",
        "promptCenter.item.ritual.title" => "Ritual · Gewohnheiten aufbauen",
        "promptCenter.item.ember.title" => "Ember · Stimmungstagebuch",
        "promptCenter.item.volt.title" => "Volt · Elektroauto-Begleiter",
        "promptCenter.item.aloft.title" => "Aloft · Flugverfolgung",
        "promptCenter.item.gallery.title" => "Gallery · Ausstellungen und Kultur",
        "promptCenter.item.nightcap.title" => "Nightcap · Cocktails zu Hause",
        "promptCenter.item.bloom.title" => "Bloom · Familienmomente und Entwicklung",
        "promptCenter.item.extremeWeather.title" => "Extrem · Wetter-App",
        "promptCenter.item.extremeNowPlaying.title" => "Extrem · Aktueller Titel",
        "promptCenter.item.extremeDailyApp.title" => "Extrem · Jeden Tag öffnen",
        "promptCenter.item.extremeCalendar.title" => "Extrem · Kalender neu erfinden",
        "promptCenter.item.extremeCalm.title" => "Extrem · Ein Screen voller Ruhe",
        "promptCenter.item.webOrbit.title" => "Orbit · Landingpage für den KI-Arbeitsbereich",
        "promptCenter.item.webAtelier.title" => "Atelier · Möbel-E-Commerce",
        "promptCenter.item.dashboardPulse.title" => "Pulse · Wachstumsanalyse-Dashboard",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · Logistikbetrieb",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · Unternehmens-Datentabelle",
        "promptCenter.item.componentFormLab.title" => "Form Lab · Formular-Komponentensystem",
        "promptCenter.item.modifyPolishCurrent.title" => "Aktuellen Screen verfeinern",
        "promptCenter.item.modifyCompleteStates.title" => "Komponentenzustände vervollständigen",
        "collab.ownerConfirm.title" => "Bestätige, wem du beitrittst",
        "collab.ownerConfirm.hint" => "Aus dieser Sitzung wurde noch nichts geladen.",
        "collab.ownerConfirm.account" => "Verifiziertes Konto",
        "collab.ownerConfirm.device" => "Verifiziertes Gerät",
        "collab.ownerConfirm.claimedName" => "Von diesem Konto gewählter Name (nicht verifiziert)",
        "collab.action.confirmOwner" => "Dieser Sitzung beitreten",
        "collab.action.rejectOwner" => "Nicht beitreten",
        "collab.error.ownerNotConfirmed" => "Du hast den Host nicht bestätigt, daher wurde nichts geladen.",
        "sceneTemplate.title" => "Szenenvorlagen",
        "sceneTemplate.searchPlaceholder" => "Szenen oder Vorlagen suchen…",
        "sceneTemplate.empty" => "Keine passenden Vorlagen gefunden",
        "sceneTemplate.frames" => "Seiten: {{count}}",
        "sceneTemplate.filter.all" => "Alle",
        "sceneTemplate.scene.tutorial" => "Tutorialgrafik",
        "sceneTemplate.scene.comparison" => "Vergleichsgrafik",
        "sceneTemplate.scene.carousel" => "Wissenskarten",
        "sceneTemplate.scene.slides" => "Präsentation",
        "sceneTemplate.item.screenshotTutorial.title" => "Screenshot-Tutorial · 3 Schritte",
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "Cover, drei Anleitungsschritte und ein abschließender Call-to-Action. Screenshots und Texte ersetzen – fertig zur Veröffentlichung."
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "Wissens- und Insights-Karussell",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "Cover, drei Kernpunkte und eine Zusammenfassung – ideal, um einen Gedanken in wischbare Karten aufzuteilen."
        }
        "sceneTemplate.item.beforeAfter.title" => "Redesign-Vergleich: Vorher/Nachher",
        "sceneTemplate.item.beforeAfter.summary" => {
            "Vorher und Nachher nebeneinander mit Hinweisen zu den Änderungen – ideal für Retrospektiven und Portfolios."
        }
        "sceneTemplate.item.slideDeck.title" => "Präsentation · 6 Folien",
        "sceneTemplate.item.slideDeck.summary" => {
            "Cover, Agenda, Kernpunkte, Daten, Diagramm und Abschluss im 16:9-Format. Texte ersetzen und präsentieren."
        }
        "fileMenu.newFromTemplate" => "Neu aus Vorlage",
        _ => return super::de_collab::lookup(key),
    })
}
