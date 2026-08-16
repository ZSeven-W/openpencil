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
        "promptCenter.item.webKilnform.title" => "Kilnform · Site d'infrastructure de design",
        "promptCenter.item.webReefwright.title" => "Reefwright · Site de connaissance support IA",
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
        "sceneTemplate.generate.placeholder" => "Décrivez un sujet, l'IA génère toute la présentation",
        "sceneTemplate.generate.button" => "Générer",
        "sceneTemplate.generate.hint" => "Un nouveau document, construit à partir de votre sujet en présentation complète.",
        "sceneTemplate.generate.promptTemplate" => "Crée une présentation (PPT) sur le sujet suivant : {{topic}}",
        "sceneTemplate.card.addToCanvas" => "Ajouter au canevas",
        "sceneTemplate.card.generateFrom" => "Générer d'après",
        "sceneTemplate.generate.basis" => "D'après : ",
        "sceneTemplate.filter.all" => "Tous",
        "sceneTemplate.scene.tutorial" => "Tutoriels",
        "sceneTemplate.scene.comparison" => "Comparaison",
        "sceneTemplate.scene.carousel" => "Carrousel",
        "sceneTemplate.scene.slides" => "Diapos",
        "sceneTemplate.scene.card" => "Cartes",
        "sceneTemplate.scene.web" => "Pages web",
        "sceneTemplate.generate.webPromptTemplate" => "Conçois une page d'atterrissage web en plusieurs sections sur le sujet suivant : {{topic}}",
        "sceneTemplate.item.saasLandingOrange.title" => "Page SaaS · Orange",
        "sceneTemplate.item.saasLandingOrange.summary" => "Une page marketing claire bâtie sur des panneaux noirs et un seul orange : navigation, hero avec capture produit, trois cartes de fonctionnalités, démonstration du flux, témoignages et pied de page d'abonnement. Changez les textes et c'est un site.",
        "sceneTemplate.item.productLandingLight.title" => "Page produit · Claire",
        "sceneTemplate.item.productLandingLight.summary" => "Une page produit blanc papier, d'allure éditoriale : démo interactive en hero, colonnes de fonctionnalités, tableau de bord, comparatif avant/après et trois offres tarifaires. Pour les sites SaaS et les lancements.",
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
        "sceneTemplate.item.knowledgeCardVertical.title" => "Carte de connaissances · Portrait",
        "sceneTemplate.item.knowledgeCardVertical.summary" => "Une carte 3:4 unique avec un titre, quatre points clés et une signature. Remplacez les textes et publiez.",
        "sceneTemplate.item.knowledgeCardSquare.title" => "Carte de connaissances · Carré",
        "sceneTemplate.item.knowledgeCardSquare.summary" => "Une carte 1:1 dans la même mise en page, assez compacte pour une image d’en-tête ou un partage social.",
        "sceneTemplate.item.pitchDeckDark.title" => "Pitch deck · Sombre",
        "sceneTemplate.item.pitchDeckDark.summary" => "Couverture, problème, solution, chiffres, feuille de route et page de contact. Grands caractères sur fond sombre, pensé pour la levée de fonds.",
        "sceneTemplate.item.lectureDeckLight.title" => "Support de cours · Clair",
        "sceneTemplate.item.lectureDeckLight.summary" => "Couverture, objectifs, explication d’un concept, exercice résolu, tableau comparatif et récapitulatif. Fond blanc papier, confortable sur toute une séance.",
        "sceneTemplate.item.minimalKeynote.title" => "Keynote minimaliste",
        "sceneTemplate.item.minimalKeynote.summary" => "Des blancs généreux, une typo surdimensionnée, une phrase centrée par page — neuf pages sans la moindre carte, et un sommaire réduit à des filets et des chiffres. Pour lancements et conférences.",
        "sceneTemplate.item.gradientTech.title" => "Tech dégradé",
        "sceneTemplate.item.gradientTech.summary" => "Fond dégradé sombre et cartes en verre dépoli : architecture, performances et mur de clients. Pour un lancement produit développeur.",
        "sceneTemplate.scene.infographic" => "Infographies",
        "sceneTemplate.item.punchQuoteCard.title" => "Carte citation · Affiche",
        "sceneTemplate.item.punchQuoteCard.summary" => "Une carte 3:4 sur fond presque noir : deux lignes en très grand au-dessus d'une bande jaune. Une seule phrase, pour les prises de position et les citations.",
        "sceneTemplate.item.journalChecklistCard.title" => "Carte à cocher · Base de connaissances",
        "sceneTemplate.item.journalChecklistCard.summary" => "Une carte blanche sur fond gris clair : cinq tâches à cocher, une étiquette et une citation. Pour les plans de la semaine.",
        "sceneTemplate.item.dataReportInfographic.title" => "Infographie de résultats",
        "sceneTemplate.item.dataReportInfographic.summary" => "Une image verticale à faire défiler : bandeau sombre, trois chiffres clés, un comparatif en barres, une répartition et trois conclusions. Changez les chiffres et publiez.",
        "sceneTemplate.item.stepsFlowInfographic.title" => "Infographie pas à pas",
        "sceneTemplate.item.stepsFlowInfographic.summary" => "Une image verticale à faire défiler : cinq étapes numérotées enchaînées, chacune avec sa durée, plus deux conseils. Pour les tutoriels et les guides.",
        "sceneTemplate.item.eventPosterDeck.title" => "Deck événement · Affiche",
        "sceneTemplate.item.eventPosterDeck.summary" => "Couverture, temps forts, programme, accès, billetterie et clôture. Fond blanc de galerie, aplats rouges et bleus, aucun angle arrondi et aucun dégradé — pour marchés, événements associatifs et ouvertures.",
        "sceneTemplate.item.pitfallListInfographic.title" => "Infographie des pièges à éviter",
        "sceneTemplate.item.pitfallListInfographic.summary" => "Une image verticale à faire défiler : six erreurs classées par fréquence, chacune avec ce qui cloche et quoi faire à la place, plus une vérification en quatre points avant publication. Noir, blanc et gris uniquement.",
        "sceneTemplate.item.spineCultureCard.title" => "Carte à titre vertical · Pigments minéraux",
        "sceneTemplate.item.spineCultureCard.summary" => "Une carte 3:4 sur fond d'argile ocre : titre chinois à la verticale, plâtre écaillé, grains de pigment. Pour la culture, les longs textes et les couvertures d'auteur.",
        "sceneTemplate.item.metricSingleCard.title" => "Carte à chiffre unique · Grille Hanzi",
        "sceneTemplate.item.metricSingleCard.summary" => "Une carte 1:1 : un chiffre énorme sur blanc pur, une grille suisse stricte et un seul carré rouge de signal. Pour les conclusions et les résultats.",
        "sceneTemplate.item.quoteFrameCard.title" => "Carte citation · Soie bleu-vert",
        "sceneTemplate.item.quoteFrameCard.summary" => "Une carte 4:5 sur soie jaunie : une phrase encadrée, une montagne d'azurite et de malachite en pied. Pour extraits, entretiens et citations.",
        "sceneTemplate.item.dailySignCard.title" => "Carte du jour · Fenêtre de jardin",
        "sceneTemplate.item.dailySignCard.summary" => "Une carte 3:4 sur mur à la chaux, percée d'une fenêtre hexagonale : la date et une ligne à l'intérieur. Le vide est le décor. Pour les publications quotidiennes.",
        "sceneTemplate.item.priceTierCard.title" => "Carte tarifs · Néons d'arcade",
        "sceneTemplate.item.priceTierCard.summary" => "Une carte 1:1 sur nuit bleu encre : trois niveaux de tarifs, contours de tubes néon et leur halo. Pour boutiques, événements et forfaits.",
        "sceneTemplate.item.noticeBoardCard.title" => "Carte d'annonce · Typographie plomb",
        "sceneTemplate.item.noticeBoardCard.summary" => "Une carte 4:5 sur papier journal : filets de manchette avec repérage décalé, articles numérotés et un cachet de série. Pour avis, règles et consignes.",
        "sceneTemplate.item.milestoneTimelineInfographic.title" => "Infographie chronologique",
        "sceneTemplate.item.milestoneTimelineInfographic.summary" => "Une image verticale à faire défiler : un axe traversant toute la hauteur, des repères d'années à côté des cartes d'étapes, et la suite en clôture. Pour bilans, histoire de marque et parcours de projet.",
        "sceneTemplate.item.conceptContrastInfographic.title" => {
            "Infographie de comparaison de concepts"
        }
        "sceneTemplate.item.conceptContrastInfographic.summary" => "Une image verticale à faire défiler : la conclusion d'abord, puis une carte de définition par concept, un tableau à deux colonnes par critère, et enfin comment choisir.",
        "sceneTemplate.item.rankingBoardInfographic.title" => "Infographie de classement Top N",
        "sceneTemplate.item.rankingBoardInfographic.summary" => "Une image verticale à faire défiler : un palmarès or sur encre — grands badges pour le trio de tête, badges filaires ensuite, avec l'usage et la fréquence.",
        "sceneTemplate.item.faqThreadInfographic.title" => "Infographie FAQ",
        "sceneTemplate.item.faqThreadInfographic.summary" => "Une image verticale à faire défiler : six paires question-réponse, Q pleine et R filaire. Ni numéros ni ordre : chaque paire tient seule.",
        "sceneTemplate.item.dataStoryInfographic.title" => "Infographie récit de données",
        "sceneTemplate.item.dataStoryInfographic.summary" => "Une image verticale à faire défiler : quatre chiffres enchaînés en une ligne causale, chaque étape en grille de dix blocs, et une conclusion applicable pour finir.",
        "sceneTemplate.item.challengeTrackerInfographic.title" => "Infographie défi 30 jours",
        "sceneTemplate.item.challengeTrackerInfographic.summary" => "Une image verticale à faire défiler : une grille de trente cases, six par cinq, avec des jalons aux jours 7, 15 et 30. À enregistrer et à cocher chaque jour.",
        "sceneTemplate.item.ecosystemMapInfographic.title" => "Infographie carte d'écosystème",
        "sceneTemplate.item.ecosystemMapInfographic.summary" => "Une image verticale à faire défiler : quatre positions d'une même chaîne en deux par deux, trois acteurs par case, et les vides signalés. Cartes blanches sur ardoise.",
        "sceneTemplate.item.doDontComparison.title" => "Deux colonnes à faire / à éviter",
        "sceneTemplate.item.doDontComparison.summary" => "Une carte 3:4 : deux façons de faire la même chose côte à côte, distinguées par la matière et l'icône plutôt que par le rouge et le vert, lisible aussi pour les daltoniens.",
        "sceneTemplate.item.mythTruthComparison.title" => "Idées reçues et réalité",
        "sceneTemplate.item.mythTruthComparison.summary" => "Une image haute : cinq paires « ce qu'on dit / ce qui est vrai », l'idée reçue étroite et pâle à gauche, la réalité large et sombre à droite, une paire à la fois.",
        "sceneTemplate.item.pricingTiersComparison.title" => "Comparatif de tarifs",
        "sceneTemplate.item.pricingTiersComparison.summary" => "Une carte 3:4 : Gratuit, Pro et Équipe côte à côte, le prix comme point d'ancrage, chaque colonne contenant la précédente. Pour les pages tarifs.",
        "sceneTemplate.item.scenarioGuideComparison.title" => "Guide de choix par situation",
        "sceneTemplate.item.scenarioGuideComparison.summary" => "Une image haute : pas de spécifications, sept situations, chacune assortie d'un verdict. Le lecteur n'a qu'à trouver sa ligne.",
        "sceneTemplate.item.specTableComparison.title" => "Tableau comparatif de spécifications",
        "sceneTemplate.item.specTableComparison.summary" => "Une image haute : deux candidats dans un vrai tableau, ligne par ligne, la cellule gagnante relevée par un fond sombre.",
        "sceneTemplate.item.threeWayComparison.title" => "Comparatif à trois options",
        "sceneTemplate.item.threeWayComparison.summary" => "Une image haute : trois options côte à côte, la recommandation au centre ; chaque colonne s'ouvre sur une situation plutôt que sur un nom.",
        "sceneTemplate.item.timeShiftComparison.title" => "Avant / maintenant à un an d'écart",
        "sceneTemplate.item.timeShiftComparison.summary" => "Une carte 3:4 : une colonne vertébrale d'étiquettes au centre, il y a un an à gauche, aujourd'hui à droite, les deux valeurs sur la même ligne.",
        "sceneTemplate.item.tradeoffScaleComparison.title" => "Balance avantages / inconvénients",
        "sceneTemplate.item.tradeoffScaleComparison.summary" => "Une carte 1:1 : un fléau, deux plateaux — ce que ça vaut à gauche, ce que ça coûte à droite, une case vide devant chaque ligne. À vous de peser.",
        "sceneTemplate.item.versionDiffComparison.title" => "Différences entre versions",
        "sceneTemplate.item.versionDiffComparison.summary" => "Une carte 1:1 : pas de colonnes — chaque ligne accomplit son propre « avant → après », il suffit de faire défiler.",
        "sceneTemplate.item.appOnboardingTriptych.title" => "Triptyque d'onboarding d'app",
        "sceneTemplate.item.appOnboardingTriptych.summary" => "Une carte 3:4 : trois téléphones alignés avec des emplacements d'image vides. Déposez vos trois écrans, ajoutez le texte, c'est prêt pour la revue ou la publication.",
        "sceneTemplate.item.diyBlueprintGuide.title" => "Guide DIY illustré",
        "sceneTemplate.item.diyBlueprintGuide.summary" => "Une image haute où le tableau des matériaux occupe autant de place que les étapes — le bricolage échoue à la préparation, pas dans les mains.",
        "sceneTemplate.item.photoCompositionTutorial.title" => "Composition photo au téléphone",
        "sceneTemplate.item.photoCompositionTutorial.summary" => "3:4, cinq vues : chacune un viseur sombre et des repères fluo par-dessus l'emplacement photo — la composition ne s'explique que sur le cadre.",
        "sceneTemplate.item.recipeFourStep.title" => "Recette en quatre étapes",
        "sceneTemplate.item.recipeFourStep.summary" => "Une carte 4:5 en 2×2 : les quatre étapes sur une seule carte. Capture d'écran et on cuisine — devant les fourneaux, tourner les pages est un fardeau.",
        "sceneTemplate.item.skincareRoutineCards.title" => "Cartes de routine soin",
        "sceneTemplate.item.skincareRoutineCards.summary" => "4:5, six vues : chaque étape fixe trois nombres — la dose, le temps de pose, matin ou soir. Les ratés viennent de la dose, pas de l'ordre.",
        "sceneTemplate.item.softwareStepTutorial.title" => "Tutoriel logiciel pas à pas",
        "sceneTemplate.item.softwareStepTutorial.summary" => "Une carte 4:5, la seule sombre de la série : emplacements de captures et consignes numérotées, pour les outils et les fonctionnalités.",
        "sceneTemplate.item.storageMakeoverSteps.title" => "Étapes de rangement",
        "sceneTemplate.item.storageMakeoverSteps.summary" => "3:4, six vues : outre le geste et l'image, chaque étape fixe un critère d'achèvement et un budget temps — c'est fini quand ça ressemble à ça.",
        "sceneTemplate.item.weeklyReportLesson.title" => "Leçon de rapport hebdomadaire",
        "sceneTemplate.item.weeklyReportLesson.summary" => "Une image haute : après la structure en quatre temps, elle livre un squelette à trous soulignés — capture et remplissez.",
        "sceneTemplate.item.workoutBreakdownGuide.title" => "Guide de décomposition d'exercices",
        "sceneTemplate.item.workoutBreakdownGuide.summary" => "Une image haute : chaque mouvement porte une barre fixe séries / répétitions / repos à côté de l'image et des consignes.",
        "sceneTemplate.item.bookreviewSilkCarousel.title" => "Carrousel d'analyse livre / film",
        "sceneTemplate.item.bookreviewSilkCarousel.summary" => "3:4, cinq planches : accroche, extrait annoté, trois idées, une phrase à citer, clôture. On démonte l'œuvre en pièces à emporter plutôt que de résumer l'intrigue.",
        "sceneTemplate.item.cityguideFilmCarousel.title" => "Carrousel guide de ville",
        "sceneTemplate.item.cityguideFilmCarousel.summary" => "3:4, sept planches : lieux et itinéraires alternent — les lieux pour ceux qui rêvent, la journée type et le tableau manger-dormir pour ceux qui planifient.",
        "sceneTemplate.item.datareportGridCarousel.title" => "Carrousel rapport de données",
        "sceneTemplate.item.datareportGridCarousel.summary" => "3:4, six planches : chaque planche de données est suivie d'une planche sans données, pour que personne ne décroche au troisième graphique.",
        "sceneTemplate.item.opinionLongformCarousel.title" => "Carrousel d'opinion longue",
        "sceneTemplate.item.opinionLongformCarousel.summary" => "3:4, six planches : une maquette stricte de bout en bout, numéro et titre toujours au même endroit. Sur un carrousel, la planche précédente a disparu.",
        "sceneTemplate.item.qaChalkboardCarousel.title" => "Carrousel questions-réponses",
        "sceneTemplate.item.qaChalkboardCarousel.summary" => "3:4, six planches : une question par planche, avec un numéro-point d'interrogation dessiné à la main dans le coin.",
        "sceneTemplate.item.storyNightCarousel.title" => "Carrousel récit",
        "sceneTemplate.item.storyNightCarousel.summary" => "3:4, sept planches : un retour d'expérience bâti sur le temps — la frise de la cinquième planche est le mur porteur.",
        "sceneTemplate.item.toolkitNotebookCarousel.title" => "Carrousel boîte à outils",
        "sceneTemplate.item.toolkitNotebookCarousel.summary" => "3:4, six planches : six outils déployés un par planche, la dernière les listant avec leurs numéros — le lecteur vient pour enregistrer.",
        "sceneTemplate.item.tutorialJournalCarousel.title" => "Carrousel tutoriel",
        "sceneTemplate.item.tutorialJournalCarousel.summary" => "3:4, six planches : une étape par planche, le doigt fait office de barre de progression.",
        "sceneTemplate.item.yearreviewMineralCarousel.title" => "Carrousel bilan de l'année",
        "sceneTemplate.item.yearreviewMineralCarousel.summary" => "3:4, huit planches : planches chiffrées froides et planches de ressenti chaudes, en alternance.",
        "fileMenu.newFromTemplate" => "Nouveau à partir d’un modèle",
        "fileMenu.exportSlideshowHtml" => "Exporter le diaporama HTML...",
        "fileMenu.exportPptx" => "Exporter en PowerPoint...",
        "dialog.slideshowHtmlTitle" => "Exporter le diaporama",
        "dialog.slideshowHtmlSummary" => "{{count}} diapositives exportées vers :",
        "dialog.slideshowHtmlEmpty" => "Cette présentation n'a aucune diapositive visible à exporter.",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "Le contenu HTML importable est indisponible.",
        "htmlImport.warn.content.empty_body" => {
            "Le contenu importable du corps HTML est indisponible."
        }
        "htmlImport.warn.content.dom_depth_truncated" => {
            "Le HTML imbriqué au-delà de {{max_depth}} niveaux a été supprimé."
        }
        "htmlImport.warn.content.node_limit_truncated" => {
            "Limite de nœuds atteinte : le reste du contenu de la page a été omis."
        }
        "htmlImport.warn.content.node_limit_mapping" => {
            "Limite de nœuds atteinte : une partie de l'arbre HTML a été omise."
        }
        "htmlImport.warn.content.node_limit_inline_row" => {
            "Limite de nœuds atteinte : une ligne de mise en forme en ligne a été omise."
        }
        "htmlImport.warn.content.node_limit_pseudo" => {
            "Limite de nœuds atteinte : les pseudo-éléments générés ont été omis."
        }
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "Les règles CSS imbriquées au-delà de {{max_depth}} règles @ ont été ignorées."
        }
        "htmlImport.warn.css.unterminated_rule" => "Une règle CSS non terminée a été ignorée.",
        "htmlImport.warn.css.marker_rules_unsupported" => {
            "Les règles CSS ::marker n'ont pas été importées."
        }
        "htmlImport.warn.css.nesting_unsupported" => {
            "Les règles de style CSS imbriquées ont été ignorées."
        }
        "htmlImport.warn.css.invalid_layer_name" => {
            "Le nom @layer invalide '{{name}}' a été ignoré."
        }
        "htmlImport.warn.css.unsupported_statement" => {
            "L'instruction @{{name}} non prise en charge a été ignorée."
        }
        "htmlImport.warn.css.media_without_viewport" => {
            "Les règles @media sans fenêtre d'affichage ont été ignorées."
        }
        "htmlImport.warn.css.invalid_layer_block_name" => {
            "Le nom de bloc @layer invalide '{{name}}' a été ignoré."
        }
        "htmlImport.warn.css.unsupported_container_block" => "Le bloc @container a été ignoré.",
        "htmlImport.warn.css.unsupported_block" => {
            "Le bloc @{{name}} non pris en charge a été ignoré."
        }
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "La police web @font-face '{{family}}' est indisponible."
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "Les décalages en pourcentage d'un élément positionné en absolu ont été approximés."
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "Les décalages position:relative en pourcentage ont été approximés."
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "La propriété CSS aspect-ratio sans axe défini a été ignorée."
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "La propriété CSS aspect-ratio dans un bloc conteneur indéfini a été ignorée."
        }
        "htmlImport.warn.layout.position_sticky_ignored" => {
            "La propriété CSS position:sticky a été ignorée."
        }
        "htmlImport.warn.layout.grid_tracks_approximated" => {
            "Les pistes de grille CSS non prises en charge ont été approximées."
        }
        "htmlImport.warn.layout.float_ignored" => "La propriété CSS float a été ignorée.",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "La propriété CSS mix-blend-mode au niveau du nœud a été approximée."
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "La propriété CSS overflow: auto / scroll a été approximée."
        }
        "htmlImport.warn.layout.negative_margins_ignored" => {
            "Les marges CSS négatives ont été ignorées."
        }
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => {
            "Les marges CSS sur une boîte visuelle ont été ignorées."
        }
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "Le dimensionnement en pourcentage content-box a été approximé."
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "Les cellules de grille CSS vides laissées par des lignes de départ explicites ont été approximées."
        }
        "htmlImport.warn.layout.grid_span_reflowed" => {
            "Un élément de grille CSS dont l'étendue ne tenait pas sur sa ligne de départ a été approximé."
        }
        "htmlImport.warn.layout.grid_rows_node_limit" => {
            "Limite de nœuds atteinte : les enveloppes de lignes de grille CSS ont été omises."
        }
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "Les largeurs de pistes de grille CSS utilisant auto-fit / auto-fill ont été approximées."
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "Le placement CSS grid-template-areas n'a pas été importé."
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => {
            "Le placement CSS grid-row n'a pas été importé."
        }
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "La valeur CSS grid-column `{{value}}` a été approximée."
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => {
            "Les marges auto CSS sur l'axe de bloc n'ont pas été importées."
        }
        "htmlImport.warn.layout.auto_margin_node_limit" => {
            "Limite de nœuds atteinte : l'alignement CSS par marges auto a été omis."
        }
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "Un décalage CSS dans le flux sur un élément sans taille définie a été supprimé."
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => {
            "Limite de nœuds atteinte : un décalage CSS dans le flux a été omis."
        }
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "Les décalages CSS dans le flux (retraits position:relative, translation transform) ont été approximés."
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "Un décalage CSS dans le flux sur une boîte ne pouvant héberger d'enveloppe de décalage a été supprimé."
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "flex-wrap sur un conteneur flex en colonne n'a pas été importé."
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => {
            "flex-wrap:wrap-reverse a été approximé."
        }
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "flex-wrap sur un conteneur sans largeur définie a été ignoré."
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "La propriété CSS align-content sur un conteneur flex multiligne n'a pas été importée."
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "flex-wrap avec des tailles d'enfants indéterminées sur l'axe principal a été ignoré."
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => {
            "Limite de nœuds atteinte : les lignes flex-wrap ont été omises."
        }
        "htmlImport.warn.transform.unsupported_syntax" => {
            "La syntaxe CSS transform non prise en charge a été ignorée."
        }
        "htmlImport.warn.transform.unsupported_function" => {
            "Les fonctions CSS transform non prises en charge (3D, matrix3d) ont été ignorées."
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "Une translation CSS transform en pourcentage sur un axe indéfini a été supprimée."
        }
        "htmlImport.warn.transform.non_finite_matrix" => {
            "Une transformation CSS produisant une matrice non finie a été ignorée."
        }
        "htmlImport.warn.transform.skew_dropped" => "L'inclinaison CSS transform a été supprimée.",
        "htmlImport.warn.transform.degenerate_scale" => {
            "Une transformation CSS avec une échelle nulle ou non finie a été approximée."
        }
        "htmlImport.warn.transform.mirroring_absolute" => {
            "La symétrie CSS transform a été approximée."
        }
        "htmlImport.warn.transform.origin_z_ignored" => {
            "Le décalage Z de la propriété CSS transform-origin a été ignoré."
        }
        "htmlImport.warn.transform.scale_not_baked" => {
            "Une mise à l'échelle CSS transform non intégrable à la taille du nœud a été supprimée."
        }
        "htmlImport.warn.transform.scale_baked" => {
            "La mise à l'échelle CSS transform intégrée à la taille du nœud a été approximée."
        }
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "La mise à l'échelle CSS transform sur un élément dimensionné automatiquement a été ignorée."
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "La propriété CSS background-repeat directionnelle ou espacée a été approximée."
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => {
            "Une taille de tuile d'arrière-plan CSS explicite a été ignorée."
        }
        "htmlImport.warn.visual.background_size_auto_box" => {
            "La propriété CSS background-size sur un élément dimensionné automatiquement a été approximée."
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "La propriété CSS background-size nécessitant la taille intrinsèque de l'image a été approximée."
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "Une valeur CSS background-position non prise en charge a été ignorée."
        }
        "htmlImport.warn.visual.background_image_url_empty" => {
            "Une URL d'image d'arrière-plan CSS vide a été ignorée."
        }
        "htmlImport.warn.visual.conic_gradient_ignored" => {
            "Les dégradés coniques CSS ont été ignorés."
        }
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "Une couche CSS background-image non prise en charge a été ignorée."
        }
        "htmlImport.warn.visual.background_color_unresolved" => {
            "Une couleur d'arrière-plan CSS non résolue a été ignorée."
        }
        "htmlImport.warn.visual.background_position_dropped" => {
            "La propriété CSS background-position a été ignorée."
        }
        "htmlImport.warn.visual.border_colors_approximated" => {
            "Les couleurs de bordure CSS par côté ont été approximées."
        }
        "htmlImport.warn.visual.border_styles_approximated" => {
            "Les styles de bordure CSS mixtes par côté ont été approximés."
        }
        "htmlImport.warn.visual.border_style_complex" => {
            "Un style de bordure CSS complexe a été approximé."
        }
        "htmlImport.warn.visual.border_style_unsupported" => {
            "Un style de bordure CSS non pris en charge a été approximé."
        }
        "htmlImport.warn.visual.border_radius_elliptical" => {
            "Les rayons de bordure CSS elliptiques ont été approximés."
        }
        "htmlImport.warn.visual.border_radius_unsupported" => {
            "Un rayon de bordure CSS non pris en charge a été ignoré."
        }
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "Une couche CSS box-shadow non prise en charge a été ignorée."
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => {
            "La méthode d'interpolation des couleurs du dégradé CSS a été ignorée."
        }
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "Une direction CSS linear-gradient non prise en charge a été ignorée."
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => {
            "Les repères de couleur des dégradés CSS ont été ignorés."
        }
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => {
            "Un arrêt de couleur de dégradé CSS non pris en charge a été ignoré."
        }
        "htmlImport.warn.visual.gradient_too_few_stops" => {
            "Un dégradé CSS comptant moins de deux arrêts utilisables a été ignoré."
        }
        "htmlImport.warn.visual.gradient_repeating_approximated" => {
            "Un dégradé CSS répétitif a été approximé."
        }
        "htmlImport.warn.visual.gradient_stops_clamped" => {
            "Les arrêts de dégradé CSS hors plage ont été approximés."
        }
        "htmlImport.warn.visual.blur_radius_unsupported" => {
            "Un rayon de flou CSS non pris en charge a été ignoré."
        }
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "Un filtre CSS drop-shadow() non pris en charge a été ignoré."
        }
        "htmlImport.warn.visual.filter_function_unsupported" => {
            "Une fonction de filtre CSS non prise en charge a été ignorée."
        }
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "Une fonction CSS backdrop-filter non prise en charge a été ignorée."
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "Une valeur CSS background-blend-mode non prise en charge a été ignorée."
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "La propriété CSS mix-blend-mode sur des remplissages individuels a été approximée."
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "Une valeur CSS mix-blend-mode non prise en charge a été ignorée."
        }
        "htmlImport.warn.visual.property_not_representable" => {
            "La propriété CSS {{property}} a été ignorée."
        }
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "La propriété CSS background-size sur un dégradé a été ignorée."
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "Une position CSS radial-gradient non prise en charge a été ignorée."
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "Un dégradé CSS radial-gradient elliptique a été approximé."
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "Un mot-clé d'étendue CSS radial-gradient a été approximé."
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "Une taille CSS radial-gradient non prise en charge a été ignorée."
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => {
            "Une couche CSS text-shadow non prise en charge a été ignorée."
        }
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "Les couches CSS text-shadow au-delà de la première ont été ignorées."
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => {
            "La propriété CSS text-shadow sur un élément en ligne a été ignorée."
        }
        "htmlImport.warn.list.style_image_ignored" => {
            "La propriété CSS list-style-image n'a pas été importée."
        }
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "Un marqueur suspendu `list-style-position: outside` a été approximé."
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "La valeur CSS list-style-type `{{value}}` non prise en charge a été approximée."
        }
        "htmlImport.warn.media.object_fit_scale_down" => {
            "La propriété CSS object-fit:scale-down a été approximée."
        }
        "htmlImport.warn.media.object_fit_none_ignored" => {
            "La propriété CSS object-fit:none a été ignorée."
        }
        "htmlImport.warn.media.object_position_ignored" => {
            "La propriété CSS object-position a été ignorée."
        }
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "Une valeur CSS mix-blend-mode non prise en charge sur une image a été ignorée."
        }
        "htmlImport.warn.media.inline_svg_placeholder" => {
            "Un élément <svg> en ligne a été importé comme espace réservé."
        }
        "htmlImport.warn.media.input_type_fallback" => {
            "Un type <input> non pris en charge a été approximé."
        }
        "htmlImport.warn.media.element_placeholder" => {
            "L'élément <{{tag}}> a été importé comme espace réservé."
        }
        "htmlImport.warn.media.picture_undecodable_types" => {
            "Un <picture> dont toutes les sources sont de types non décodables a été approximé."
        }
        "htmlImport.warn.table.rowspan_ignored" => "L'attribut HTML rowspan n'a pas été importé.",
        "htmlImport.warn.table.row_groups_unflattened" => {
            "Les largeurs de colonnes d'un tableau dont les groupes de lignes ont été dé-aplatis par CSS ont été approximées."
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "Les largeurs de colonnes d'un tableau CSS sans largeur définie ont été approximées."
        }
        "htmlImport.warn.resource.invalid_base_href" => {
            "Le <base href> invalide {{href}} a été ignoré."
        }
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "Le <base href> {{href}} hors de l'origine du projet a été ignoré."
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => {
            "La feuille de style externe {{url}} est indisponible."
        }
        "htmlImport.warn.resource.image_outside_origin" => {
            "L'image {{url}} hors de l'origine du projet a été importée comme espace réservé."
        }
        "htmlImport.warn.resource.image_unavailable" => {
            "L'image indisponible {{url}} a été importée comme espace réservé."
        }
        "htmlImport.warn.resource.css_import_invalid" => {
            "Le @import CSS invalide {{prelude}} a été ignoré."
        }
        "htmlImport.warn.resource.css_import_unresolvable" => {
            "Le @import CSS {{reference}} est indisponible."
        }
        "htmlImport.warn.resource.css_import_cycle" => {
            "Le @import CSS cyclique {{url}} a été ignoré."
        }
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "Le @import CSS {{url}} au-delà de la profondeur {{max_depth}} a été ignoré."
        }
        "htmlImport.warn.resource.css_import_unavailable" => {
            "Le @import CSS {{url}} est indisponible."
        }
        "htmlImport.warn.project.multiple_html_entries" => {
            "{{count}} points d'entrée HTML trouvés ; {{entry}} a été retenu et les autres ont été approximés."
        }
        "htmlImport.warn.snapshot.truncated" => {
            "Une partie de la capture du navigateur a été supprimée."
        }
        "htmlImport.warn.snapshot.node_limit" => {
            "Limite de nœuds atteinte : le reste du contenu de la capture a été omis."
        }
        "htmlImport.warn.snapshot.tainted_images" => {
            "{{count}} images altérées par CORS, conservées en URL distantes, sont indisponibles."
        }
        "htmlImport.warn.snapshot.invalid_rect" => {
            "Un nœud de capture au rectangle manquant ou invalide a été supprimé."
        }
        "htmlImport.warn.snapshot.unknown_kind" => {
            "Un nœud de capture de type inconnu a été supprimé."
        }
        "htmlImport.warn.snapshot.rejected" => {
            "La capture du navigateur ({{reason}}) a été supprimée."
        }
        "htmlImport.warn.snapshot.unsupported_transform" => {
            "Une transformation de capture non prise en charge a été ignorée."
        }
        "htmlImport.warn.css.media_empty_query" => "Une requête @media vide a été ignorée.",
        "htmlImport.warn.css.media_unsupported_type" => {
            "Le type @media '{{name}}' non pris en charge a été ignoré."
        }
        "htmlImport.warn.css.media_unsupported_condition" => {
            "La condition @media '{{input}}' non prise en charge a été ignorée."
        }
        "htmlImport.warn.css.media_invalid_orientation" => {
            "L'orientation @media invalide '{{value}}' a été ignorée."
        }
        "htmlImport.warn.css.media_unsupported_feature" => {
            "La caractéristique @media '{{name}}' non prise en charge a été ignorée."
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "La plage @media '({{input}})' non prise en charge a été ignorée."
        }
        "htmlImport.warn.css.media_invalid_range" => {
            "La plage @media invalide '({{input}})' a été ignorée."
        }
        "htmlImport.warn.css.media_invalid_length" => {
            "La longueur @media invalide '{{value}}' a été ignorée."
        }
        "htmlImport.diagnostics.title" => "Import HTML terminé",
        "htmlImport.diagnostics.summary" => "Éléments dégradés : {{count}}",
        "htmlImport.diagnostics.dismiss" => "Fermer",
        "htmlImport.diagnostics.expand" => "Afficher les détails",
        "htmlImport.diagnostics.collapse" => "Masquer les détails",
        "htmlImport.diagnostics.more" => "+{{count}} autres",
        "dialog.pptxTitle" => "Exporter en PowerPoint",
        "dialog.pptxSummary" => "{{count}} diapositives exportées vers :",
        "dialog.pptxEmpty" => "Cette présentation n'a aucune diapositive visible à exporter.",
        "settings.agents.acpQuickAdd" => "Ajout rapide",
        "settings.agents.acpPresetAdd" => "Ajouter",
        "settings.agents.acpNotInstalled" => "Non installé",
        "assetCenter.title" => "Centre de ressources",
        "assetCenter.tab.templates" => "Modèles",
        "assetCenter.tab.styles" => "Styles",
        "assetCenter.style.empty" => "Aucun style correspondant",
        "assetCenter.style.pinned" => "Épinglé",
        "assetCenter.style.searchPlaceholder" => "Rechercher des styles ou des tags",
        "assetCenter.style.generateHint" => "Un nouveau document créé à partir de votre sujet, dans le style épinglé.",
        "ai.pinnedStyle" => "Style : {{name}}",
        "assetCenter.style.import" => "Importer un style",
        "assetCenter.style.mine" => "Mes styles",
        "assetCenter.style.builtIn" => "Styles intégrés",
        "assetCenter.style.importTitle" => "Importer un DESIGN.md",
        "assetCenter.style.importHint" => "Collez l'intégralité du DESIGN.md, puis confirmez l'import.",
        "assetCenter.style.importSource" => "Vous pouvez copier un style depuis une bibliothèque DESIGN.md comme styles.refero.design.",
        "assetCenter.style.importConfirm" => "Importer",
        "assetCenter.style.importCancel" => "Annuler",
        "assetCenter.style.importPickFile" => "Parcourir…",
        "assetCenter.style.importHintFile" => "Choisissez un fichier DESIGN.md, ou collez le document entier ci-dessous.",
        "assetCenter.style.importPlaceholder" => "Collez votre DESIGN.md ici",
        "assetCenter.style.importEmpty" => "Ce fichier est vide, ou trop court pour être un guide de style.",
        "assetCenter.style.importNotText" => "Ce fichier n'est pas lisible comme du texte Markdown.",
        "assetCenter.style.importTooLarge" => "Ce fichier dépasse 512 Ko.",
        "slidesPanel.tabSlides" => "Diapositives",
        "slidesPanel.tabCards" => "Cartes",
        "slidesPanel.present" => "Présenter",
        "slidesPanel.exportPdf" => "Exporter en PDF",
        "slidesPanel.exportAllSlides" => "Exporter toutes les diapositives",
        "slidesPanel.exportSelectedSlides" => "Exporter les diapositives sélectionnées ({{count}})",
        "settings.tab.ai" => "IA",
        "settings.agents.heroTitle" => "Connectez votre fournisseur d'IA",
        "settings.agents.heroSubtitle" => "OpenPencil pilote vos agents CLI locaux et vos fournisseurs d'API — connectez-en un pour générer des designs.",
        "settings.agents.statusConnected" => "Connecté",
        "settings.agents.statusNotConnected" => "Non connecté",
        "settings.agents.statusChecking" => "Vérification…",
        "settings.mcp.heroTitle" => "Connecter OpenPencil via MCP depuis l'extérieur",
        "settings.mcp.heroSubtitle" => "Pointez n'importe quel CLI ou éditeur compatible MCP vers cet espace de travail et pilotez le canevas avec les outils de l'agent intégré.",
        "settings.mcp.terminalFootnote" => "* Au démarrage, MCP est configuré automatiquement pour les outils CLI sélectionnés.",
        "settings.mcp.customConfigTitle" => "Configuration serveur MCP personnalisée",
        "settings.mcp.customConfigDesc" => "Collez ceci dans tout client qui lit un bloc de serveur MCP standard.",
        "settings.mcp.copyConfig" => "Copier la config MCP",
        "settings.system.heroTitle" => "Préférences système",
        "settings.system.heroSubtitle" => "Apparence, mises à jour et comportement du canevas pour cette installation.",
        "settings.system.appearance" => "Apparence",
        "settings.system.appearanceLight" => "Clair",
        "settings.system.appearanceDark" => "Sombre",
        "settings.system.pencilCursor" => "Curseur crayon",
        "settings.images.heroTitle" => "Des images pour vos designs",
        "settings.images.heroSubtitle" => "Cherchez des photos sur Openverse, ou connectez un fournisseur pour en générer à la demande.",
        "settings.fonts.heroTitle" => "Polices de ce document",
        "settings.fonts.heroSubtitle" => "Résolvez les polices demandées par un document mais absentes de cette machine, et gérez celles que vous avez importées.",
        "settings.account.heroTitle" => "Votre compte",
        "settings.account.heroSubtitle" => "Connectez-vous pour synchroniser votre espace de travail et votre licence entre appareils.",
        "tooltip.topbar.file" => "Fichier",
        "tooltip.topbar.import" => "Importer",
        "tooltip.topbar.language" => "Langue",
        "tooltip.topbar.collaboration" => "Collaboration",
        "tooltip.topbar.preview" => "Aperçu",
        "tooltip.topbar.exitPreview" => "Quitter l’aperçu",
        "tooltip.topbar.account" => "Compte",
        "settings.agents.providerRollMore" => "et {{count}} autres",
        "ai.thinking.adaptive" => "Réflexion : auto",
        "ai.thinking.disabled" => "Réflexion : désactivée",
        "ai.thinking.enabled" => "Réflexion : activée",
        "ai.designProgress.detail.repairsApplied" => "{{count}} correction(s) automatique(s) appliquée(s)",
        "ai.designProgress.detail.repairsMore" => "… et {{count}} de plus (voir le journal)",
        "ai.styleCard.builtin" => "Style intégré",
        "ai.styleCard.imported" => "DESIGN.md importé",
        "ai.styleCard.documentDesignMd" => "design.md du document",
        _ => return super::fr_collab::lookup(key),
    })
}
