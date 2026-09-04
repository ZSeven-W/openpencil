//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `en_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Search images...",
        "imagePanel.searching" => "Searching...",
        "imagePanel.noResults" => "No results found",
        "imagePanel.searchPrompt" => "Search for images",
        "imagePanel.sourceNotice" => {
            "Images from {{source}}. Freely licensed — verify license before use."
        }
        "imagePanel.genNotConfigured" => "Image generation not configured",
        "imagePanel.openSettings" => "Open Settings",
        "imagePanel.promptPlaceholder" => "Describe the image...",
        "providerProbe.connectedViaCli" => "Connected via {{name}} CLI",
        "providerProbe.cliExitedWithError" => "{{name}} CLI exited with an error",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI produced no version output",
        "providerProbe.modelQueryFailed" => "{{name}} model query failed or timed out",
        "providerProbe.modelQueryFailedRunLogin" => {
            "{{name}} model query failed. Run {{command}} once to authenticate."
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "{{name}} model query requires authentication. Run {{command}} once to sign in."
        }
        "providerProbe.unrecognizedModelCatalog" => {
            "{{name}} returned an unrecognized model catalog"
        }
        "providerProbe.connectedAs" => "Connected as @{{login}}{{method}}",
        "providerProbe.connectedViaGithub" => "Connected via GitHub",
        "importProgress.figmaTitle" => "Parsing Figma file…",
        "importProgress.htmlTitle" => "Parsing HTML and page resources…",
        "importProgress.htmlSubtitle" => "Loading styles and images. Please wait.",
        "importProgress.largeFileSubtitle" => "Large files take a few seconds. Please wait.",
        "account.signedOutHint" => "Sign in to sync your settings and preferences",
        "code.noUsableCode" => "The AI returned no usable code. Retry or switch AI models.",
        "code.previousResultKept" => "The previous generated result is still available",
        "promptCenter.title" => "Prompt Center",
        "promptCenter.searchPlaceholder" => "Search prompts…",
        "promptCenter.category.all" => "All",
        "promptCenter.category.starter" => "Starter",
        "promptCenter.category.mobileApp" => "Mobile Apps",
        "promptCenter.category.webPage" => "Web Pages",
        "promptCenter.category.dashboard" => "Dashboards",
        "promptCenter.category.component" => "Components",
        "promptCenter.category.modify" => "Modify",
        "promptCenter.category.custom" => "Mine",
        "promptCenter.empty" => "No matching prompts",
        "promptCenter.saveCurrent" => "Save current input",
        "promptCenter.saveTitlePlaceholder" => "Prompt title",
        "promptCenter.save" => "Save",
        "promptCenter.cancel" => "Cancel",
        "promptCenter.delete" => "Delete",
        "promptCenter.screens" => "{{count}} screens",
        "promptCenter.freeform" => "Freeform",
        "promptCenter.item.wander.title" => "Wander · Travel Itinerary",
        "promptCenter.item.forage.title" => "Forage · Seasonal Recipes",
        "promptCenter.item.still.title" => "Still · Meditation & Sleep",
        "promptCenter.item.hearth.title" => "Hearth · Smart Home",
        "promptCenter.item.meteo.title" => "Meteo · Immersive Weather",
        "promptCenter.item.marginalia.title" => "Marginalia · Reading & Annotation",
        "promptCenter.item.lingua.title" => "Lingua · Language Learning",
        "promptCenter.item.daybreak.title" => "Daybreak · Coffee Ordering",
        "promptCenter.item.verdant.title" => "Verdant · Plant Care",
        "promptCenter.item.companion.title" => "Companion · Pet Life",
        "promptCenter.item.relic.title" => "Relic · Curated Resale",
        "promptCenter.item.nocturne.title" => "Nocturne · Stargazing Guide",
        "promptCenter.item.marquee.title" => "Marquee · Movie Watchlist",
        "promptCenter.item.ritual.title" => "Ritual · Habit Building",
        "promptCenter.item.ember.title" => "Ember · Mood Journal",
        "promptCenter.item.volt.title" => "Volt · EV Companion",
        "promptCenter.item.aloft.title" => "Aloft · Flight Tracking",
        "promptCenter.item.gallery.title" => "Gallery · Exhibitions & Culture",
        "promptCenter.item.nightcap.title" => "Nightcap · Home Bartending",
        "promptCenter.item.bloom.title" => "Bloom · Family Growth Tracker",
        "promptCenter.item.extremeWeather.title" => "Extreme · Weather App",
        "promptCenter.item.extremeNowPlaying.title" => "Extreme · Now Playing",
        "promptCenter.item.extremeDailyApp.title" => "Extreme · Everyday App",
        "promptCenter.item.extremeCalendar.title" => "Extreme · Calendar",
        "promptCenter.item.extremeCalm.title" => "Extreme · Calm",
        "promptCenter.item.webOrbit.title" => "Orbit · AI Workbench Landing Page",
        "promptCenter.item.webAtelier.title" => "Atelier · Furniture Commerce",
        "promptCenter.item.webKilnform.title" => "Kilnform · Design Infrastructure Site",
        "promptCenter.item.webReefwright.title" => "Reefwright · AI Support Knowledge Site",
        "promptCenter.item.dashboardPulse.title" => "Pulse · Growth Analytics",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · Logistics Operations",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · Enterprise Data Table",
        "promptCenter.item.componentFormLab.title" => "Form Lab · Form System",
        "promptCenter.item.modifyPolishCurrent.title" => "Polish the Current Screen",
        "promptCenter.item.modifyCompleteStates.title" => "Complete Component States",
        "collab.ownerConfirm.title" => "Confirm who you are joining",
        "collab.ownerConfirm.hint" => "Nothing from this session has been loaded yet.",
        "collab.ownerConfirm.account" => "Verified account",
        "collab.ownerConfirm.device" => "Verified device",
        "collab.ownerConfirm.claimedName" => "Name chosen by this account (not verified)",
        "collab.action.confirmOwner" => "Join this session",
        "collab.action.rejectOwner" => "Do not join",
        "collab.error.ownerNotConfirmed" => "You did not confirm the host, so nothing was loaded.",
        "sceneTemplate.title" => "Scene Templates",
        "sceneTemplate.searchPlaceholder" => "Search scenes or templates",
        "sceneTemplate.empty" => "No matching templates",
        "sceneTemplate.frames" => "Pages: {{count}}",
        "sceneTemplate.generate.placeholder" => "Describe a topic — AI generates the whole deck",
        "sceneTemplate.generate.button" => "Generate",
        "sceneTemplate.generate.hint" => "A new document, built from your topic as a full slide deck.",
        "sceneTemplate.generate.promptTemplate" => "Create a presentation deck (PPT) on the following topic: {{topic}}",
        "sceneTemplate.card.addToCanvas" => "Add to canvas",
        "sceneTemplate.card.generateFrom" => "Generate from this",
        "sceneTemplate.generate.basis" => "Based on: ",
        "sceneTemplate.filter.all" => "All",
        "sceneTemplate.scene.tutorial" => "Tutorials",
        "sceneTemplate.scene.comparison" => "Comparison",
        "sceneTemplate.scene.carousel" => "Carousel",
        "sceneTemplate.scene.slides" => "Slides",
        "sceneTemplate.scene.card" => "Cards",
        "sceneTemplate.scene.web" => "Web Pages",
        "sceneTemplate.generate.webPromptTemplate" => "Design a multi-section web landing page on the following topic: {{topic}}",
        "sceneTemplate.item.saasLandingOrange.title" => "SaaS Landing Page · Orange",
        "sceneTemplate.item.saasLandingOrange.summary" => "A light marketing page built on near-black panels and one orange accent: navigation, a hero with a product shot, three capability cards, a workflow walkthrough, testimonials and a subscribe footer. Swap the copy and it is a site.",
        "sceneTemplate.item.productLandingLight.title" => "Product Landing Page · Light",
        "sceneTemplate.item.productLandingLight.summary" => "A paper-white broadsheet product page: an interactive hero demo, capability columns, an analytics board, an old-versus-new comparison and three pricing tiers. Built for SaaS sites and product launches.",
        "sceneTemplate.item.screenshotTutorial.title" => "Three-Step Screenshot Tutorial Cards",
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "A cover, three how-to steps, and a closing call to action. Replace the screenshots and instructions to publish."
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "Knowledge and Insights Carousel",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "A cover, three key points, and a summary page—ideal for breaking one idea into a continuous, swipeable card series."
        }
        "sceneTemplate.item.beforeAfter.title" => "Before-and-After Redesign Comparison",
        "sceneTemplate.item.beforeAfter.summary" => {
            "A side-by-side before-and-after comparison with notes on the changes, ideal for retrospectives and portfolio showcases."
        }
        "sceneTemplate.item.slideDeck.title" => "Presentation · Six Slides",
        "sceneTemplate.item.slideDeck.summary" => {
            "A cover, agenda, key points, data, charts, and a closing slide in 16:9 presentation format. Replace the copy and present."
        }
        "sceneTemplate.item.knowledgeCardVertical.title" => "Knowledge Card · Portrait",
        "sceneTemplate.item.knowledgeCardVertical.summary" => "A single 3:4 card with a headline, four takeaways, and a byline. Swap the copy and post it.",
        "sceneTemplate.item.knowledgeCardSquare.title" => "Knowledge Card · Square",
        "sceneTemplate.item.knowledgeCardSquare.summary" => "A 1:1 card in the same layout, compact enough for a post header or a social share.",
        "sceneTemplate.item.pitchDeckDark.title" => "Pitch Deck · Dark",
        "sceneTemplate.item.pitchDeckDark.summary" => "A cover, the problem, the solution, the numbers, a roadmap and a contact slide. Big type on a dark ground, built for fundraising and launch talks.",
        "sceneTemplate.item.lectureDeckLight.title" => "Lecture Deck · Light",
        "sceneTemplate.item.lectureDeckLight.summary" => "A course cover, objectives, a concept walkthrough, a worked example, a comparison table and a wrap-up. Paper-white, easy on the eyes for a full class.",
        "sceneTemplate.item.minimalKeynote.title" => "Minimal Keynote",
        "sceneTemplate.item.minimalKeynote.summary" => "White space, oversized type, one centred line per slide — nine pages without a single card, and an agenda of nothing but rules and numbers. Built for launches and keynotes.",
        "sceneTemplate.item.gradientTech.title" => "Gradient Tech",
        "sceneTemplate.item.gradientTech.summary" => "A dark gradient ground with frosted-glass cards, covering architecture, benchmarks and a customer wall. For developer product launches.",
        "sceneTemplate.scene.infographic" => "Infographics",
        "sceneTemplate.item.punchQuoteCard.title" => "Quote Card · Poster",
        "sceneTemplate.item.punchQuoteCard.summary" => "A 3:4 card on near-black: two lines of oversized type over one yellow highlight bar. One sentence and nothing else, built for opinions and quotes.",
        "sceneTemplate.item.journalChecklistCard.title" => "Checklist Card · Knowledge Base",
        "sceneTemplate.item.journalChecklistCard.summary" => "A white checklist card on a soft grey ground: five tickable to-dos, a tag and a pull quote. Built for weekly plans and habit posts.",
        "sceneTemplate.item.dataReportInfographic.title" => "Data Report Infographic",
        "sceneTemplate.item.dataReportInfographic.summary" => "A tall scrolling graphic: dark masthead, three headline numbers, a ranked bar comparison, a share breakdown and three takeaways. Swap the numbers and post it.",
        "sceneTemplate.item.stepsFlowInfographic.title" => "Step-by-Step Infographic",
        "sceneTemplate.item.stepsFlowInfographic.summary" => "A tall scrolling graphic: five numbered step cards strung into one flow, each with a time estimate, plus two closing tips. Built for tutorials and how-to guides.",
        "sceneTemplate.item.eventPosterDeck.title" => "Event Deck · Poster",
        "sceneTemplate.item.eventPosterDeck.summary" => "A cover, highlights, a schedule, directions, ticket tiers and a sign-off. Gallery-white ground with decisive red and blue blocks, no rounded corners and no gradients — built for markets, club events and openings.",
        "sceneTemplate.item.pitfallListInfographic.title" => "Pitfall Checklist Infographic",
        "sceneTemplate.item.pitfallListInfographic.summary" => "A tall scrolling graphic: six mistakes ranked by how often people make them, each with what goes wrong and what to do instead, plus a four-line pre-publish check. Black, white and grey only.",
        "sceneTemplate.item.spineCultureCard.title" => "Vertical Spine Card · Mineral Pigment",
        "sceneTemplate.item.spineCultureCard.summary" => "A 3:4 card on ochre-clay dark ground with a vertical Chinese title, flaking plaster and pigment grains. For culture, long reads and personal-brand covers.",
        "sceneTemplate.item.metricSingleCard.title" => "Single Metric Card · Grid Hanzi",
        "sceneTemplate.item.metricSingleCard.summary" => "A 1:1 card: one huge number on pure white, a strict Swiss grid and a single red signal square. For conclusions and results.",
        "sceneTemplate.item.quoteFrameCard.title" => "Quote Frame Card · Silk Blue-Green",
        "sceneTemplate.item.quoteFrameCard.summary" => "A 4:5 card on aged silk yellow: one framed sentence, with an azurite-and-malachite mountain along the foot. For excerpts, interviews and quotations.",
        "sceneTemplate.item.dailySignCard.title" => "Daily Card · Garden Frame",
        "sceneTemplate.item.dailySignCard.summary" => "A 3:4 card on lime-washed wall with a hexagonal lattice window; date and one line sit inside it. Whitespace is the decoration. For daily posts and brand lines.",
        "sceneTemplate.item.priceTierCard.title" => "Price Tier Card · Arcade Neon",
        "sceneTemplate.item.priceTierCard.summary" => "A 1:1 card on ink-blue night with a three-tier price table, neon tube outlines and their scatter. For shops, events and package pricing.",
        "sceneTemplate.item.noticeBoardCard.title" => "Notice Card · Lead Type Press",
        "sceneTemplate.item.noticeBoardCard.summary" => "A 4:5 card on newsprint: masthead rules with a misregistered red plate, numbered clauses and a serial stamp. For notices, rules and house guidelines.",
        "sceneTemplate.item.milestoneTimelineInfographic.title" => "Milestone Timeline Infographic",
        "sceneTemplate.item.milestoneTimelineInfographic.summary" => "A tall scrolling graphic: one axis running the whole height, year ticks beside milestone cards, closing on what comes next. For retrospectives, brand history and project journeys.",
        "sceneTemplate.item.conceptContrastInfographic.title" => "Concept Contrast Infographic",
        "sceneTemplate.item.conceptContrastInfographic.summary" => "A tall scrolling graphic: the verdict first, then a definition card for each concept, a two-column breakdown by dimension, and finally how to choose.",
        "sceneTemplate.item.rankingBoardInfographic.title" => "Top-N Ranking Infographic",
        "sceneTemplate.item.rankingBoardInfographic.summary" => "A tall scrolling graphic: a gold-on-ink recommendation board — large badges for the top three, outlined ones for four through eight, each with when to use it and how often.",
        "sceneTemplate.item.faqThreadInfographic.title" => "FAQ Thread Infographic",
        "sceneTemplate.item.faqThreadInfographic.summary" => "A tall scrolling graphic: six question-and-answer pairs, Q solid and A outlined. No numbering, no order — any single pair stands on its own.",
        "sceneTemplate.item.dataStoryInfographic.title" => "Data Story Infographic",
        "sceneTemplate.item.dataStoryInfographic.summary" => "A tall scrolling graphic: four numbers strung into one causal line, each stage shown as a ten-block array, closing on a conclusion you can act on.",
        "sceneTemplate.item.challengeTrackerInfographic.title" => "30-Day Challenge Infographic",
        "sceneTemplate.item.challengeTrackerInfographic.summary" => "A tall scrolling graphic: a thirty-box grid, six by five, with milestones only on days 7, 15 and 30. Save it and cross off one box a day.",
        "sceneTemplate.item.ecosystemMapInfographic.title" => "Ecosystem Map Infographic",
        "sceneTemplate.item.ecosystemMapInfographic.summary" => "A tall scrolling graphic: a two-by-two array of four positions along one chain, three players in each, with the gaps called out. White cards floating on slate.",
        "sceneTemplate.item.doDontComparison.title" => "Do / Don't Two-Column",
        "sceneTemplate.item.doDontComparison.summary" => "A 3:4 card: two ways of doing the same thing side by side, told apart by material and icon rather than red-versus-green, so colour-blind readers can read it too.",
        "sceneTemplate.item.mythTruthComparison.title" => "Myth vs Truth Infographic",
        "sceneTemplate.item.mythTruthComparison.summary" => "A tall graphic: five pairs of “what people say / what is actually true”, myth narrow and pale on the left, truth wide and dark on the right, one pair at a time.",
        "sceneTemplate.item.pricingTiersComparison.title" => "Pricing Tiers Comparison",
        "sceneTemplate.item.pricingTiersComparison.summary" => "A 3:4 card: Free, Pro and Team side by side, price as the anchor, each column containing the one before it. For pricing pages and plan explainers.",
        "sceneTemplate.item.scenarioGuideComparison.title" => "Scenario Guide Infographic",
        "sceneTemplate.item.scenarioGuideComparison.summary" => "A tall graphic: no specs, just seven situations, each tagged with a verdict. The reader only has to find their own row.",
        "sceneTemplate.item.specTableComparison.title" => "Spec Table Infographic",
        "sceneTemplate.item.specTableComparison.summary" => "A tall graphic: two candidates in one real table, row by row, the winning cell lifted with a dark fill so you can see at a glance who wins where.",
        "sceneTemplate.item.threeWayComparison.title" => "Three-Way Comparison Infographic",
        "sceneTemplate.item.threeWayComparison.summary" => "A tall graphic: three options side by side with the recommendation in the middle; each column opens with a situation rather than a name, because readers are looking for themselves.",
        "sceneTemplate.item.timeShiftComparison.title" => "Time Shift · A Year Ago and Now",
        "sceneTemplate.item.timeShiftComparison.summary" => "A 3:4 card: a centred spine of labels, a year ago on the left and now on the right, both values of each item landing on the same row.",
        "sceneTemplate.item.tradeoffScaleComparison.title" => "Trade-off Scale",
        "sceneTemplate.item.tradeoffScaleComparison.summary" => "A 1:1 card: one beam, two pans — worth it on the left, what it costs on the right, an empty checkbox before every line. The card lists both sides and hands you the pen.",
        "sceneTemplate.item.versionDiffComparison.title" => "Version Diff",
        "sceneTemplate.item.versionDiffComparison.summary" => "A 1:1 card: no left-right split — every row completes its own “old → new”, so you just scroll and read.",
        "sceneTemplate.item.appOnboardingTriptych.title" => "App Onboarding Triptych",
        "sceneTemplate.item.appOnboardingTriptych.summary" => "A 3:4 card: three phones side by side with empty image wells. Drop in your own three onboarding screens, add the copy, and it is ready for review or posting.",
        "sceneTemplate.item.diyBlueprintGuide.title" => "DIY Blueprint Guide",
        "sceneTemplate.item.diyBlueprintGuide.summary" => "A tall graphic where the materials-and-specs table takes as much room as the steps — DIY usually fails in the preparation, not in the hands.",
        "sceneTemplate.item.photoCompositionTutorial.title" => "Phone Photography Composition",
        "sceneTemplate.item.photoCompositionTutorial.summary" => "3:4, five frames: each one a dark viewfinder with bright guide lines over the photo well, because composition can only be explained on the frame itself.",
        "sceneTemplate.item.recipeFourStep.title" => "Four-Step Recipe Card",
        "sceneTemplate.item.recipeFourStep.summary" => "A 4:5 card, 2×2: all four steps on one card. Screenshot it and cook from it — nobody wants to swipe pages at the stove.",
        "sceneTemplate.item.skincareRoutineCards.title" => "Skincare Routine Cards",
        "sceneTemplate.item.skincareRoutineCards.summary" => "4:5, six frames: every step fixes three numbers — how much, how long to wait, morning or night. Skincare goes wrong on dose and interval, not on order.",
        "sceneTemplate.item.softwareStepTutorial.title" => "Software Step Tutorial",
        "sceneTemplate.item.softwareStepTutorial.summary" => "A 4:5 card, the only dark one in the tutorial set: screenshot wells with numbered instructions, for tools and feature walkthroughs.",
        "sceneTemplate.item.storageMakeoverSteps.title" => "Storage Makeover Steps",
        "sceneTemplate.item.storageMakeoverSteps.summary" => "3:4, six frames: besides the action and the image well, each step fixes a done-condition and a time budget — you are finished when it looks like that.",
        "sceneTemplate.item.weeklyReportLesson.title" => "Weekly Report Lesson",
        "sceneTemplate.item.weeklyReportLesson.summary" => "A tall graphic: after the four-part structure it hands you a skeleton with underlined blanks — screenshot it and fill it in.",
        "sceneTemplate.item.workoutBreakdownGuide.title" => "Workout Breakdown Guide",
        "sceneTemplate.item.workoutBreakdownGuide.summary" => "A tall graphic: every movement carries a fixed sets / reps / rest bar alongside the image well and the cues, because people save this to train by the numbers.",
        "sceneTemplate.item.bookreviewSilkCarousel.title" => "Book / Film Review Carousel",
        "sceneTemplate.item.bookreviewSilkCarousel.summary" => "3:4, five boards: hook, an annotated excerpt, three insights, one quotable line, close. It takes a work apart into pieces you can carry away instead of retelling the plot.",
        "sceneTemplate.item.cityguideFilmCarousel.title" => "City Guide Carousel",
        "sceneTemplate.item.cityguideFilmCarousel.summary" => "3:4, seven boards: places and routes alternate — the place boards feed the dreamers, the day route and the eat-and-stay table feed the planners.",
        "sceneTemplate.item.datareportGridCarousel.title" => "Data Report Carousel",
        "sceneTemplate.item.datareportGridCarousel.summary" => "3:4, six boards: every data board is followed by a non-data one, so nobody swipes past the third chart. For quarterly reviews and industry notes.",
        "sceneTemplate.item.opinionLongformCarousel.title" => "Long-Form Opinion Carousel",
        "sceneTemplate.item.opinionLongformCarousel.summary" => "3:4, six boards: one strict visual master throughout — page number and title never move. On a carousel the previous board is gone, so consistency is not decoration.",
        "sceneTemplate.item.qaChalkboardCarousel.title" => "Q&A Carousel",
        "sceneTemplate.item.qaChalkboardCarousel.summary" => "3:4, six boards: one question per board, each with a hand-drawn question number in the corner. The question itself is the reason to swipe on.",
        "sceneTemplate.item.storyNightCarousel.title" => "Story Carousel",
        "sceneTemplate.item.storyNightCarousel.summary" => "3:4, seven boards: a personal retrospective built on time — the timeline on board five is the load-bearing wall, and the first four boards are each one of its marks.",
        "sceneTemplate.item.toolkitNotebookCarousel.title" => "Toolkit Collection Carousel",
        "sceneTemplate.item.toolkitNotebookCarousel.summary" => "3:4, six boards: six tools unfolded one per board, with the last board listing them all with page numbers — the collection reader is here to save it.",
        "sceneTemplate.item.tutorialJournalCarousel.title" => "Tutorial Carousel",
        "sceneTemplate.item.tutorialJournalCarousel.summary" => "3:4, six boards: one step per board, the finger is the progress bar. For crafts, software and everyday how-tos.",
        "sceneTemplate.item.yearreviewMineralCarousel.title" => "Year in Review Carousel",
        "sceneTemplate.item.yearreviewMineralCarousel.summary" => "3:4, eight boards: number boards run cold and reflection boards run warm, alternating. For year-end summaries and personal retrospectives.",
        "fileMenu.newFromTemplate" => "New from Template",
        "fileMenu.exportSlideshowHtml" => "Export slideshow HTML...",
        "fileMenu.exportPptx" => "Export PowerPoint...",
        "dialog.slideshowHtmlTitle" => "Export slideshow",
        "dialog.slideshowHtmlSummary" => "Exported {{count}} slides to:",
        "dialog.slideshowHtmlEmpty" => "This deck has no visible slides to export.",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "Importable HTML content is unavailable.",
        "htmlImport.warn.content.empty_body" => {
            "Importable content in the HTML body is unavailable."
        }
        "htmlImport.warn.content.dom_depth_truncated" => {
            "HTML nested deeper than {{max_depth}} levels was dropped."
        }
        "htmlImport.warn.content.node_limit_truncated" => {
            "Node limit reached; the remaining page content was omitted."
        }
        "htmlImport.warn.content.node_limit_mapping" => {
            "Node limit reached; part of the HTML tree was omitted."
        }
        "htmlImport.warn.content.node_limit_inline_row" => {
            "Node limit reached; an inline formatting row was omitted."
        }
        "htmlImport.warn.content.node_limit_pseudo" => {
            "Node limit reached; generated pseudo-elements were omitted."
        }
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "CSS rules nested deeper than {{max_depth}} at-rules were ignored."
        }
        "htmlImport.warn.css.unterminated_rule" => "An unterminated CSS rule was ignored.",
        "htmlImport.warn.css.marker_rules_unsupported" => "CSS ::marker rules were not imported.",
        "htmlImport.warn.css.nesting_unsupported" => "Nested CSS style rules were ignored.",
        "htmlImport.warn.css.invalid_layer_name" => {
            "The invalid @layer name '{{name}}' was ignored."
        }
        "htmlImport.warn.css.unsupported_statement" => {
            "The unsupported @{{name}} statement was ignored."
        }
        "htmlImport.warn.css.media_without_viewport" => {
            "@media rules without a viewport were ignored."
        }
        "htmlImport.warn.css.invalid_layer_block_name" => {
            "The invalid @layer block name '{{name}}' was ignored."
        }
        "htmlImport.warn.css.unsupported_container_block" => "The @container block was ignored.",
        "htmlImport.warn.css.unsupported_block" => "The unsupported @{{name}} block was ignored.",
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "The @font-face web font '{{family}}' is unavailable."
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "Percentage offsets of an absolutely positioned element were approximated."
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "Percentage position:relative offsets were approximated."
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "CSS aspect-ratio without a definite axis was ignored."
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "CSS aspect-ratio inside an indefinite containing block was ignored."
        }
        "htmlImport.warn.layout.position_sticky_ignored" => "CSS position:sticky was ignored.",
        "htmlImport.warn.layout.grid_tracks_approximated" => {
            "Unsupported CSS grid tracks were approximated."
        }
        "htmlImport.warn.layout.float_ignored" => "CSS float was ignored.",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "CSS mix-blend-mode at node level was approximated."
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "CSS overflow: auto / scroll was approximated."
        }
        "htmlImport.warn.layout.negative_margins_ignored" => "Negative CSS margins were ignored.",
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => {
            "CSS margins on a visual box were ignored."
        }
        "htmlImport.warn.layout.inline_margin_wrapping_approximated" => "An inline with CSS margins was boxed and may no longer wrap across lines.",
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "content-box percentage sizing was approximated."
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "Empty CSS grid cells left by explicit start lines were approximated."
        }
        "htmlImport.warn.layout.grid_span_reflowed" => {
            "A CSS grid item whose span did not fit its start line was approximated."
        }
        "htmlImport.warn.layout.grid_rows_node_limit" => {
            "Node limit reached; CSS grid row wrappers were omitted."
        }
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "CSS grid track widths using auto-fit / auto-fill were approximated."
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "CSS grid-template-areas placement was not imported."
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => {
            "CSS grid-row placement was not imported."
        }
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "CSS grid-column `{{value}}` was approximated."
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => {
            "CSS block-axis auto margins were not imported."
        }
        "htmlImport.warn.layout.auto_margin_node_limit" => {
            "Node limit reached; CSS auto-margin alignment was omitted."
        }
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "A CSS in-flow offset on an element with no definite size was dropped."
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => {
            "Node limit reached; a CSS in-flow offset was omitted."
        }
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "CSS in-flow offsets (position:relative insets, transform translation) were approximated."
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "A CSS in-flow offset on a box that cannot host an offset wrapper was dropped."
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "flex-wrap on a column flex container was not imported."
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => {
            "flex-wrap:wrap-reverse was approximated."
        }
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "flex-wrap on a container with no definite width was ignored."
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "CSS align-content on a wrapping flex container was not imported."
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "flex-wrap with indeterminate child main-axis sizes was ignored."
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => {
            "Node limit reached; flex-wrap rows were omitted."
        }
        "htmlImport.warn.transform.unsupported_syntax" => {
            "Unsupported CSS transform syntax was ignored."
        }
        "htmlImport.warn.transform.unsupported_function" => {
            "Unsupported CSS transform functions (3D, matrix3d) were ignored."
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "A percentage CSS transform translation on an indefinite axis was dropped."
        }
        "htmlImport.warn.transform.non_finite_matrix" => {
            "A CSS transform that produced a non-finite matrix was ignored."
        }
        "htmlImport.warn.transform.skew_dropped" => "CSS transform skew was dropped.",
        "htmlImport.warn.transform.degenerate_scale" => {
            "A CSS transform with a zero or non-finite scale was approximated."
        }
        "htmlImport.warn.transform.mirroring_absolute" => {
            "CSS transform mirroring was approximated."
        }
        "htmlImport.warn.transform.origin_z_ignored" => {
            "The CSS transform-origin Z offset was ignored."
        }
        "htmlImport.warn.transform.scale_not_baked" => {
            "A CSS transform scale that could not be baked into the node size was dropped."
        }
        "htmlImport.warn.transform.scale_baked" => {
            "CSS transform scale baked into the node size was approximated."
        }
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "CSS transform scale on an auto-sized element was ignored."
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "Directional or spaced CSS background-repeat was approximated."
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => {
            "An explicit CSS background tile size was ignored."
        }
        "htmlImport.warn.visual.background_size_auto_box" => {
            "CSS background-size on an auto-sized element was approximated."
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "CSS background-size that needs the image's intrinsic size was approximated."
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "An unsupported CSS background-position was ignored."
        }
        "htmlImport.warn.visual.background_image_url_empty" => {
            "An empty CSS background image URL was ignored."
        }
        "htmlImport.warn.visual.conic_gradient_ignored" => "CSS conic gradients were ignored.",
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "An unsupported CSS background-image layer was ignored."
        }
        "htmlImport.warn.visual.background_color_unresolved" => {
            "An unresolved CSS background color was ignored."
        }
        "htmlImport.warn.visual.background_position_dropped" => {
            "CSS background-position was ignored."
        }
        "htmlImport.warn.visual.border_colors_approximated" => {
            "Per-side CSS border colors were approximated."
        }
        "htmlImport.warn.visual.border_styles_approximated" => {
            "Mixed per-side CSS border styles were approximated."
        }
        "htmlImport.warn.visual.border_style_complex" => {
            "A complex CSS border style was approximated."
        }
        "htmlImport.warn.visual.border_style_unsupported" => {
            "An unsupported CSS border style was approximated."
        }
        "htmlImport.warn.visual.border_radius_elliptical" => {
            "Elliptical CSS border radii were approximated."
        }
        "htmlImport.warn.visual.border_radius_unsupported" => {
            "An unsupported CSS border radius was ignored."
        }
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "An unsupported CSS box-shadow layer was ignored."
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => {
            "The CSS gradient color interpolation method was ignored."
        }
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "An unsupported CSS linear-gradient direction was ignored."
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => {
            "CSS gradient color hints were ignored."
        }
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => {
            "An unsupported CSS gradient color stop was ignored."
        }
        "htmlImport.warn.visual.gradient_too_few_stops" => {
            "A CSS gradient with fewer than two usable stops was ignored."
        }
        "htmlImport.warn.visual.gradient_repeating_approximated" => {
            "A repeating CSS gradient was approximated."
        }
        "htmlImport.warn.visual.gradient_stops_clamped" => {
            "Out-of-range CSS gradient stops were approximated."
        }
        "htmlImport.warn.visual.blur_radius_unsupported" => {
            "An unsupported CSS blur radius was ignored."
        }
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "An unsupported CSS filter drop-shadow() was ignored."
        }
        "htmlImport.warn.visual.filter_function_unsupported" => {
            "An unsupported CSS filter function was ignored."
        }
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "An unsupported CSS backdrop-filter function was ignored."
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "An unsupported CSS background-blend-mode was ignored."
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "CSS mix-blend-mode on individual fills was approximated."
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "An unsupported CSS mix-blend-mode was ignored."
        }
        "htmlImport.warn.visual.property_not_representable" => "CSS {{property}} was ignored.",
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "CSS background-size on a gradient was ignored."
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "An unsupported CSS radial-gradient position was ignored."
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "An elliptical CSS radial-gradient was approximated."
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "A CSS radial-gradient extent keyword was approximated."
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "An unsupported CSS radial-gradient size was ignored."
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => {
            "An unsupported CSS text-shadow layer was ignored."
        }
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "CSS text-shadow layers after the first were ignored."
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => {
            "CSS text-shadow on an inline element was ignored."
        }
        "htmlImport.warn.list.style_image_ignored" => "CSS list-style-image was not imported.",
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "A `list-style-position: outside` hanging marker was approximated."
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "The unsupported CSS list-style-type `{{value}}` was approximated."
        }
        "htmlImport.warn.media.object_fit_scale_down" => {
            "CSS object-fit:scale-down was approximated."
        }
        "htmlImport.warn.media.object_fit_none_ignored" => "CSS object-fit:none was ignored.",
        "htmlImport.warn.media.object_position_ignored" => "CSS object-position was ignored.",
        "htmlImport.warn.media.image_intrinsic_axis_unresolved" => {
            "The image's intrinsic aspect ratio could not resolve the missing axis because the authored size is dynamic or its containing block is indefinite."
        }
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "An unsupported CSS mix-blend-mode on an image was ignored."
        }
        "htmlImport.warn.media.inline_svg_placeholder" => {
            "An inline <svg> element was imported as a placeholder."
        }
        "htmlImport.warn.media.input_type_fallback" => {
            "An unsupported <input> type was approximated."
        }
        "htmlImport.warn.media.element_placeholder" => {
            "The <{{tag}}> element was imported as a placeholder."
        }
        "htmlImport.warn.media.picture_undecodable_types" => {
            "A <picture> with only undecodable source types was approximated."
        }
        "htmlImport.warn.table.rowspan_ignored" => "The HTML rowspan attribute was not imported.",
        "htmlImport.warn.table.row_groups_unflattened" => {
            "Column widths of a table whose row groups CSS un-flattened were approximated."
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "Column widths of a CSS table without a definite width were approximated."
        }
        "htmlImport.warn.resource.invalid_base_href" => {
            "The invalid <base href> {{href}} was ignored."
        }
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "The <base href> {{href}} outside the project origin was ignored."
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => {
            "The external stylesheet {{url}} is unavailable."
        }
        "htmlImport.warn.resource.image_outside_origin" => {
            "The image {{url}} outside the project origin was imported as a placeholder."
        }
        "htmlImport.warn.resource.image_unavailable" => {
            "The unavailable image {{url}} was imported as a placeholder."
        }
        "htmlImport.warn.resource.css_import_invalid" => {
            "The invalid CSS @import {{prelude}} was ignored."
        }
        "htmlImport.warn.resource.css_import_unresolvable" => {
            "The CSS @import {{reference}} is unavailable."
        }
        "htmlImport.warn.resource.css_import_cycle" => {
            "The cyclic CSS @import {{url}} was ignored."
        }
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "The CSS @import {{url}} beyond depth {{max_depth}} was ignored."
        }
        "htmlImport.warn.resource.css_import_unavailable" => {
            "The CSS @import {{url}} is unavailable."
        }
        "htmlImport.warn.project.multiple_html_entries" => {
            "{{count}} HTML entries were found; {{entry}} was chosen and the rest were approximated."
        }
        "htmlImport.warn.snapshot.truncated" => "Part of the browser snapshot was dropped.",
        "htmlImport.warn.snapshot.node_limit" => {
            "Node limit reached; the remaining snapshot content was omitted."
        }
        "htmlImport.warn.snapshot.tainted_images" => {
            "{{count}} CORS-tainted images, kept as remote URLs, are unavailable."
        }
        "htmlImport.warn.snapshot.invalid_rect" => {
            "A snapshot node with a missing or invalid rect was dropped."
        }
        "htmlImport.warn.snapshot.unknown_kind" => "A snapshot node of unknown kind was dropped.",
        "htmlImport.warn.snapshot.rejected" => "The browser snapshot ({{reason}}) was dropped.",
        "htmlImport.warn.snapshot.unsupported_transform" => {
            "An unsupported snapshot transform was ignored."
        }
        "htmlImport.warn.css.media_empty_query" => "An empty @media query was ignored.",
        "htmlImport.warn.css.media_unsupported_type" => {
            "The unsupported @media type '{{name}}' was ignored."
        }
        "htmlImport.warn.css.media_unsupported_condition" => {
            "The unsupported @media condition '{{input}}' was ignored."
        }
        "htmlImport.warn.css.media_invalid_orientation" => {
            "The invalid @media orientation '{{value}}' was ignored."
        }
        "htmlImport.warn.css.media_unsupported_feature" => {
            "The unsupported @media feature '{{name}}' was ignored."
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "The unsupported @media range '({{input}})' was ignored."
        }
        "htmlImport.warn.css.media_invalid_range" => {
            "The invalid @media range '({{input}})' was ignored."
        }
        "htmlImport.warn.css.media_invalid_length" => {
            "The invalid @media length '{{value}}' was ignored."
        }
        "htmlImport.diagnostics.title" => "HTML import finished",
        "htmlImport.diagnostics.summary" => "Degraded items: {{count}}",
        "htmlImport.diagnostics.dismiss" => "Dismiss",
        "htmlImport.diagnostics.expand" => "Show details",
        "htmlImport.diagnostics.collapse" => "Hide details",
        "htmlImport.diagnostics.more" => "+{{count}} more",
        "dialog.pptxTitle" => "Export PowerPoint",
        "dialog.pptxSummary" => "Exported {{count}} slides to:",
        "dialog.pptxEmpty" => "This deck has no visible slides to export.",
        "settings.agents.acpQuickAdd" => "Quick add",
        "settings.agents.acpPresetAdd" => "Add",
        "settings.agents.acpNotInstalled" => "Not installed",
        "assetCenter.title" => "Asset Center",
        "assetCenter.tab.templates" => "Templates",
        "assetCenter.tab.styles" => "Styles",
        "assetCenter.style.empty" => "No matching styles",
        "assetCenter.style.pinned" => "Pinned",
        "assetCenter.style.searchPlaceholder" => "Search styles or tags",
        "assetCenter.style.generateHint" => "A new document built from your topic, in the pinned style.",
        "ai.pinnedStyle" => "Style: {{name}}",
        "assetCenter.style.import" => "Import style",
        "assetCenter.style.mine" => "My styles",
        "assetCenter.style.builtIn" => "Built-in styles",
        "assetCenter.style.importTitle" => "Import DESIGN.md",
        "assetCenter.style.importHint" => "Paste the whole DESIGN.md, then confirm the import.",
        "assetCenter.style.importSource" => "You can copy a style from a DESIGN.md library such as styles.refero.design.",
        "assetCenter.style.importConfirm" => "Import",
        "assetCenter.style.importCancel" => "Cancel",
        "assetCenter.style.importPickFile" => "Choose file…",
        "assetCenter.style.importHintFile" => "Choose a DESIGN.md file, or paste the whole document below.",
        "assetCenter.style.importPlaceholder" => "Paste your DESIGN.md here",
        "assetCenter.style.importEmpty" => "That file is empty, or too short to be a style guide.",
        "assetCenter.style.importNotText" => "That file does not read as Markdown text.",
        "assetCenter.style.importTooLarge" => "That file is larger than 512 KB.",
        "slidesPanel.tabSlides" => "Slides",
        "slidesPanel.tabCards" => "Cards",
        "slidesPanel.present" => "Present",
        "slidesPanel.exportPdf" => "Export PDF",
        "slidesPanel.exportAllSlides" => "Export all slides",
        "slidesPanel.exportSelectedSlides" => "Export selected slides ({{count}})",
        "settings.tab.ai" => "AI",
        "settings.agents.heroTitle" => "Connect your AI provider",
        "settings.agents.heroSubtitle" => "OpenPencil drives your local CLI agents and API providers — connect one to start generating designs.",
        "settings.agents.statusConnected" => "Connected",
        "settings.agents.statusNotConnected" => "Not connected",
        "settings.agents.statusChecking" => "Checking status…",
        "settings.mcp.heroTitle" => "Connect to OpenPencil via MCP externally",
        "settings.mcp.heroSubtitle" => "Point any MCP-speaking CLI or editor at this workspace, then drive the canvas with the same tools the built-in agent uses.",
        "settings.mcp.terminalFootnote" => "* On startup, MCP is automatically set up for the selected CLI tools.",
        "settings.mcp.customConfigTitle" => "Custom MCP Server Configuration",
        "settings.mcp.customConfigDesc" => "Paste this into any client that reads a standard MCP server block.",
        "settings.mcp.copyConfig" => "Copy MCP config",
        "settings.system.heroTitle" => "System preferences",
        "settings.system.heroSubtitle" => "Appearance, updates and canvas behaviour for this install.",
        "settings.system.appearance" => "Appearance",
        "settings.system.appearanceLight" => "Light",
        "settings.system.appearanceDark" => "Dark",
        "settings.system.pencilCursor" => "Pencil cursor",
        "settings.images.heroTitle" => "Images for your designs",
        "settings.images.heroSubtitle" => "Search Openverse for photos, or connect a provider to generate them on demand.",
        "settings.fonts.heroTitle" => "Fonts in this document",
        "settings.fonts.heroSubtitle" => "Resolve fonts a document asks for but this machine doesn't have, and manage the ones you imported.",
        "settings.account.heroTitle" => "Your account",
        "settings.account.heroSubtitle" => "Sign in to sync your workspace and licence across devices.",
        "tooltip.topbar.file" => "File",
        "tooltip.topbar.import" => "Import",
        "tooltip.topbar.language" => "Language",
        "tooltip.topbar.collaboration" => "Collaboration",
        "tooltip.topbar.preview" => "Preview",
        "tooltip.topbar.exitPreview" => "Exit preview",
        "tooltip.topbar.account" => "Account",
        "settings.agents.providerRollMore" => "and {{count}} more",
        "ai.thinking.adaptive" => "Thinking: Auto",
        "ai.thinking.disabled" => "Thinking: Off",
        "ai.thinking.enabled" => "Thinking: On",
        "ai.designProgress.detail.repairsApplied" => "{{count}} auto-repair(s) applied",
        "ai.designProgress.detail.repairsMore" => "… and {{count}} more (see log)",
        "ai.styleCard.builtin" => "Built-in style",
        "ai.styleCard.imported" => "Imported DESIGN.md",
        "ai.styleCard.documentDesignMd" => "Document design.md",
        _ => return super::en_collab::lookup(key),
    })
}
