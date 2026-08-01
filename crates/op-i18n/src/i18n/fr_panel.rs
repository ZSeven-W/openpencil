//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `fr_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Rechercher des images…",
        "imagePanel.searching" => "Recherche…",
        "imagePanel.noResults" => "Aucun résultat",
        "imagePanel.searchPrompt" => "Recherchez des images",
        "imagePanel.sourceNotice" => {
            "Images de {{source}}. Licence libre — vérifiez la licence avant utilisation."
        }
        "imagePanel.genNotConfigured" => "Génération d'images non configurée",
        "imagePanel.openSettings" => "Ouvrir les réglages",
        "imagePanel.promptPlaceholder" => "Décrivez l'image…",
        "providerProbe.connectedViaCli" => "Connecté via la CLI {{name}}",
        "providerProbe.cliExitedWithError" => "La CLI {{name}} s'est arrêtée sur une erreur",
        "providerProbe.cliNoVersionOutput" => "La CLI {{name}} n'a produit aucune information de version",
        "providerProbe.modelQueryFailed" => "La requête de modèles {{name}} a échoué ou expiré",
        "providerProbe.modelQueryFailedRunLogin" => "La requête de modèles {{name}} a échoué. Exécutez {{command}} une fois pour vous authentifier.",
        "providerProbe.modelQueryNeedsAuth" => "La requête de modèles {{name}} nécessite une authentification. Exécutez {{command}} une fois pour vous connecter.",
        "providerProbe.unrecognizedModelCatalog" => "{{name}} a renvoyé un catalogue de modèles non reconnu",
        "promptCenter.title" => "Bibliothèque de prompts",
        "promptCenter.searchPlaceholder" => "Rechercher des prompts…",
        "promptCenter.category.all" => "Tous",
        "promptCenter.category.starter" => "Démarrage",
        "promptCenter.category.mobileApp" => "App mobile",
        "promptCenter.category.webPage" => "Page web",
        "promptCenter.category.dashboard" => "Tableau de bord",
        "promptCenter.category.component" => "Composant",
        "promptCenter.category.modify" => "Modification",
        "promptCenter.category.custom" => "Mes prompts",
        "promptCenter.empty" => "Aucun prompt correspondant",
        "promptCenter.saveCurrent" => "Enregistrer le texte actuel comme prompt",
        "promptCenter.saveTitlePlaceholder" => "Titre du prompt",
        "promptCenter.save" => "Enregistrer",
        "promptCenter.cancel" => "Annuler",
        "promptCenter.delete" => "Supprimer",
        "promptCenter.screens" => "{{count}} écrans",
        "promptCenter.freeform" => "Libre",
        "promptCenter.item.wander.title" => "Wander · Itinéraires de voyage",
        "promptCenter.item.forage.title" => "Forage · Recettes de saison",
        "promptCenter.item.still.title" => "Still · Méditation et sommeil",
        "promptCenter.item.hearth.title" => "Hearth · Maison connectée",
        "promptCenter.item.meteo.title" => "Meteo · Météo immersive",
        "promptCenter.item.marginalia.title" => "Marginalia · Lecture et annotations",
        "promptCenter.item.lingua.title" => "Lingua · Apprentissage des langues",
        "promptCenter.item.daybreak.title" => "Daybreak · Commande de café",
        "promptCenter.item.verdant.title" => "Verdant · Entretien des plantes",
        "promptCenter.item.companion.title" => "Companion · Vie avec son animal",
        "promptCenter.item.relic.title" => "Relic · Marché de seconde main",
        "promptCenter.item.nocturne.title" => "Nocturne · Guide d’observation des étoiles",
        "promptCenter.item.marquee.title" => "Marquee · Liste de films à voir",
        "promptCenter.item.ritual.title" => "Ritual · Création d’habitudes",
        "promptCenter.item.ember.title" => "Ember · Journal d’humeur",
        "promptCenter.item.volt.title" => "Volt · Compagnon pour véhicule électrique",
        "promptCenter.item.aloft.title" => "Aloft · Suivi des vols",
        "promptCenter.item.gallery.title" => "Gallery · Expositions et culture",
        "promptCenter.item.nightcap.title" => "Nightcap · Cocktails à la maison",
        "promptCenter.item.bloom.title" => "Bloom · Suivi de la croissance familiale",
        "promptCenter.item.extremeWeather.title" => "Extrême · App météo",
        "promptCenter.item.extremeNowPlaying.title" => "Extrême · À l’écoute",
        "promptCenter.item.extremeDailyApp.title" => "Extrême · À ouvrir chaque jour",
        "promptCenter.item.extremeCalendar.title" => "Extrême · Réinventer le calendrier",
        "promptCenter.item.extremeCalm.title" => "Extrême · Un écran de sérénité",
        "promptCenter.item.webOrbit.title" => "Orbit · Page d’accueil de l’espace de travail IA",
        "promptCenter.item.webAtelier.title" => "Atelier · E-commerce de mobilier",
        "promptCenter.item.dashboardPulse.title" => "Pulse · Tableau d’analyse de croissance",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · Opérations logistiques",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · Tableau de données d’entreprise",
        "promptCenter.item.componentFormLab.title" => {
            "Form Lab · Système de composants de formulaire"
        }
        "promptCenter.item.modifyPolishCurrent.title" => "Peaufiner l’écran actuel",
        "promptCenter.item.modifyCompleteStates.title" => "Compléter les états des composants",
        "collab.ownerConfirm.title" => "Confirmez qui vous rejoignez",
        "collab.ownerConfirm.hint" => "Rien de cette session n’a encore été chargé.",
        "collab.ownerConfirm.account" => "Compte vérifié",
        "collab.ownerConfirm.device" => "Appareil vérifié",
        "collab.ownerConfirm.claimedName" => "Nom choisi par ce compte (non vérifié)",
        "collab.action.confirmOwner" => "Rejoindre cette session",
        "collab.action.rejectOwner" => "Ne pas rejoindre",
        "collab.error.ownerNotConfirmed" => "Vous n’avez pas confirmé l’hôte, rien n’a été chargé.",
        "sceneTemplate.title" => "Modèles de scènes",
        "sceneTemplate.searchPlaceholder" => "Rechercher des scènes ou des modèles…",
        "sceneTemplate.empty" => "Aucun modèle correspondant",
        "sceneTemplate.frames" => "Pages : {{count}}",
        "sceneTemplate.filter.all" => "Tous",
        "sceneTemplate.scene.tutorial" => "Tutoriel illustré",
        "sceneTemplate.scene.comparison" => "Comparatif",
        "sceneTemplate.scene.carousel" => "Cartes de connaissances",
        "sceneTemplate.scene.slides" => "PPT",
        "sceneTemplate.item.screenshotTutorial.title" => "Tutoriel par captures d’écran · 3 étapes",
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "Une couverture, trois étapes et un appel à l’action final : remplacez les captures d’écran et les textes pour publier."
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "Carrousel de connaissances et d’idées",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "Une couverture, trois idées clés et une page de synthèse, pour décomposer un point de vue en cartes à faire défiler."
        }
        "sceneTemplate.item.beforeAfter.title" => "Comparatif avant/après",
        "sceneTemplate.item.beforeAfter.summary" => {
            "Comparaison avant/après côte à côte, accompagnée de notes sur les changements, idéale pour les rétrospectives et les portfolios."
        }
        "sceneTemplate.item.slideDeck.title" => "Présentation · 6 diapositives",
        "sceneTemplate.item.slideDeck.summary" => {
            "Couverture, sommaire, points clés, données, graphique et conclusion au format 16:9. Remplacez les textes et présentez."
        }
        "fileMenu.newFromTemplate" => "Nouveau à partir d’un modèle",
        _ => return super::fr_collab::lookup(key),
    })
}
