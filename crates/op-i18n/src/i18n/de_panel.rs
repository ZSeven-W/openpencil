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
        "promptCenter.item.webKilnform.title" => "Kilnform · Website für Design-Infrastruktur",
        "promptCenter.item.webReefwright.title" => "Reefwright · Website für KI-Supportwissen",
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
        "sceneTemplate.generate.placeholder" => "Thema beschreiben – die KI erzeugt die ganze Präsentation",
        "sceneTemplate.generate.button" => "Erzeugen",
        "sceneTemplate.generate.hint" => "Ein neues Dokument, aus deinem Thema als vollständige Präsentation erzeugt.",
        "sceneTemplate.generate.promptTemplate" => "Erstelle eine Präsentation (PPT) zum folgenden Thema: {{topic}}",
        "sceneTemplate.card.addToCanvas" => "Zur Leinwand hinzufügen",
        "sceneTemplate.card.generateFrom" => "Damit erzeugen",
        "sceneTemplate.generate.basis" => "Basis: ",
        "sceneTemplate.filter.all" => "Alle",
        "sceneTemplate.scene.tutorial" => "Tutorials",
        "sceneTemplate.scene.comparison" => "Vergleich",
        "sceneTemplate.scene.carousel" => "Karussell",
        "sceneTemplate.scene.slides" => "Folien",
        "sceneTemplate.scene.card" => "Karten",
        "sceneTemplate.scene.web" => "Webseiten",
        "sceneTemplate.generate.webPromptTemplate" => "Entwirf eine mehrteilige Web-Landingpage zum folgenden Thema: {{topic}}",
        "sceneTemplate.item.saasLandingOrange.title" => "SaaS-Landingpage · Orange",
        "sceneTemplate.item.saasLandingOrange.summary" => "Eine helle Marketingseite aus fast schwarzen Flächen und einem Orange: Navigation, Hero mit Produktansicht, drei Feature-Karten, ein Workflow-Rundgang, Kundenstimmen und ein Abo-Footer. Texte tauschen, fertig ist die Website.",
        "sceneTemplate.item.productLandingLight.title" => "Produkt-Landingpage · Hell",
        "sceneTemplate.item.productLandingLight.summary" => "Eine papierweiße Produktseite im Zeitungslook: interaktive Hero-Demo, Feature-Spalten, Analyse-Board, Vorher-Nachher-Vergleich und drei Preisstufen. Für SaaS-Sites und Produktlaunches.",
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
        "sceneTemplate.item.knowledgeCardVertical.title" => "Wissenskarte · Hochformat",
        "sceneTemplate.item.knowledgeCardVertical.summary" => "Eine einzelne 3:4-Karte mit Überschrift, vier Kernpunkten und Signaturzeile. Texte ersetzen und posten.",
        "sceneTemplate.item.knowledgeCardSquare.title" => "Wissenskarte · Quadratisch",
        "sceneTemplate.item.knowledgeCardSquare.summary" => "Eine 1:1-Karte im gleichen Layout, kompakt genug für ein Beitragsbild oder einen Social-Post.",
        "sceneTemplate.item.pitchDeckDark.title" => "Pitch-Deck · Dunkel",
        "sceneTemplate.item.pitchDeckDark.summary" => "Titel, Problem, Lösung, Zahlen, Roadmap und Kontaktseite. Große Schrift auf dunklem Grund, gebaut für Finanzierungsrunden und Launches.",
        "sceneTemplate.item.lectureDeckLight.title" => "Vorlesungsfolien · Hell",
        "sceneTemplate.item.lectureDeckLight.summary" => "Kursdeckblatt, Lernziele, Konzepterklärung, Rechenbeispiel, Vergleichstabelle und Zusammenfassung. Papierweiß, auch über eine ganze Stunde angenehm.",
        "sceneTemplate.item.minimalKeynote.title" => "Minimalistische Keynote",
        "sceneTemplate.item.minimalKeynote.summary" => "Viel Weißraum, überdimensionale Schrift, ein zentrierter Satz pro Seite — neun Seiten ganz ohne Karten, das Inhaltsverzeichnis nur Linien und Zahlen. Für Launches und Keynotes.",
        "sceneTemplate.item.gradientTech.title" => "Gradient Tech",
        "sceneTemplate.item.gradientTech.summary" => "Dunkler Verlauf mit Milchglaskarten: Architektur, Benchmarks und eine Kundenwand. Für Entwickler-Produktlaunches.",
        "sceneTemplate.scene.infographic" => "Infografiken",
        "sceneTemplate.item.punchQuoteCard.title" => "Zitatkarte · Plakat",
        "sceneTemplate.item.punchQuoteCard.summary" => "Eine 3:4-Karte auf fast schwarzem Grund: zwei riesige Zeilen über einem gelben Balken. Ein Satz, mehr nicht — für Haltungen und Zitate.",
        "sceneTemplate.item.journalChecklistCard.title" => "Checklisten-Karte · Wissensdatenbank",
        "sceneTemplate.item.journalChecklistCard.summary" => "Eine weiße Karte auf hellgrauem Grund: fünf abhakbare Aufgaben, ein Tag und ein Zitatblock. Für Wochenpläne.",
        "sceneTemplate.item.dataReportInfographic.title" => "Daten-Infografik",
        "sceneTemplate.item.dataReportInfographic.summary" => "Ein hohes Scroll-Bild: dunkler Kopf, drei große Zahlen, ein Balkenvergleich, eine Aufteilung und drei Schlüsse. Zahlen tauschen und posten.",
        "sceneTemplate.item.stepsFlowInfographic.title" => "Schritt-für-Schritt-Infografik",
        "sceneTemplate.item.stepsFlowInfographic.summary" => "Ein hohes Scroll-Bild: fünf nummerierte Schritte zu einem Ablauf verkettet, jeder mit Zeitangabe, dazu zwei Hinweise. Für Anleitungen.",
        "sceneTemplate.item.eventPosterDeck.title" => "Event-Deck · Plakat",
        "sceneTemplate.item.eventPosterDeck.summary" => "Titel, Highlights, Programm, Anfahrt, Tickets und Abschluss. Galerieweißer Grund mit roten und blauen Flächen, keine runden Ecken und keine Verläufe — für Märkte, Vereinsevents und Eröffnungen.",
        "sceneTemplate.item.pitfallListInfographic.title" => "Infografik der häufigsten Fehler",
        "sceneTemplate.item.pitfallListInfographic.summary" => "Ein hohes Scroll-Bild: sechs Fehler nach Häufigkeit sortiert, je mit dem Problem und der Alternative, dazu eine Vier-Punkte-Prüfung vor dem Posten. Nur Schwarz, Weiß und Grau.",
        "sceneTemplate.item.spineCultureCard.title" => {
            "Karte mit vertikalem Titel · Mineralpigment"
        }
        "sceneTemplate.item.spineCultureCard.summary" => "Eine 3:4-Karte auf ockerfarbenem Lehmgrund: vertikaler chinesischer Titel, abblätternder Putz, Pigmentkörner. Für Kultur, lange Texte und Autorentitel.",
        "sceneTemplate.item.metricSingleCard.title" => "Einzelwert-Karte · Raster-Hanzi",
        "sceneTemplate.item.metricSingleCard.summary" => "Eine 1:1-Karte: eine riesige Zahl auf reinem Weiß, ein strenges Schweizer Raster und ein einziges rotes Signalquadrat. Für Ergebnisse und Fazit.",
        "sceneTemplate.item.quoteFrameCard.title" => "Zitatkarte · Seide Blaugrün",
        "sceneTemplate.item.quoteFrameCard.summary" => "Eine 4:5-Karte auf vergilbter Seide: ein gerahmter Satz, am Fuß ein Gebirge aus Azurit und Malachit. Für Auszüge, Interviews und Zitate.",
        "sceneTemplate.item.dailySignCard.title" => "Tageskarte · Gartenfenster",
        "sceneTemplate.item.dailySignCard.summary" => "Eine 3:4-Karte auf Kalkwand mit sechseckigem Gitterfenster: darin Datum und eine Zeile. Die Leere ist der Schmuck. Für Tagesposts und Markensätze.",
        "sceneTemplate.item.priceTierCard.title" => "Preisstaffel-Karte · Arkaden-Neon",
        "sceneTemplate.item.priceTierCard.summary" => "Eine 1:1-Karte auf tintenblauer Nacht: dreistufige Preisliste, Neonröhren-Konturen und ihr Streulicht. Für Läden, Events und Pakete.",
        "sceneTemplate.item.noticeBoardCard.title" => "Aushang-Karte · Bleisatz",
        "sceneTemplate.item.noticeBoardCard.summary" => "Eine 4:5-Karte auf Zeitungspapier: Kopflinien mit versetzter Rotplatte, nummerierte Punkte und eine Seriennummer. Für Aushänge und Hausordnungen.",
        "sceneTemplate.item.milestoneTimelineInfographic.title" => "Zeitleisten-Infografik",
        "sceneTemplate.item.milestoneTimelineInfographic.summary" => "Ein hohes Scroll-Bild: eine Achse über die ganze Höhe, Jahresmarken neben den Meilenstein-Karten, am Ende der nächste Schritt. Für Rückblicke, Markenhistorie und Projektverläufe.",
        "sceneTemplate.item.conceptContrastInfographic.title" => "Konzeptvergleich-Infografik",
        "sceneTemplate.item.conceptContrastInfographic.summary" => "Ein hohes Scroll-Bild: zuerst das Fazit, dann je eine Definitionskarte, eine zweispaltige Aufschlüsselung nach Kriterien und zum Schluss die Entscheidungshilfe.",
        "sceneTemplate.item.rankingBoardInfographic.title" => "Top-N-Ranking-Infografik",
        "sceneTemplate.item.rankingBoardInfographic.summary" => "Ein hohes Scroll-Bild: eine Empfehlungstafel in Gold auf Tinte — große Abzeichen für die ersten drei, Konturabzeichen für vier bis acht, je mit Einsatz und Häufigkeit.",
        "sceneTemplate.item.faqThreadInfographic.title" => "FAQ-Infografik",
        "sceneTemplate.item.faqThreadInfographic.summary" => "Ein hohes Scroll-Bild: sechs Frage-Antwort-Paare, F gefüllt und A als Kontur. Ohne Nummern und Reihenfolge — jedes Paar steht für sich.",
        "sceneTemplate.item.dataStoryInfographic.title" => "Datengeschichte-Infografik",
        "sceneTemplate.item.dataStoryInfographic.summary" => "Ein hohes Scroll-Bild: vier Zahlen zu einer Kausalkette verknüpft, jede Stufe als Zehnerraster, am Ende ein Fazit zum Handeln.",
        "sceneTemplate.item.challengeTrackerInfographic.title" => "30-Tage-Challenge-Infografik",
        "sceneTemplate.item.challengeTrackerInfographic.summary" => "Ein hohes Scroll-Bild: ein Raster aus dreißig Feldern, sechs mal fünf, mit Meilensteinen nur an Tag 7, 15 und 30. Speichern und täglich eins abhaken.",
        "sceneTemplate.item.ecosystemMapInfographic.title" => "Ökosystem-Karten-Infografik",
        "sceneTemplate.item.ecosystemMapInfographic.summary" => "Ein hohes Scroll-Bild: vier Positionen einer Kette als Zwei-mal-zwei, je drei Akteure, die Lücken benannt. Weiße Karten auf Schiefer.",
        "sceneTemplate.item.doDontComparison.title" => "Zwei Spalten: so und nicht so",
        "sceneTemplate.item.doDontComparison.summary" => "Eine 3:4-Karte: zwei Wege für dieselbe Sache nebeneinander, unterschieden durch Material und Symbol statt durch Rot und Grün — auch für Farbfehlsichtige lesbar.",
        "sceneTemplate.item.mythTruthComparison.title" => "Irrtum und Wirklichkeit",
        "sceneTemplate.item.mythTruthComparison.summary" => "Ein hohes Bild: fünf Paare „man sagt / tatsächlich“, der Irrtum schmal und hell links, die Wirklichkeit breit und dunkel rechts.",
        "sceneTemplate.item.pricingTiersComparison.title" => "Preisstufen im Vergleich",
        "sceneTemplate.item.pricingTiersComparison.summary" => "Eine 3:4-Karte: Free, Pro und Team nebeneinander, der Preis als Anker, jede Spalte enthält die vorige. Für Preisseiten.",
        "sceneTemplate.item.scenarioGuideComparison.title" => "Auswahlhilfe nach Situation",
        "sceneTemplate.item.scenarioGuideComparison.summary" => "Ein hohes Bild: keine Datenblätter, sieben Situationen, jede mit einem Urteil versehen. Man sucht nur seine Zeile.",
        "sceneTemplate.item.specTableComparison.title" => "Spezifikationstabelle",
        "sceneTemplate.item.specTableComparison.summary" => "Ein hohes Bild: zwei Kandidaten in einer echten Tabelle, Zeile für Zeile, die Gewinnerzelle mit dunklem Grund hervorgehoben.",
        "sceneTemplate.item.threeWayComparison.title" => "Dreiwege-Vergleich",
        "sceneTemplate.item.threeWayComparison.summary" => "Ein hohes Bild: drei Optionen nebeneinander, die Empfehlung in der Mitte; jede Spalte beginnt mit einer Situation statt mit einem Namen.",
        "sceneTemplate.item.timeShiftComparison.title" => "Vor einem Jahr und heute",
        "sceneTemplate.item.timeShiftComparison.summary" => "Eine 3:4-Karte: eine mittige Beschriftungsachse, vor einem Jahr links, heute rechts, beide Werte in derselben Zeile.",
        "sceneTemplate.item.tradeoffScaleComparison.title" => "Waage der Vor- und Nachteile",
        "sceneTemplate.item.tradeoffScaleComparison.summary" => "Eine 1:1-Karte: ein Balken, zwei Schalen — links der Wert, rechts der Preis, vor jeder Zeile ein leeres Kästchen.",
        "sceneTemplate.item.versionDiffComparison.title" => "Änderungen zwischen Versionen",
        "sceneTemplate.item.versionDiffComparison.summary" => "Eine 1:1-Karte: keine Spalten — jede Zeile erledigt ihr eigenes „alt → neu“, einfach durchscrollen.",
        "sceneTemplate.item.appOnboardingTriptych.title" => "App-Onboarding-Triptychon",
        "sceneTemplate.item.appOnboardingTriptych.summary" => "Eine 3:4-Karte: drei Telefone nebeneinander mit leeren Bildfeldern. Eigene drei Screens einsetzen, Text dazu — fertig für Review oder Post.",
        "sceneTemplate.item.diyBlueprintGuide.title" => "DIY-Bauanleitung",
        "sceneTemplate.item.diyBlueprintGuide.summary" => "Ein hohes Bild, in dem die Materialtabelle so viel Raum bekommt wie die Schritte — DIY scheitert an der Vorbereitung, nicht an den Händen.",
        "sceneTemplate.item.photoCompositionTutorial.title" => "Bildkomposition mit dem Handy",
        "sceneTemplate.item.photoCompositionTutorial.summary" => "3:4, fünf Seiten: je ein dunkler Sucher mit leuchtenden Hilfslinien über dem Bildfeld — Komposition lässt sich nur am Rahmen erklären.",
        "sceneTemplate.item.recipeFourStep.title" => "Rezept in vier Schritten",
        "sceneTemplate.item.recipeFourStep.summary" => "Eine 4:5-Karte im 2×2: alle vier Schritte auf einer Karte. Screenshot machen und danach kochen — am Herd blättert niemand.",
        "sceneTemplate.item.skincareRoutineCards.title" => "Pflegeroutine-Karten",
        "sceneTemplate.item.skincareRoutineCards.summary" => "4:5, sechs Seiten: jeder Schritt nennt drei Zahlen — Menge, Wartezeit, morgens oder abends. Fehler passieren bei Dosis und Abstand.",
        "sceneTemplate.item.softwareStepTutorial.title" => "Software-Schrittanleitung",
        "sceneTemplate.item.softwareStepTutorial.summary" => "Eine 4:5-Karte, die einzige dunkle der Reihe: Screenshot-Felder mit nummerierten Anweisungen, für Tools und Funktionen.",
        "sceneTemplate.item.storageMakeoverSteps.title" => "Ordnungs-Umbau in Schritten",
        "sceneTemplate.item.storageMakeoverSteps.summary" => "3:4, sechs Seiten: neben Handgriff und Bildfeld nennt jeder Schritt ein Fertig-Kriterium und ein Zeitbudget.",
        "sceneTemplate.item.weeklyReportLesson.title" => "Lektion Wochenbericht",
        "sceneTemplate.item.weeklyReportLesson.summary" => "Ein hohes Bild: nach der Viererstruktur folgt ein Gerüst mit unterstrichenen Lücken — Screenshot machen und ausfüllen.",
        "sceneTemplate.item.workoutBreakdownGuide.title" => "Übungen im Detail",
        "sceneTemplate.item.workoutBreakdownGuide.summary" => "Ein hohes Bild: jede Übung trägt neben Bildfeld und Hinweisen eine feste Leiste aus Sätzen, Wiederholungen und Pause.",
        "sceneTemplate.item.bookreviewSilkCarousel.title" => "Karussell zur Buch-/Filmanalyse",
        "sceneTemplate.item.bookreviewSilkCarousel.summary" => "3:4, fünf Tafeln: Aufhänger, kommentiertes Zitat, drei Einsichten, ein zitierbarer Satz, Schluss. Es zerlegt das Werk in mitnehmbare Teile statt die Handlung nachzuerzählen.",
        "sceneTemplate.item.cityguideFilmCarousel.title" => "Stadtführer-Karussell",
        "sceneTemplate.item.cityguideFilmCarousel.summary" => "3:4, sieben Tafeln: Orte und Wege im Wechsel — die Orte für die Träumenden, Tagesroute und Essen-Schlafen-Tabelle für die Planenden.",
        "sceneTemplate.item.datareportGridCarousel.title" => "Datenbericht-Karussell",
        "sceneTemplate.item.datareportGridCarousel.summary" => "3:4, sechs Tafeln: auf jede Datentafel folgt eine ohne Daten, damit niemand beim dritten Diagramm weiterwischt.",
        "sceneTemplate.item.opinionLongformCarousel.title" => "Karussell für lange Meinungstexte",
        "sceneTemplate.item.opinionLongformCarousel.summary" => "3:4, sechs Tafeln: eine strenge Vorlage durchgehend, Seitenzahl und Titel immer am selben Platz.",
        "sceneTemplate.item.qaChalkboardCarousel.title" => "Frage-Antwort-Karussell",
        "sceneTemplate.item.qaChalkboardCarousel.summary" => "3:4, sechs Tafeln: eine Frage je Tafel, mit handgezeichneter Fragezeichen-Nummer in der Ecke.",
        "sceneTemplate.item.storyNightCarousel.title" => "Erzähl-Karussell",
        "sceneTemplate.item.storyNightCarousel.summary" => "3:4, sieben Tafeln: ein persönlicher Rückblick auf der Achse Zeit — der Zeitstrahl auf Tafel fünf ist die tragende Wand.",
        "sceneTemplate.item.toolkitNotebookCarousel.title" => "Toolkit-Karussell",
        "sceneTemplate.item.toolkitNotebookCarousel.summary" => "3:4, sechs Tafeln: sechs Werkzeuge je eine Tafel, die letzte listet sie mit Seitenzahlen — Sammlungen liest man, um sie zu speichern.",
        "sceneTemplate.item.tutorialJournalCarousel.title" => "Tutorial-Karussell",
        "sceneTemplate.item.tutorialJournalCarousel.summary" => {
            "3:4, sechs Tafeln: ein Schritt je Tafel, der Finger ist die Fortschrittsleiste."
        }
        "sceneTemplate.item.yearreviewMineralCarousel.title" => "Jahresrückblick-Karussell",
        "sceneTemplate.item.yearreviewMineralCarousel.summary" => {
            "3:4, acht Tafeln: Zahlentafeln kühl, Reflexionstafeln warm, im Wechsel."
        }
        "fileMenu.newFromTemplate" => "Neu aus Vorlage",
        "fileMenu.exportSlideshowHtml" => "Diashow als HTML exportieren...",
        "fileMenu.exportPptx" => "Als PowerPoint exportieren...",
        "dialog.slideshowHtmlTitle" => "Diashow exportieren",
        "dialog.slideshowHtmlSummary" => "{{count}} Folien exportiert nach:",
        "dialog.slideshowHtmlEmpty" => "Diese Präsentation hat keine sichtbaren Folien zum Exportieren.",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "Importierbare HTML-Inhalte sind nicht verfügbar.",
        "htmlImport.warn.content.empty_body" => {
            "Importierbare Inhalte im HTML-Rumpf sind nicht verfügbar."
        }
        "htmlImport.warn.content.dom_depth_truncated" => {
            "HTML, das tiefer als {{max_depth}} Ebenen verschachtelt ist, wurde verworfen."
        }
        "htmlImport.warn.content.node_limit_truncated" => {
            "Knotenlimit erreicht; der restliche Seiteninhalt wurde ausgelassen."
        }
        "htmlImport.warn.content.node_limit_mapping" => {
            "Knotenlimit erreicht; ein Teil des HTML-Baums wurde ausgelassen."
        }
        "htmlImport.warn.content.node_limit_inline_row" => {
            "Knotenlimit erreicht; eine Zeile für Inline-Formatierung wurde ausgelassen."
        }
        "htmlImport.warn.content.node_limit_pseudo" => {
            "Knotenlimit erreicht; erzeugte Pseudo-Elemente wurden ausgelassen."
        }
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "CSS-Regeln, die tiefer als {{max_depth}} At-Regeln verschachtelt sind, wurden ignoriert."
        }
        "htmlImport.warn.css.unterminated_rule" => {
            "Eine nicht abgeschlossene CSS-Regel wurde ignoriert."
        }
        "htmlImport.warn.css.marker_rules_unsupported" => {
            "CSS-::marker-Regeln wurden nicht importiert."
        }
        "htmlImport.warn.css.nesting_unsupported" => {
            "Verschachtelte CSS-Stilregeln wurden ignoriert."
        }
        "htmlImport.warn.css.invalid_layer_name" => {
            "Der ungültige @layer-Name '{{name}}' wurde ignoriert."
        }
        "htmlImport.warn.css.unsupported_statement" => {
            "Die nicht unterstützte @{{name}}-Anweisung wurde ignoriert."
        }
        "htmlImport.warn.css.media_without_viewport" => {
            "@media-Regeln ohne Anzeigebereich wurden ignoriert."
        }
        "htmlImport.warn.css.invalid_layer_block_name" => {
            "Der ungültige @layer-Blockname '{{name}}' wurde ignoriert."
        }
        "htmlImport.warn.css.unsupported_container_block" => {
            "Der @container-Block wurde ignoriert."
        }
        "htmlImport.warn.css.unsupported_block" => {
            "Der nicht unterstützte @{{name}}-Block wurde ignoriert."
        }
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "Die @font-face-Webschrift '{{family}}' ist nicht verfügbar."
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "Prozentuale Versätze eines absolut positionierten Elements wurden angenähert."
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "Prozentuale position:relative-Versätze wurden angenähert."
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "CSS-aspect-ratio ohne festgelegte Achse wurde ignoriert."
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "CSS-aspect-ratio in einem unbestimmten enthaltenden Block wurde ignoriert."
        }
        "htmlImport.warn.layout.position_sticky_ignored" => "CSS-position:sticky wurde ignoriert.",
        "htmlImport.warn.layout.grid_tracks_approximated" => {
            "Nicht unterstützte CSS-Rasterspuren wurden angenähert."
        }
        "htmlImport.warn.layout.float_ignored" => "CSS-float wurde ignoriert.",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "CSS-mix-blend-mode auf Knotenebene wurde angenähert."
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "CSS-overflow: auto / scroll wurde angenähert."
        }
        "htmlImport.warn.layout.negative_margins_ignored" => {
            "Negative CSS-Außenabstände wurden ignoriert."
        }
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => {
            "CSS-Außenabstände an einem sichtbaren Kasten wurden ignoriert."
        }
        "htmlImport.warn.layout.inline_margin_wrapping_approximated" => "Ein Inline-Element mit CSS-Außenabständen wurde als Box angenähert und kann möglicherweise nicht mehr über Zeilen umbrechen.",
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "Prozentuale Größenberechnung mit content-box wurde angenähert."
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "Durch ausdrückliche Startlinien entstandene leere CSS-Rasterzellen wurden angenähert."
        }
        "htmlImport.warn.layout.grid_span_reflowed" => {
            "Ein CSS-Rasterelement, dessen Bereich nicht zu seiner Startlinie passte, wurde angenähert."
        }
        "htmlImport.warn.layout.grid_rows_node_limit" => {
            "Knotenlimit erreicht; Zeilencontainer des CSS-Rasters wurden ausgelassen."
        }
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "CSS-Rasterspurbreiten mit auto-fit / auto-fill wurden angenähert."
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "Die Platzierung per CSS-grid-template-areas wurde nicht importiert."
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => {
            "Die Platzierung per CSS-grid-row wurde nicht importiert."
        }
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "CSS-grid-column `{{value}}` wurde angenähert."
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => {
            "Automatische CSS-Außenabstände in der Blockachse wurden nicht importiert."
        }
        "htmlImport.warn.layout.auto_margin_node_limit" => {
            "Knotenlimit erreicht; die Ausrichtung über automatische CSS-Außenabstände wurde ausgelassen."
        }
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "Ein CSS-Versatz im Fluss an einem Element ohne festgelegte Größe wurde verworfen."
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => {
            "Knotenlimit erreicht; ein CSS-Versatz im Fluss wurde ausgelassen."
        }
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "CSS-Versätze im Fluss (position:relative-Abstände, transform-Verschiebung) wurden angenähert."
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "Ein CSS-Versatz im Fluss an einem Kasten, der keinen Versatzcontainer aufnehmen kann, wurde verworfen."
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "flex-wrap an einem spaltenweisen Flex-Container wurde nicht importiert."
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => {
            "flex-wrap:wrap-reverse wurde angenähert."
        }
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "flex-wrap an einem Container ohne festgelegte Breite wurde ignoriert."
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "CSS-align-content an einem umbrechenden Flex-Container wurde nicht importiert."
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "flex-wrap mit unbestimmten Hauptachsengrößen der Kindelemente wurde ignoriert."
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => {
            "Knotenlimit erreicht; flex-wrap-Zeilen wurden ausgelassen."
        }
        "htmlImport.warn.transform.unsupported_syntax" => {
            "Nicht unterstützte CSS-transform-Syntax wurde ignoriert."
        }
        "htmlImport.warn.transform.unsupported_function" => {
            "Nicht unterstützte CSS-transform-Funktionen (3D, matrix3d) wurden ignoriert."
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "Eine prozentuale CSS-transform-Verschiebung auf einer unbestimmten Achse wurde verworfen."
        }
        "htmlImport.warn.transform.non_finite_matrix" => {
            "Eine CSS-Transformation, die eine nicht endliche Matrix ergab, wurde ignoriert."
        }
        "htmlImport.warn.transform.skew_dropped" => "CSS-transform-Scherung wurde verworfen.",
        "htmlImport.warn.transform.degenerate_scale" => {
            "Eine CSS-Transformation mit Skalierung null oder nicht endlichem Wert wurde angenähert."
        }
        "htmlImport.warn.transform.mirroring_absolute" => {
            "CSS-transform-Spiegelung wurde angenähert."
        }
        "htmlImport.warn.transform.origin_z_ignored" => {
            "Der Z-Versatz von CSS-transform-origin wurde ignoriert."
        }
        "htmlImport.warn.transform.scale_not_baked" => {
            "Eine CSS-transform-Skalierung, die nicht in die Knotengröße übernommen werden konnte, wurde verworfen."
        }
        "htmlImport.warn.transform.scale_baked" => {
            "In die Knotengröße übernommene CSS-transform-Skalierung wurde angenähert."
        }
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "CSS-transform-Skalierung an einem automatisch dimensionierten Element wurde ignoriert."
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "Gerichtetes oder verteiltes CSS-background-repeat wurde angenähert."
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => {
            "Eine ausdrückliche CSS-Kachelgröße des Hintergrunds wurde ignoriert."
        }
        "htmlImport.warn.visual.background_size_auto_box" => {
            "CSS-background-size an einem automatisch dimensionierten Element wurde angenähert."
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "CSS-background-size, das die eigene Bildgröße benötigt, wurde angenähert."
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "Eine nicht unterstützte CSS-background-position wurde ignoriert."
        }
        "htmlImport.warn.visual.background_image_url_empty" => {
            "Eine leere URL eines CSS-Hintergrundbilds wurde ignoriert."
        }
        "htmlImport.warn.visual.conic_gradient_ignored" => {
            "Konische CSS-Verläufe wurden ignoriert."
        }
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "Eine nicht unterstützte CSS-background-image-Ebene wurde ignoriert."
        }
        "htmlImport.warn.visual.background_color_unresolved" => {
            "Eine nicht aufgelöste CSS-Hintergrundfarbe wurde ignoriert."
        }
        "htmlImport.warn.visual.background_position_dropped" => {
            "CSS-background-position wurde ignoriert."
        }
        "htmlImport.warn.visual.border_colors_approximated" => {
            "Seitenweise CSS-Rahmenfarben wurden angenähert."
        }
        "htmlImport.warn.visual.border_styles_approximated" => {
            "Gemischte seitenweise CSS-Rahmenstile wurden angenähert."
        }
        "htmlImport.warn.visual.border_style_complex" => {
            "Ein komplexer CSS-Rahmenstil wurde angenähert."
        }
        "htmlImport.warn.visual.border_style_unsupported" => {
            "Ein nicht unterstützter CSS-Rahmenstil wurde angenähert."
        }
        "htmlImport.warn.visual.border_radius_elliptical" => {
            "Elliptische CSS-Eckenradien wurden angenähert."
        }
        "htmlImport.warn.visual.border_radius_unsupported" => {
            "Ein nicht unterstützter CSS-Eckenradius wurde ignoriert."
        }
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "Eine nicht unterstützte CSS-box-shadow-Ebene wurde ignoriert."
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => {
            "Die Farbinterpolationsmethode des CSS-Verlaufs wurde ignoriert."
        }
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "Eine nicht unterstützte Richtung von CSS-linear-gradient wurde ignoriert."
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => {
            "Farbhinweise in CSS-Verläufen wurden ignoriert."
        }
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => {
            "Ein nicht unterstützter Farbstopp eines CSS-Verlaufs wurde ignoriert."
        }
        "htmlImport.warn.visual.gradient_too_few_stops" => {
            "Ein CSS-Verlauf mit weniger als zwei nutzbaren Farbstopps wurde ignoriert."
        }
        "htmlImport.warn.visual.gradient_repeating_approximated" => {
            "Ein sich wiederholender CSS-Verlauf wurde angenähert."
        }
        "htmlImport.warn.visual.gradient_stops_clamped" => {
            "Farbstopps von CSS-Verläufen außerhalb des gültigen Bereichs wurden angenähert."
        }
        "htmlImport.warn.visual.blur_radius_unsupported" => {
            "Ein nicht unterstützter CSS-Weichzeichnungsradius wurde ignoriert."
        }
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "Ein nicht unterstütztes CSS-filter-drop-shadow() wurde ignoriert."
        }
        "htmlImport.warn.visual.filter_function_unsupported" => {
            "Eine nicht unterstützte CSS-Filterfunktion wurde ignoriert."
        }
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "Eine nicht unterstützte CSS-backdrop-filter-Funktion wurde ignoriert."
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "Ein nicht unterstützter CSS-background-blend-mode wurde ignoriert."
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "CSS-mix-blend-mode an einzelnen Füllungen wurde angenähert."
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "Ein nicht unterstützter CSS-mix-blend-mode wurde ignoriert."
        }
        "htmlImport.warn.visual.property_not_representable" => "CSS-{{property}} wurde ignoriert.",
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "CSS-background-size an einem Verlauf wurde ignoriert."
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "Eine nicht unterstützte Position von CSS-radial-gradient wurde ignoriert."
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "Ein elliptischer CSS-radial-gradient wurde angenähert."
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "Ein Ausdehnungsschlüsselwort von CSS-radial-gradient wurde angenähert."
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "Eine nicht unterstützte Größe von CSS-radial-gradient wurde ignoriert."
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => {
            "Eine nicht unterstützte CSS-text-shadow-Ebene wurde ignoriert."
        }
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "CSS-text-shadow-Ebenen nach der ersten wurden ignoriert."
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => {
            "CSS-text-shadow an einem Inline-Element wurde ignoriert."
        }
        "htmlImport.warn.list.style_image_ignored" => {
            "CSS-list-style-image wurde nicht importiert."
        }
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "Eine hängende Aufzählungsmarke mit `list-style-position: outside` wurde angenähert."
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "Der nicht unterstützte CSS-list-style-type `{{value}}` wurde angenähert."
        }
        "htmlImport.warn.media.object_fit_scale_down" => {
            "CSS-object-fit:scale-down wurde angenähert."
        }
        "htmlImport.warn.media.object_fit_none_ignored" => "CSS-object-fit:none wurde ignoriert.",
        "htmlImport.warn.media.object_position_ignored" => "CSS-object-position wurde ignoriert.",
        "htmlImport.warn.media.image_intrinsic_axis_unresolved" => {
            "Das intrinsische Seitenverhältnis des Bildes konnte die fehlende Achse nicht bestimmen, da die angegebene Größe dynamisch ist oder der umschließende Block keine bestimmte Größe hat."
        }
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "Ein nicht unterstützter CSS-mix-blend-mode an einem Bild wurde ignoriert."
        }
        "htmlImport.warn.media.inline_svg_placeholder" => {
            "Ein eingebettetes <svg>-Element wurde als Platzhalter importiert."
        }
        "htmlImport.warn.media.input_type_fallback" => {
            "Ein nicht unterstützter <input>-Typ wurde angenähert."
        }
        "htmlImport.warn.media.element_placeholder" => {
            "Das <{{tag}}>-Element wurde als Platzhalter importiert."
        }
        "htmlImport.warn.media.picture_undecodable_types" => {
            "Ein <picture> mit ausschließlich nicht dekodierbaren Quelltypen wurde angenähert."
        }
        "htmlImport.warn.table.rowspan_ignored" => {
            "Das HTML-Attribut rowspan wurde nicht importiert."
        }
        "htmlImport.warn.table.row_groups_unflattened" => {
            "Spaltenbreiten einer Tabelle mit durch CSS entflachten Zeilengruppen wurden angenähert."
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "Spaltenbreiten einer CSS-Tabelle ohne festgelegte Breite wurden angenähert."
        }
        "htmlImport.warn.resource.invalid_base_href" => {
            "Das ungültige <base href> {{href}} wurde ignoriert."
        }
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "Das <base href> {{href}} außerhalb des Projektursprungs wurde ignoriert."
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => {
            "Die externe CSS-Stilvorlage {{url}} ist nicht verfügbar."
        }
        "htmlImport.warn.resource.image_outside_origin" => {
            "Das Bild {{url}} außerhalb des Projektursprungs wurde als Platzhalter importiert."
        }
        "htmlImport.warn.resource.image_unavailable" => {
            "Das nicht verfügbare Bild {{url}} wurde als Platzhalter importiert."
        }
        "htmlImport.warn.resource.css_import_invalid" => {
            "Der ungültige CSS-@import {{prelude}} wurde ignoriert."
        }
        "htmlImport.warn.resource.css_import_unresolvable" => {
            "Der CSS-@import {{reference}} ist nicht verfügbar."
        }
        "htmlImport.warn.resource.css_import_cycle" => {
            "Der zyklische CSS-@import {{url}} wurde ignoriert."
        }
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "Der CSS-@import {{url}} jenseits von Tiefe {{max_depth}} wurde ignoriert."
        }
        "htmlImport.warn.resource.css_import_unavailable" => {
            "Der CSS-@import {{url}} ist nicht verfügbar."
        }
        "htmlImport.warn.project.multiple_html_entries" => {
            "{{count}} HTML-Einstiegsdateien gefunden; {{entry}} wurde gewählt, der Rest wurde angenähert."
        }
        "htmlImport.warn.snapshot.truncated" => {
            "Ein Teil der Browser-Momentaufnahme wurde verworfen."
        }
        "htmlImport.warn.snapshot.node_limit" => {
            "Knotenlimit erreicht; der restliche Inhalt der Momentaufnahme wurde ausgelassen."
        }
        "htmlImport.warn.snapshot.tainted_images" => {
            "{{count}} durch CORS belastete Bilder wurden als entfernte URLs beibehalten und sind nicht verfügbar."
        }
        "htmlImport.warn.snapshot.invalid_rect" => {
            "Ein Knoten der Momentaufnahme mit fehlendem oder ungültigem Rechteck wurde verworfen."
        }
        "htmlImport.warn.snapshot.unknown_kind" => {
            "Ein Knoten der Momentaufnahme unbekannter Art wurde verworfen."
        }
        "htmlImport.warn.snapshot.rejected" => {
            "Die Browser-Momentaufnahme ({{reason}}) wurde verworfen."
        }
        "htmlImport.warn.snapshot.unsupported_transform" => {
            "Eine nicht unterstützte Transformation der Momentaufnahme wurde ignoriert."
        }
        "htmlImport.warn.css.media_empty_query" => "Eine leere @media-Abfrage wurde ignoriert.",
        "htmlImport.warn.css.media_unsupported_type" => {
            "Der nicht unterstützte @media-Typ '{{name}}' wurde ignoriert."
        }
        "htmlImport.warn.css.media_unsupported_condition" => {
            "Die nicht unterstützte @media-Bedingung '{{input}}' wurde ignoriert."
        }
        "htmlImport.warn.css.media_invalid_orientation" => {
            "Die ungültige @media-Ausrichtung '{{value}}' wurde ignoriert."
        }
        "htmlImport.warn.css.media_unsupported_feature" => {
            "Das nicht unterstützte @media-Merkmal '{{name}}' wurde ignoriert."
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "Der nicht unterstützte @media-Bereich '({{input}})' wurde ignoriert."
        }
        "htmlImport.warn.css.media_invalid_range" => {
            "Der ungültige @media-Bereich '({{input}})' wurde ignoriert."
        }
        "htmlImport.warn.css.media_invalid_length" => {
            "Die ungültige @media-Länge '{{value}}' wurde ignoriert."
        }
        "htmlImport.diagnostics.title" => "HTML-Import abgeschlossen",
        "htmlImport.diagnostics.summary" => "Eingeschränkte Elemente: {{count}}",
        "htmlImport.diagnostics.dismiss" => "Schließen",
        "htmlImport.diagnostics.expand" => "Details anzeigen",
        "htmlImport.diagnostics.collapse" => "Details ausblenden",
        "htmlImport.diagnostics.more" => "+{{count}} weitere",
        "dialog.pptxTitle" => "Als PowerPoint exportieren",
        "dialog.pptxSummary" => "{{count}} Folien exportiert nach:",
        "dialog.pptxEmpty" => "Diese Präsentation hat keine sichtbaren Folien zum Exportieren.",
        "settings.agents.acpQuickAdd" => "Schnell hinzufügen",
        "settings.agents.acpPresetAdd" => "Hinzufügen",
        "settings.agents.acpNotInstalled" => "Nicht installiert",
        "assetCenter.title" => "Asset-Center",
        "assetCenter.tab.templates" => "Vorlagen",
        "assetCenter.tab.styles" => "Stile",
        "assetCenter.style.empty" => "Keine passenden Stile",
        "assetCenter.style.pinned" => "Angeheftet",
        "assetCenter.style.searchPlaceholder" => "Stile oder Tags suchen",
        "assetCenter.style.generateHint" => "Ein neues Dokument aus deinem Thema, im angehefteten Stil.",
        "ai.pinnedStyle" => "Stil: {{name}}",
        "assetCenter.style.import" => "Stil importieren",
        "assetCenter.style.mine" => "Meine Stile",
        "assetCenter.style.builtIn" => "Integrierte Stile",
        "assetCenter.style.importTitle" => "DESIGN.md importieren",
        "assetCenter.style.importHint" => "Füge die vollständige DESIGN.md ein und bestätige den Import.",
        "assetCenter.style.importSource" => "Du kannst einen Stil aus einer DESIGN.md-Bibliothek wie styles.refero.design kopieren.",
        "assetCenter.style.importConfirm" => "Importieren",
        "assetCenter.style.importCancel" => "Abbrechen",
        "assetCenter.style.importPickFile" => "Datei wählen…",
        "assetCenter.style.importHintFile" => "Wähle eine DESIGN.md-Datei oder füge das ganze Dokument unten ein.",
        "assetCenter.style.importPlaceholder" => "DESIGN.md hier einfügen",
        "assetCenter.style.importEmpty" => "Diese Datei ist leer oder zu kurz für einen Styleguide.",
        "assetCenter.style.importNotText" => "Diese Datei lässt sich nicht als Markdown-Text lesen.",
        "assetCenter.style.importTooLarge" => "Diese Datei ist größer als 512 KB.",
        "slidesPanel.tabSlides" => "Folien",
        "slidesPanel.tabCards" => "Karten",
        "slidesPanel.present" => "Präsentieren",
        "slidesPanel.exportPdf" => "Als PDF exportieren",
        "slidesPanel.exportAllSlides" => "Alle Folien exportieren",
        "slidesPanel.exportSelectedSlides" => "Ausgewählte Folien exportieren ({{count}})",
        "settings.tab.ai" => "KI",
        "settings.agents.heroTitle" => "Verbinde deinen KI-Anbieter",
        "settings.agents.heroSubtitle" => "OpenPencil steuert deine lokalen CLI-Agenten und API-Anbieter — verbinde einen, um Designs zu generieren.",
        "settings.agents.statusConnected" => "Verbunden",
        "settings.agents.statusNotConnected" => "Nicht verbunden",
        "settings.agents.statusChecking" => "Status wird geprüft…",
        "settings.mcp.heroTitle" => "OpenPencil extern über MCP verbinden",
        "settings.mcp.heroSubtitle" => "Richte ein beliebiges MCP-fähiges CLI oder einen Editor auf diesen Workspace und steuere die Leinwand mit denselben Werkzeugen wie der eingebaute Agent.",
        "settings.mcp.terminalFootnote" => "* Beim Start wird MCP automatisch für die ausgewählten CLI-Tools eingerichtet.",
        "settings.mcp.customConfigTitle" => "Eigene MCP-Serverkonfiguration",
        "settings.mcp.customConfigDesc" => "Füge das in jeden Client ein, der einen Standard-MCP-Serverblock liest.",
        "settings.mcp.copyConfig" => "MCP-Konfiguration kopieren",
        "settings.system.heroTitle" => "Systemeinstellungen",
        "settings.system.heroSubtitle" => "Erscheinungsbild, Updates und Leinwandverhalten dieser Installation.",
        "settings.system.appearance" => "Erscheinungsbild",
        "settings.system.appearanceLight" => "Hell",
        "settings.system.appearanceDark" => "Dunkel",
        "settings.system.pencilCursor" => "Stiftzeiger",
        "settings.images.heroTitle" => "Bilder für deine Designs",
        "settings.images.heroSubtitle" => "Suche Fotos auf Openverse oder verbinde einen Anbieter, um sie bei Bedarf zu erzeugen.",
        "settings.fonts.heroTitle" => "Schriften in diesem Dokument",
        "settings.fonts.heroSubtitle" => "Ersetze Schriften, die ein Dokument verlangt und die hier fehlen, und verwalte deine importierten.",
        "settings.account.heroTitle" => "Dein Konto",
        "settings.account.heroSubtitle" => "Melde dich an, um Workspace und Lizenz geräteübergreifend zu synchronisieren.",
        "tooltip.topbar.file" => "Datei",
        "tooltip.topbar.import" => "Importieren",
        "tooltip.topbar.language" => "Sprache",
        "tooltip.topbar.collaboration" => "Zusammenarbeit",
        "tooltip.topbar.preview" => "Vorschau",
        "tooltip.topbar.exitPreview" => "Vorschau beenden",
        "tooltip.topbar.account" => "Konto",
        "settings.agents.providerRollMore" => "und {{count}} weitere",
        "ai.thinking.adaptive" => "Denken: automatisch",
        "ai.thinking.disabled" => "Denken: aus",
        "ai.thinking.enabled" => "Denken: ein",
        "ai.designProgress.detail.repairsApplied" => "{{count}} automatische Korrektur(en) angewendet",
        "ai.designProgress.detail.repairsMore" => "… und {{count}} weitere (siehe Protokoll)",
        "ai.styleCard.builtin" => "Integrierter Stil",
        "ai.styleCard.imported" => "Importierte DESIGN.md",
        "ai.styleCard.documentDesignMd" => "design.md des Dokuments",
        _ => return super::de_collab::lookup(key),
    })
}
