//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `hi_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "छवियां खोजें…",
        "imagePanel.searching" => "खोज रहे हैं…",
        "imagePanel.noResults" => "कोई परिणाम नहीं मिला",
        "imagePanel.searchPrompt" => "छवियां खोजें",
        "imagePanel.sourceNotice" => "{{source}} से छवियां। मुक्त लाइसेंस — उपयोग से पहले लाइसेंस जांचें।",
        "imagePanel.genNotConfigured" => "छवि निर्माण कॉन्फ़िगर नहीं है",
        "imagePanel.openSettings" => "सेटिंग्स खोलें",
        "imagePanel.promptPlaceholder" => "छवि का वर्णन करें…",
        "providerProbe.connectedViaCli" => "{{name}} CLI के ज़रिए कनेक्ट किया गया",
        "providerProbe.cliExitedWithError" => "{{name}} CLI त्रुटि के साथ बंद हो गई",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI ने कोई वर्शन जानकारी नहीं दी",
        "providerProbe.modelQueryFailed" => "{{name}} मॉडल क्वेरी विफल रही या समय समाप्त हो गया",
        "providerProbe.modelQueryFailedRunLogin" => {
            "{{name}} मॉडल क्वेरी विफल रही। प्रमाणीकरण के लिए एक बार {{command}} चलाएँ।"
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "{{name}} मॉडल क्वेरी के लिए प्रमाणीकरण आवश्यक है। साइन इन करने के लिए एक बार {{command}} चलाएँ।"
        }
        "providerProbe.unrecognizedModelCatalog" => "{{name}} ने एक अपरिचित मॉडल सूची लौटाई",
        "promptCenter.title" => "प्रॉम्प्ट केंद्र",
        "promptCenter.searchPlaceholder" => "प्रॉम्प्ट खोजें…",
        "promptCenter.category.all" => "सभी",
        "promptCenter.category.starter" => "त्वरित शुरुआत",
        "promptCenter.category.mobileApp" => "मोबाइल ऐप",
        "promptCenter.category.webPage" => "वेब पेज",
        "promptCenter.category.dashboard" => "डैशबोर्ड",
        "promptCenter.category.component" => "कॉम्पोनेंट",
        "promptCenter.category.modify" => "संशोधन",
        "promptCenter.category.custom" => "मेरे",
        "promptCenter.empty" => "कोई मेल खाता प्रॉम्प्ट नहीं मिला",
        "promptCenter.saveCurrent" => "मौजूदा इनपुट को प्रॉम्प्ट के रूप में सहेजें",
        "promptCenter.saveTitlePlaceholder" => "प्रॉम्प्ट का शीर्षक लिखें",
        "promptCenter.save" => "सहेजें",
        "promptCenter.cancel" => "रद्द करें",
        "promptCenter.delete" => "हटाएँ",
        "promptCenter.screens" => "{{count}} स्क्रीन",
        "promptCenter.freeform" => "मुक्त शैली",
        "promptCenter.item.wander.title" => "Wander · यात्रा की योजना",
        "promptCenter.item.forage.title" => "Forage · मौसमी व्यंजन",
        "promptCenter.item.still.title" => "Still · ध्यान और नींद",
        "promptCenter.item.hearth.title" => "Hearth · स्मार्ट होम",
        "promptCenter.item.meteo.title" => "Meteo · तल्लीन कर देने वाला मौसम",
        "promptCenter.item.marginalia.title" => "Marginalia · पढ़ना और टिप्पणियाँ",
        "promptCenter.item.lingua.title" => "Lingua · भाषा सीखना",
        "promptCenter.item.daybreak.title" => "Daybreak · कॉफी ऑर्डर",
        "promptCenter.item.verdant.title" => "Verdant · पौधों की देखभाल",
        "promptCenter.item.companion.title" => "Companion · पालतू जीवन",
        "promptCenter.item.relic.title" => "Relic · चुनिंदा पुरानी वस्तुओं का बाज़ार",
        "promptCenter.item.nocturne.title" => "Nocturne · तारों को देखने की मार्गदर्शिका",
        "promptCenter.item.marquee.title" => "Marquee · फ़िल्म देखने की सूची",
        "promptCenter.item.ritual.title" => "Ritual · आदतें बनाना",
        "promptCenter.item.ember.title" => "Ember · मनोदशा डायरी",
        "promptCenter.item.volt.title" => "Volt · इलेक्ट्रिक वाहन साथी",
        "promptCenter.item.aloft.title" => "Aloft · उड़ान ट्रैकिंग",
        "promptCenter.item.gallery.title" => "Gallery · प्रदर्शनियाँ और संस्कृति",
        "promptCenter.item.nightcap.title" => "Nightcap · घर पर कॉकटेल बनाना",
        "promptCenter.item.bloom.title" => "Bloom · बच्चे की विकास डायरी",
        "promptCenter.item.extremeWeather.title" => "मौसम ऐप · मुझे चौंकाएँ",
        "promptCenter.item.extremeNowPlaying.title" => "अभी चल रहा है · प्रकाशित करने लायक सुंदर",
        "promptCenter.item.extremeDailyApp.title" => "हर दिन खोलने लायक ऐप",
        "promptCenter.item.extremeCalendar.title" => "कैलेंडर को नए सिरे से गढ़ें",
        "promptCenter.item.extremeCalm.title" => "एक स्क्रीन में शांति",
        "promptCenter.item.webOrbit.title" => "Orbit · एआई वर्कबेंच लैंडिंग पेज",
        "promptCenter.item.webAtelier.title" => "Atelier · फ़र्नीचर ई-कॉमर्स",
        "promptCenter.item.webKilnform.title" => "Kilnform · डिज़ाइन इन्फ़्रास्ट्रक्चर साइट",
        "promptCenter.item.webReefwright.title" => "Reefwright · AI सपोर्ट नॉलेज साइट",
        "promptCenter.item.dashboardPulse.title" => "Pulse · विकास विश्लेषण डैशबोर्ड",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · लॉजिस्टिक्स संचालन",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · एंटरप्राइज़ डेटा तालिका",
        "promptCenter.item.componentFormLab.title" => "Form Lab · फ़ॉर्म कॉम्पोनेंट प्रणाली",
        "promptCenter.item.modifyPolishCurrent.title" => "वर्तमान स्क्रीन को निखारें",
        "promptCenter.item.modifyCompleteStates.title" => "कॉम्पोनेंट की अवस्थाएँ पूरी करें",
        "sceneTemplate.title" => "सीन टेम्पलेट",
        "sceneTemplate.searchPlaceholder" => "सीन या टेम्पलेट खोजें…",
        "sceneTemplate.empty" => "कोई मेल खाता टेम्पलेट नहीं मिला",
        "sceneTemplate.frames" => "{{count}} पेज",
        "sceneTemplate.generate.placeholder" => "विषय बताइए — AI पूरी प्रस्तुति बना देगा",
        "sceneTemplate.generate.button" => "बनाएँ",
        "sceneTemplate.generate.hint" => "एक नया दस्तावेज़, आपके विषय से पूरी प्रस्तुति के रूप में बनाया गया।",
        "sceneTemplate.generate.promptTemplate" => "इस विषय पर एक प्रस्तुति (PPT) बनाइए: {{topic}}",
        "sceneTemplate.card.addToCanvas" => "कैनवास में जोड़ें",
        "sceneTemplate.card.generateFrom" => "इससे जनरेट करें",
        "sceneTemplate.generate.basis" => "आधार: ",
        "sceneTemplate.filter.all" => "सभी",
        "sceneTemplate.scene.tutorial" => "ट्यूटोरियल",
        "sceneTemplate.scene.comparison" => "तुलना",
        "sceneTemplate.scene.carousel" => "कैरोसेल",
        "sceneTemplate.scene.slides" => "स्लाइड",
        "sceneTemplate.scene.card" => "कार्ड",
        "sceneTemplate.scene.web" => "वेब पेज",
        "sceneTemplate.generate.webPromptTemplate" => "निम्न विषय पर कई सेक्शन वाला वेब लैंडिंग पेज डिज़ाइन करें: {{topic}}",
        "sceneTemplate.item.saasLandingOrange.title" => "SaaS लैंडिंग पेज · नारंगी",
        "sceneTemplate.item.saasLandingOrange.summary" => "हल्की पृष्ठभूमि पर लगभग-काले पैनल और एक नारंगी रंग वाला मार्केटिंग पेज: नेविगेशन, प्रोडक्ट स्क्रीनशॉट के साथ हीरो, तीन क्षमता कार्ड, वर्कफ़्लो प्रदर्शन, ग्राहक प्रशंसा और सब्सक्राइब फ़ुटर। टेक्स्ट बदलिए और साइट तैयार।",
        "sceneTemplate.item.productLandingLight.title" => "प्रोडक्ट लैंडिंग पेज · हल्का",
        "sceneTemplate.item.productLandingLight.summary" => "काग़ज़-सफ़ेद अख़बारी अंदाज़ का प्रोडक्ट पेज: इंटरैक्टिव हीरो डेमो, क्षमता कॉलम, एनालिटिक्स बोर्ड, पहले-बाद की तुलना और तीन मूल्य स्तर। SaaS साइट और प्रोडक्ट लॉन्च के लिए।",
        "sceneTemplate.item.screenshotTutorial.title" => "तीन चरणों वाला स्क्रीनशॉट ट्यूटोरियल कार्ड",
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "कवर, तीन चरण और अंत में कार्रवाई का आह्वान; स्क्रीनशॉट और टेक्स्ट बदलकर प्रकाशित करें।"
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "ज्ञान और विचारों की कार्ड-श्रृंखला",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "कवर, तीन मुख्य बिंदु और सारांश पेज; किसी विचार को स्वाइप किए जा सकने वाले क्रमिक कार्डों में बाँटने के लिए उपयुक्त।"
        }
        "sceneTemplate.item.beforeAfter.title" => "रीडिज़ाइन से पहले और बाद की तुलना",
        "sceneTemplate.item.beforeAfter.summary" => {
            "बदलावों के नोट्स के साथ पहले और बाद का साथ-साथ तुलनात्मक दृश्य, समीक्षा और पोर्टफ़ोलियो में दिखाने के लिए उपयुक्त।"
        }
        "sceneTemplate.item.slideDeck.title" => "प्रस्तुति · छह स्लाइड",
        "sceneTemplate.item.slideDeck.summary" => {
            "कवर, विषय-सूची, मुख्य बिंदु, डेटा, चार्ट और समापन; 16:9 प्रस्तुति अनुपात में, टेक्स्ट बदलते ही प्रस्तुत करने के लिए तैयार।"
        }
        "sceneTemplate.item.knowledgeCardVertical.title" => "नॉलेज कार्ड · लंबवत",
        "sceneTemplate.item.knowledgeCardVertical.summary" => "शीर्षक, चार मुख्य बिंदु और नाम-पट्टी वाला एक 3:4 कार्ड; टेक्स्ट बदलकर प्रकाशित करें।",
        "sceneTemplate.item.knowledgeCardSquare.title" => "नॉलेज कार्ड · वर्गाकार",
        "sceneTemplate.item.knowledgeCardSquare.summary" => "उसी लेआउट का 1:1 कार्ड, जो पोस्ट हेडर या सोशल शेयर के लिए पर्याप्त सघन है।",
        "sceneTemplate.item.pitchDeckDark.title" => "पिच डेक · डार्क",
        "sceneTemplate.item.pitchDeckDark.summary" => "कवर, समस्या, समाधान, आँकड़े, रोडमैप और संपर्क पेज। गहरे रंग पर बड़ा टाइप — फंडरेज़िंग और लॉन्च के लिए।",
        "sceneTemplate.item.lectureDeckLight.title" => "व्याख्यान डेक · लाइट",
        "sceneTemplate.item.lectureDeckLight.summary" => "कोर्स कवर, लक्ष्य, अवधारणा की व्याख्या, हल किया उदाहरण, तुलना तालिका और सारांश। कागज़-सफ़ेद, पूरी कक्षा तक आँखों पर आसान।",
        "sceneTemplate.item.minimalKeynote.title" => "मिनिमल कीनोट",
        "sceneTemplate.item.minimalKeynote.summary" => "खुली जगह, बहुत बड़ा टाइप, हर पेज पर बीच में एक वाक्य — नौ पेज और एक भी कार्ड नहीं, सूची में सिर्फ़ रेखाएँ और अंक। लॉन्च और कीनोट के लिए।",
        "sceneTemplate.item.gradientTech.title" => "ग्रेडिएंट टेक",
        "sceneTemplate.item.gradientTech.summary" => "गहरे ग्रेडिएंट पर फ़्रॉस्टेड-ग्लास कार्ड: आर्किटेक्चर, परफ़ॉर्मेंस तुलना और ग्राहक दीवार। डेवलपर प्रोडक्ट लॉन्च के लिए।",
        "sceneTemplate.scene.infographic" => "इन्फ़ोग्राफ़िक",
        "sceneTemplate.item.punchQuoteCard.title" => "कोट कार्ड · पोस्टर",
        "sceneTemplate.item.punchQuoteCard.summary" => "लगभग काले तल पर 3:4 कार्ड: दो बहुत बड़ी पंक्तियाँ और एक पीली पट्टी। बस एक वाक्य — विचार और उद्धरण के लिए।",
        "sceneTemplate.item.journalChecklistCard.title" => "चेकलिस्ट कार्ड · नॉलेज बेस",
        "sceneTemplate.item.journalChecklistCard.summary" => "हल्के धूसर तल पर एक सफ़ेद चेकलिस्ट कार्ड: टिक करने लायक पाँच काम, एक टैग और एक उद्धरण खंड। साप्ताहिक योजना के लिए।",
        "sceneTemplate.item.dataReportInfographic.title" => "डेटा निष्कर्ष इन्फ़ोग्राफ़िक",
        "sceneTemplate.item.dataReportInfographic.summary" => "लंबी स्क्रॉल छवि: गहरा शीर्ष, तीन बड़े आँकड़े, बार तुलना, हिस्सेदारी और तीन निष्कर्ष। आँकड़े बदलिए और पोस्ट कीजिए।",
        "sceneTemplate.item.stepsFlowInfographic.title" => "चरण-दर-चरण इन्फ़ोग्राफ़िक",
        "sceneTemplate.item.stepsFlowInfographic.summary" => "लंबी स्क्रॉल छवि: क्रमांकित पाँच चरण एक प्रवाह में, हर एक पर लगने वाला समय, साथ में दो सुझाव। ट्यूटोरियल और गाइड के लिए।",
        "sceneTemplate.item.eventPosterDeck.title" => "इवेंट deck · पोस्टर",
        "sceneTemplate.item.eventPosterDeck.summary" => "कवर, ख़ास बातें, कार्यक्रम, पहुँचने का रास्ता, टिकट और समापन। गैलरी जैसा सफ़ेद तल, लाल और नीले रंग-खंड, न गोल कोने न ग्रेडिएंट — मेले, क्लब आयोजन और उद्घाटन के लिए।",
        "sceneTemplate.item.pitfallListInfographic.title" => "आम गलतियों की इन्फ़ोग्राफ़िक",
        "sceneTemplate.item.pitfallListInfographic.summary" => "लंबी स्क्रॉल छवि: छह गलतियाँ आवृत्ति के क्रम में, हर एक के साथ «क्या गलत है» और «इसके बजाय क्या करें», अंत में चार पंक्तियों की जाँच सूची। केवल काला, सफ़ेद और धूसर।",
        "sceneTemplate.item.spineCultureCard.title" => "ऊर्ध्व शीर्षक कार्ड · खनिज रंगद्रव्य",
        "sceneTemplate.item.spineCultureCard.summary" => "गेरुई मिट्टी के गहरे तल पर ऊर्ध्व चीनी शीर्षक, उखड़ता प्लास्टर और रंगद्रव्य के कण। 3:4। संस्कृति, लंबे लेख और निजी ब्रांड आवरण के लिए।",
        "sceneTemplate.item.metricSingleCard.title" => "एकल आँकड़ा कार्ड · ग्रिड हांज़ी",
        "sceneTemplate.item.metricSingleCard.summary" => "शुद्ध सफ़ेद पर एक विशाल संख्या, कठोर स्विस ग्रिड और सिर्फ़ एक लाल संकेत वर्ग। 1:1। निष्कर्ष और उपलब्धियों के लिए।",
        "sceneTemplate.item.quoteFrameCard.title" => "उद्धरण कार्ड · रेशम नील-हरित",
        "sceneTemplate.item.quoteFrameCard.summary" => "पीले पड़े रेशम पर फ़्रेम में बँधा एक वाक्य, नीचे अज़ुराइट और मैलाकाइट के पर्वत। 4:5। अंश, साक्षात्कार और उद्धरण के लिए।",
        "sceneTemplate.item.dailySignCard.title" => "दैनिक कार्ड · उद्यान की खिड़की",
        "sceneTemplate.item.dailySignCard.summary" => "चूने की दीवार पर एक षट्कोणीय जाली-खिड़की, भीतर तारीख़ और एक पंक्ति। खालीपन ही अलंकरण है। 3:4। दैनिक पोस्ट के लिए।",
        "sceneTemplate.item.priceTierCard.title" => "मूल्य कार्ड · आर्केड नियॉन",
        "sceneTemplate.item.priceTierCard.summary" => "स्याही-नीली रात पर तीन स्तरों की मूल्य सूची, नियॉन ट्यूब की रूपरेखा और उसका प्रकाश। 1:1। दुकान, आयोजन और पैकेज के लिए।",
        "sceneTemplate.item.noticeBoardCard.title" => "सूचना कार्ड · सीसा टाइप",
        "sceneTemplate.item.noticeBoardCard.summary" => "अख़बारी काग़ज़ पर शीर्ष की दोहरी रेखाएँ, विचलित लाल प्लेट, क्रमांकित खंड और क्रम-मुहर। 4:5। सूचना और नियमों के लिए।",
        "sceneTemplate.item.milestoneTimelineInfographic.title" => "समयरेखा इन्फ़ोग्राफ़िक",
        "sceneTemplate.item.milestoneTimelineInfographic.summary" => "लंबी स्क्रॉल छवि: पूरी ऊँचाई में एक अक्ष, वर्षों के निशान के साथ पड़ाव कार्ड, और अंत में अगला क़दम। समीक्षा, ब्रांड इतिहास और परियोजना यात्रा के लिए।",
        "sceneTemplate.item.conceptContrastInfographic.title" => "अवधारणा तुलना इन्फ़ोग्राफ़िक",
        "sceneTemplate.item.conceptContrastInfographic.summary" => "लंबी स्क्रॉल छवि: पहले निष्कर्ष, फिर दोनों अवधारणाओं के परिभाषा कार्ड, फिर हर पहलू पर दो-स्तंभ तुलना, और अंत में चुनने का आधार।",
        "sceneTemplate.item.rankingBoardInfographic.title" => "टॉप N सूची इन्फ़ोग्राफ़िक",
        "sceneTemplate.item.rankingBoardInfographic.summary" => "लंबी स्क्रॉल छवि: स्याही पर सुनहरी सिफ़ारिश सूची — पहले तीन को बड़े बैज, चौथे से आठवें को रेखा-बैज, हर एक के साथ कब और कितनी बार।",
        "sceneTemplate.item.faqThreadInfographic.title" => "सामान्य प्रश्न इन्फ़ोग्राफ़िक",
        "sceneTemplate.item.faqThreadInfographic.summary" => "लंबी स्क्रॉल छवि: छह प्रश्न-उत्तर जोड़े, प्र भरा और उ रेखांकित। न क्रमांक न क्रम — कोई भी एक जोड़ा अकेले पढ़ा जा सकता है।",
        "sceneTemplate.item.dataStoryInfographic.title" => "डेटा कथा इन्फ़ोग्राफ़िक",
        "sceneTemplate.item.dataStoryInfographic.summary" => "लंबी स्क्रॉल छवि: चार आँकड़े एक कारण-श्रृंखला में पिरोए, हर चरण दस खानों की पट्टी में, और अंत में अमल में लाने लायक़ निष्कर्ष।",
        "sceneTemplate.item.challengeTrackerInfographic.title" => "30 दिन चुनौती इन्फ़ोग्राफ़िक",
        "sceneTemplate.item.challengeTrackerInfographic.summary" => "लंबी स्क्रॉल छवि: छह गुणा पाँच के तीस खाने, पड़ाव सिर्फ़ 7, 15 और 30वें दिन। सहेजिए और रोज़ एक खाना काटिए।",
        "sceneTemplate.item.ecosystemMapInfographic.title" => "उद्योग मानचित्र इन्फ़ोग्राफ़िक",
        "sceneTemplate.item.ecosystemMapInfographic.summary" => "लंबी स्क्रॉल छवि: एक ही शृंखला के चार स्थान दो गुणा दो में, हर खाने में तीन नाम और ख़ाली जगहें चिह्नित। स्लेट पर सफ़ेद कार्ड।",
        "sceneTemplate.item.doDontComparison.title" => "सही और गलत दो स्तंभ",
        "sceneTemplate.item.doDontComparison.summary" => "3:4 कार्ड: एक ही काम के दो तरीके आमने-सामने, लाल-हरे की जगह बनावट और आइकन से भेद — वर्णांध पाठक भी पढ़ सकें।",
        "sceneTemplate.item.mythTruthComparison.title" => "भ्रम और सच",
        "sceneTemplate.item.mythTruthComparison.summary" => "लंबी छवि: «लोग कहते हैं / दरअसल» के पाँच जोड़े, भ्रम बाएँ संकरा और हल्का, सच दाएँ चौड़ा और गहरा।",
        "sceneTemplate.item.pricingTiersComparison.title" => "मूल्य स्तर तुलना",
        "sceneTemplate.item.pricingTiersComparison.summary" => "3:4 कार्ड: फ़्री, प्रो और टीम तीन स्तर साथ-साथ, कीमत ही लंगर है, हर स्तंभ पिछले को समेटता है।",
        "sceneTemplate.item.scenarioGuideComparison.title" => "परिस्थिति अनुसार चयन गाइड",
        "sceneTemplate.item.scenarioGuideComparison.summary" => "लंबी छवि: स्पेक नहीं, सात परिस्थितियाँ और हर एक पर एक फ़ैसला। पाठक को बस अपनी पंक्ति ढूँढनी है।",
        "sceneTemplate.item.specTableComparison.title" => "स्पेक तुलना तालिका",
        "sceneTemplate.item.specTableComparison.summary" => {
            "लंबी छवि: दो विकल्प एक ही तालिका में पंक्ति-दर-पंक्ति, जीतने वाला खाना गहरे तल पर उभरा।"
        }
        "sceneTemplate.item.threeWayComparison.title" => "तीन विकल्पों की तुलना",
        "sceneTemplate.item.threeWayComparison.summary" => "लंबी छवि: तीन विकल्प साथ-साथ, बीच वाला सुझाव; हर स्तंभ नाम से नहीं, एक परिस्थिति से शुरू होता है।",
        "sceneTemplate.item.timeShiftComparison.title" => "एक साल पहले और अब",
        "sceneTemplate.item.timeShiftComparison.summary" => "3:4 कार्ड: बीच में लेबलों की रीढ़, बाएँ एक साल पहले और दाएँ अब, दोनों मान एक ही पंक्ति में।",
        "sceneTemplate.item.tradeoffScaleComparison.title" => "नफ़ा-नुक़सान का तराजू",
        "sceneTemplate.item.tradeoffScaleComparison.summary" => {
            "1:1 कार्ड: एक डंडी और दो पलड़े — बाएँ मूल्य, दाएँ कीमत, हर पंक्ति से पहले एक खाली डिब्बा।"
        }
        "sceneTemplate.item.versionDiffComparison.title" => "संस्करण में बदलाव",
        "sceneTemplate.item.versionDiffComparison.summary" => {
            "1:1 कार्ड: स्तंभ नहीं — हर पंक्ति खुद ही «पुराना → नया» पूरा करती है।"
        }
        "sceneTemplate.item.appOnboardingTriptych.title" => "ऐप ऑनबोर्डिंग त्रिपटल",
        "sceneTemplate.item.appOnboardingTriptych.summary" => "3:4 कार्ड: तीन फ़ोन कतार में, खाली चित्र-स्थान सहित। अपनी तीन स्क्रीन डालिए, टेक्स्ट जोड़िए — समीक्षा या पोस्ट के लिए तैयार।",
        "sceneTemplate.item.diyBlueprintGuide.title" => "DIY सचित्र गाइड",
        "sceneTemplate.item.diyBlueprintGuide.summary" => "लंबी छवि जिसमें सामग्री-तालिका को चरणों जितनी ही जगह मिलती है — DIY हाथों में नहीं, तैयारी में बिगड़ता है।",
        "sceneTemplate.item.photoCompositionTutorial.title" => "मोबाइल फ़ोटो कम्पोज़िशन",
        "sceneTemplate.item.photoCompositionTutorial.summary" => {
            "3:4, पाँच फ़्रेम: हर फ़्रेम एक गहरा व्यूफ़ाइंडर और फ़ोटो-स्थान पर चमकीली गाइड लाइनें।"
        }
        "sceneTemplate.item.recipeFourStep.title" => "चार चरण की रेसिपी कार्ड",
        "sceneTemplate.item.recipeFourStep.summary" => "4:5 कार्ड 2×2: चारों चरण एक ही कार्ड पर। स्क्रीनशॉट लीजिए और बनाइए — चूल्हे के सामने पन्ने पलटना बोझ है।",
        "sceneTemplate.item.skincareRoutineCards.title" => "स्किनकेयर स्टेप कार्ड",
        "sceneTemplate.item.skincareRoutineCards.summary" => "4:5, छह फ़्रेम: हर चरण में तीन संख्याएँ — मात्रा, प्रतीक्षा और सुबह या रात। चूक क्रम में नहीं, मात्रा में होती है।",
        "sceneTemplate.item.softwareStepTutorial.title" => "सॉफ़्टवेयर स्टेप ट्यूटोरियल",
        "sceneTemplate.item.softwareStepTutorial.summary" => {
            "4:5 कार्ड, शृंखला का एकमात्र गहरा: स्क्रीनशॉट स्थान और क्रमांकित निर्देश।"
        }
        "sceneTemplate.item.storageMakeoverSteps.title" => "घर व्यवस्थित करने के चरण",
        "sceneTemplate.item.storageMakeoverSteps.summary" => {
            "3:4, छह फ़्रेम: क्रिया और चित्र के अलावा हर चरण एक पूर्णता-मानक और समय-बजट तय करता है।"
        }
        "sceneTemplate.item.weeklyReportLesson.title" => "साप्ताहिक रिपोर्ट पाठ",
        "sceneTemplate.item.weeklyReportLesson.summary" => {
            "लंबी छवि: चार-भाग की संरचना समझाने के बाद रेखांकित रिक्त स्थानों वाला ढाँचा देती है।"
        }
        "sceneTemplate.item.workoutBreakdownGuide.title" => "व्यायाम विश्लेषण गाइड",
        "sceneTemplate.item.workoutBreakdownGuide.summary" => {
            "लंबी छवि: हर मूवमेंट के साथ सेट / दोहराव / विश्राम की एक निश्चित पट्टी।"
        }
        "sceneTemplate.item.bookreviewSilkCarousel.title" => "पुस्तक/फ़िल्म समीक्षा कैरोसेल",
        "sceneTemplate.item.bookreviewSilkCarousel.summary" => "3:4, पाँच पट: हुक, टिप्पणी सहित उद्धरण, तीन अंतर्दृष्टि, एक याद रहने वाली पंक्ति, समापन। कथा दोहराने के बजाय कृति को ले जाने लायक़ टुकड़ों में बाँटता है।",
        "sceneTemplate.item.cityguideFilmCarousel.title" => "शहर गाइड कैरोसेल",
        "sceneTemplate.item.cityguideFilmCarousel.summary" => "3:4, सात पट: स्थान और मार्ग बारी-बारी — स्थान वाले पट सपने देखने वालों के लिए, दिनभर का मार्ग और खाने-ठहरने की तालिका योजना बनाने वालों के लिए।",
        "sceneTemplate.item.datareportGridCarousel.title" => "डेटा रिपोर्ट कैरोसेल",
        "sceneTemplate.item.datareportGridCarousel.summary" => {
            "3:4, छह पट: हर डेटा पट के बाद एक ग़ैर-डेटा पट, ताकि तीसरे चार्ट पर कोई आगे न खिसक जाए।"
        }
        "sceneTemplate.item.opinionLongformCarousel.title" => "दीर्घ विचार कैरोसेल",
        "sceneTemplate.item.opinionLongformCarousel.summary" => {
            "3:4, छह पट: पूरे सेट में एक कठोर विज़ुअल मास्टर, पृष्ठ संख्या और शीर्षक हमेशा एक ही जगह।"
        }
        "sceneTemplate.item.qaChalkboardCarousel.title" => "प्रश्नोत्तर कैरोसेल",
        "sceneTemplate.item.qaChalkboardCarousel.summary" => {
            "3:4, छह पट: एक पट एक प्रश्न, हर कोने में हाथ से लिखा प्रश्नचिह्न क्रमांक।"
        }
        "sceneTemplate.item.storyNightCarousel.title" => "कथा कैरोसेल",
        "sceneTemplate.item.storyNightCarousel.summary" => "3:4, सात पट: समय के ढाँचे पर बना निजी अनुभव का पुनरावलोकन — पाँचवें पट की समयरेखा पूरे सेट की भार वहन करती है।",
        "sceneTemplate.item.toolkitNotebookCarousel.title" => "संसाधन संग्रह कैरोसेल",
        "sceneTemplate.item.toolkitNotebookCarousel.summary" => {
            "3:4, छह पट: छह उपकरण एक-एक पट पर, आख़िरी पट पर पृष्ठ संख्या सहित सूची।"
        }
        "sceneTemplate.item.tutorialJournalCarousel.title" => "ट्यूटोरियल कैरोसेल",
        "sceneTemplate.item.tutorialJournalCarousel.summary" => {
            "3:4, छह पट: एक पट एक चरण, उँगली ही प्रगति पट्टी है।"
        }
        "sceneTemplate.item.yearreviewMineralCarousel.title" => "वार्षिक समीक्षा कैरोसेल",
        "sceneTemplate.item.yearreviewMineralCarousel.summary" => {
            "3:4, आठ पट: आँकड़ों वाले पट ठंडे, अनुभव वाले पट गर्म, बारी-बारी।"
        }
        "fileMenu.newFromTemplate" => "टेम्पलेट से नया बनाएँ",
        "collab.ownerConfirm.title" => "पुष्टि करें कि आप किससे जुड़ रहे हैं",
        "collab.ownerConfirm.hint" => "इस सत्र से अभी तक कुछ भी लोड नहीं हुआ है।",
        "collab.ownerConfirm.account" => "सत्यापित खाता",
        "collab.ownerConfirm.device" => "सत्यापित डिवाइस",
        "collab.ownerConfirm.claimedName" => "इस खाते द्वारा चुना गया नाम (सत्यापित नहीं)",
        "collab.action.confirmOwner" => "इस सत्र में शामिल हों",
        "collab.action.rejectOwner" => "शामिल न हों",
        "collab.error.ownerNotConfirmed" => "आपने होस्ट की पुष्टि नहीं की, इसलिए कुछ भी लोड नहीं हुआ।",
        "fileMenu.exportSlideshowHtml" => "स्लाइडशो HTML निर्यात करें...",
        "fileMenu.exportPptx" => "PowerPoint निर्यात करें...",
        "dialog.slideshowHtmlTitle" => "स्लाइडशो निर्यात करें",
        "dialog.slideshowHtmlSummary" => "{{count}} स्लाइड यहाँ निर्यात की गईं:",
        "dialog.slideshowHtmlEmpty" => "इस प्रस्तुति में निर्यात करने योग्य कोई स्लाइड नहीं है।",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "आयात योग्य HTML सामग्री उपलब्ध नहीं है।",
        "htmlImport.warn.content.empty_body" => "HTML body में आयात योग्य सामग्री उपलब्ध नहीं है।",
        "htmlImport.warn.content.dom_depth_truncated" => {
            "{{max_depth}} स्तरों से अधिक गहराई में नेस्ट किए गए HTML को हटा दिया गया।"
        }
        "htmlImport.warn.content.node_limit_truncated" => {
            "नोड सीमा पूरी हो गई; बाक़ी पेज सामग्री को छोड़ दिया गया।"
        }
        "htmlImport.warn.content.node_limit_mapping" => {
            "नोड सीमा पूरी हो गई; HTML ट्री के एक भाग को छोड़ दिया गया।"
        }
        "htmlImport.warn.content.node_limit_inline_row" => {
            "नोड सीमा पूरी हो गई; एक इनलाइन फ़ॉर्मेटिंग पंक्ति को छोड़ दिया गया।"
        }
        "htmlImport.warn.content.node_limit_pseudo" => {
            "नोड सीमा पूरी हो गई; उत्पन्न किए गए स्यूडो-एलिमेंट को छोड़ दिया गया।"
        }
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "{{max_depth}} at-rule से अधिक गहराई में नेस्ट किए गए CSS नियमों को अनदेखा किया गया।"
        }
        "htmlImport.warn.css.unterminated_rule" => "एक असमाप्त CSS नियम को अनदेखा किया गया।",
        "htmlImport.warn.css.marker_rules_unsupported" => {
            "CSS ::marker नियमों को आयात नहीं किया गया।"
        }
        "htmlImport.warn.css.nesting_unsupported" => {
            "नेस्ट किए गए CSS स्टाइल नियमों को अनदेखा किया गया।"
        }
        "htmlImport.warn.css.invalid_layer_name" => {
            "अमान्य @layer नाम '{{name}}' को अनदेखा किया गया।"
        }
        "htmlImport.warn.css.unsupported_statement" => {
            "असमर्थित @{{name}} स्टेटमेंट को अनदेखा किया गया।"
        }
        "htmlImport.warn.css.media_without_viewport" => {
            "व्यूपोर्ट रहित @media नियमों को अनदेखा किया गया।"
        }
        "htmlImport.warn.css.invalid_layer_block_name" => {
            "अमान्य @layer ब्लॉक नाम '{{name}}' को अनदेखा किया गया।"
        }
        "htmlImport.warn.css.unsupported_container_block" => "@container ब्लॉक को अनदेखा किया गया।",
        "htmlImport.warn.css.unsupported_block" => "असमर्थित @{{name}} ब्लॉक को अनदेखा किया गया।",
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "@font-face वेब फ़ॉन्ट '{{family}}' उपलब्ध नहीं है।"
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "निरपेक्ष स्थिति वाले एलिमेंट के प्रतिशत ऑफ़सेट को अनुमानित किया गया।"
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "प्रतिशत position:relative ऑफ़सेट को अनुमानित किया गया।"
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "निश्चित अक्ष के बिना CSS aspect-ratio को अनदेखा किया गया।"
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "अनिश्चित कंटेनिंग ब्लॉक के भीतर CSS aspect-ratio को अनदेखा किया गया।"
        }
        "htmlImport.warn.layout.position_sticky_ignored" => {
            "CSS position:sticky को अनदेखा किया गया।"
        }
        "htmlImport.warn.layout.grid_tracks_approximated" => {
            "असमर्थित CSS grid ट्रैक को अनुमानित किया गया।"
        }
        "htmlImport.warn.layout.float_ignored" => "CSS float को अनदेखा किया गया।",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "नोड स्तर पर CSS mix-blend-mode को अनुमानित किया गया।"
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "CSS overflow: auto / scroll को अनुमानित किया गया।"
        }
        "htmlImport.warn.layout.negative_margins_ignored" => {
            "ऋणात्मक CSS मार्जिन को अनदेखा किया गया।"
        }
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => {
            "विज़ुअल बॉक्स पर CSS मार्जिन को अनदेखा किया गया।"
        }
        "htmlImport.warn.layout.inline_margin_wrapping_approximated" => "CSS मार्जिन वाले इनलाइन तत्व को बॉक्स बनाया गया है और वह अब पंक्तियों में रैप नहीं हो सकता।",
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "content-box प्रतिशत आकार-निर्धारण को अनुमानित किया गया।"
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "स्पष्ट प्रारंभ रेखाओं से बचे खाली CSS grid सेल को अनुमानित किया गया।"
        }
        "htmlImport.warn.layout.grid_span_reflowed" => {
            "जिस CSS grid आइटम का विस्तार उसकी प्रारंभ रेखा में नहीं समाया, उसे अनुमानित किया गया।"
        }
        "htmlImport.warn.layout.grid_rows_node_limit" => {
            "नोड सीमा पूरी हो गई; CSS grid पंक्ति रैपर को छोड़ दिया गया।"
        }
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "auto-fit / auto-fill वाली CSS grid ट्रैक चौड़ाइयों को अनुमानित किया गया।"
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "CSS grid-template-areas स्थान-निर्धारण को आयात नहीं किया गया।"
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => {
            "CSS grid-row स्थान-निर्धारण को आयात नहीं किया गया।"
        }
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "CSS grid-column `{{value}}` को अनुमानित किया गया।"
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => {
            "CSS ब्लॉक-अक्ष स्वतः मार्जिन को आयात नहीं किया गया।"
        }
        "htmlImport.warn.layout.auto_margin_node_limit" => {
            "नोड सीमा पूरी हो गई; CSS स्वतः-मार्जिन संरेखण को छोड़ दिया गया।"
        }
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "निश्चित आकार रहित एलिमेंट पर CSS इन-फ़्लो ऑफ़सेट को हटा दिया गया।"
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => {
            "नोड सीमा पूरी हो गई; एक CSS इन-फ़्लो ऑफ़सेट को छोड़ दिया गया।"
        }
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "CSS इन-फ़्लो ऑफ़सेट (position:relative इनसेट, ट्रांसफ़ॉर्म स्थानांतरण) को अनुमानित किया गया।"
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "ऑफ़सेट रैपर न रख सकने वाले बॉक्स पर CSS इन-फ़्लो ऑफ़सेट को हटा दिया गया।"
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "कॉलम flex कंटेनर पर flex-wrap को आयात नहीं किया गया।"
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => {
            "flex-wrap:wrap-reverse को अनुमानित किया गया।"
        }
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "निश्चित चौड़ाई रहित कंटेनर पर flex-wrap को अनदेखा किया गया।"
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "रैप होने वाले flex कंटेनर पर CSS align-content को आयात नहीं किया गया।"
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "अनिश्चित चाइल्ड मुख्य-अक्ष आकारों वाले flex-wrap को अनदेखा किया गया।"
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => {
            "नोड सीमा पूरी हो गई; flex-wrap पंक्तियों को छोड़ दिया गया।"
        }
        "htmlImport.warn.transform.unsupported_syntax" => {
            "असमर्थित CSS ट्रांसफ़ॉर्म सिंटैक्स को अनदेखा किया गया।"
        }
        "htmlImport.warn.transform.unsupported_function" => {
            "असमर्थित CSS ट्रांसफ़ॉर्म फ़ंक्शन (3D, matrix3d) को अनदेखा किया गया।"
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "अनिश्चित अक्ष पर प्रतिशत CSS ट्रांसफ़ॉर्म स्थानांतरण को हटा दिया गया।"
        }
        "htmlImport.warn.transform.non_finite_matrix" => {
            "अपरिमित मैट्रिक्स उत्पन्न करने वाले CSS ट्रांसफ़ॉर्म को अनदेखा किया गया।"
        }
        "htmlImport.warn.transform.skew_dropped" => "CSS ट्रांसफ़ॉर्म skew को हटा दिया गया।",
        "htmlImport.warn.transform.degenerate_scale" => {
            "शून्य या अपरिमित स्केल वाले CSS ट्रांसफ़ॉर्म को अनुमानित किया गया।"
        }
        "htmlImport.warn.transform.mirroring_absolute" => {
            "CSS ट्रांसफ़ॉर्म प्रतिबिंबन को अनुमानित किया गया।"
        }
        "htmlImport.warn.transform.origin_z_ignored" => {
            "CSS transform-origin के Z ऑफ़सेट को अनदेखा किया गया।"
        }
        "htmlImport.warn.transform.scale_not_baked" => {
            "नोड आकार में समाहित न किए जा सकने वाले CSS ट्रांसफ़ॉर्म स्केल को हटा दिया गया।"
        }
        "htmlImport.warn.transform.scale_baked" => {
            "नोड आकार में समाहित CSS ट्रांसफ़ॉर्म स्केल को अनुमानित किया गया।"
        }
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "स्वतः-आकार वाले एलिमेंट पर CSS ट्रांसफ़ॉर्म स्केल को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "दिशात्मक या अंतराल वाले CSS background-repeat को अनुमानित किया गया।"
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => {
            "स्पष्ट रूप से दिए गए CSS बैकग्राउंड टाइल आकार को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.background_size_auto_box" => {
            "स्वतः-आकार वाले एलिमेंट पर CSS background-size को अनुमानित किया गया।"
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "छवि के आंतरिक आकार पर निर्भर CSS background-size को अनुमानित किया गया।"
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "असमर्थित CSS background-position को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.background_image_url_empty" => {
            "खाली CSS बैकग्राउंड छवि URL को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.conic_gradient_ignored" => {
            "CSS शंक्वाकार ग्रेडिएंट को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "असमर्थित CSS background-image परत को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.background_color_unresolved" => {
            "अनिर्धारित CSS बैकग्राउंड रंग को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.background_position_dropped" => {
            "CSS background-position को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.border_colors_approximated" => {
            "प्रति-भुजा CSS बॉर्डर रंगों को अनुमानित किया गया।"
        }
        "htmlImport.warn.visual.border_styles_approximated" => {
            "मिश्रित प्रति-भुजा CSS बॉर्डर शैलियों को अनुमानित किया गया।"
        }
        "htmlImport.warn.visual.border_style_complex" => {
            "एक जटिल CSS बॉर्डर शैली को अनुमानित किया गया।"
        }
        "htmlImport.warn.visual.border_style_unsupported" => {
            "असमर्थित CSS बॉर्डर शैली को अनुमानित किया गया।"
        }
        "htmlImport.warn.visual.border_radius_elliptical" => {
            "दीर्घवृत्तीय CSS बॉर्डर त्रिज्याओं को अनुमानित किया गया।"
        }
        "htmlImport.warn.visual.border_radius_unsupported" => {
            "असमर्थित CSS बॉर्डर त्रिज्या को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "असमर्थित CSS box-shadow परत को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => {
            "CSS ग्रेडिएंट रंग प्रक्षेप विधि को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "असमर्थित CSS linear-gradient दिशा को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => {
            "CSS ग्रेडिएंट रंग संकेतों को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => {
            "असमर्थित CSS ग्रेडिएंट रंग स्टॉप को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.gradient_too_few_stops" => {
            "दो से कम उपयोग योग्य स्टॉप वाले CSS ग्रेडिएंट को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.gradient_repeating_approximated" => {
            "दोहराए जाने वाले CSS ग्रेडिएंट को अनुमानित किया गया।"
        }
        "htmlImport.warn.visual.gradient_stops_clamped" => {
            "सीमा से बाहर के CSS ग्रेडिएंट स्टॉप को अनुमानित किया गया।"
        }
        "htmlImport.warn.visual.blur_radius_unsupported" => {
            "असमर्थित CSS ब्लर त्रिज्या को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "असमर्थित CSS फ़िल्टर drop-shadow() को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.filter_function_unsupported" => {
            "असमर्थित CSS फ़िल्टर फ़ंक्शन को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "असमर्थित CSS backdrop-filter फ़ंक्शन को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "असमर्थित CSS background-blend-mode को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "अलग-अलग फ़िल पर CSS mix-blend-mode को अनुमानित किया गया।"
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "असमर्थित CSS mix-blend-mode को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.property_not_representable" => {
            "CSS {{property}} को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "ग्रेडिएंट पर CSS background-size को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "असमर्थित CSS radial-gradient स्थिति को अनदेखा किया गया।"
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "दीर्घवृत्तीय CSS radial-gradient को अनुमानित किया गया।"
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "CSS radial-gradient विस्तार कीवर्ड को अनुमानित किया गया।"
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "असमर्थित CSS radial-gradient आकार को अनदेखा किया गया।"
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => {
            "असमर्थित CSS text-shadow परत को अनदेखा किया गया।"
        }
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "पहली के बाद की CSS text-shadow परतों को अनदेखा किया गया।"
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => {
            "इनलाइन एलिमेंट पर CSS text-shadow को अनदेखा किया गया।"
        }
        "htmlImport.warn.list.style_image_ignored" => "CSS list-style-image को आयात नहीं किया गया।",
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "`list-style-position: outside` वाले लटकते मार्कर को अनुमानित किया गया।"
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "असमर्थित CSS list-style-type `{{value}}` को अनुमानित किया गया।"
        }
        "htmlImport.warn.media.object_fit_scale_down" => {
            "CSS object-fit:scale-down को अनुमानित किया गया।"
        }
        "htmlImport.warn.media.object_fit_none_ignored" => {
            "CSS object-fit:none को अनदेखा किया गया।"
        }
        "htmlImport.warn.media.object_position_ignored" => {
            "CSS object-position को अनदेखा किया गया।"
        }
        "htmlImport.warn.media.image_intrinsic_axis_unresolved" => {
            "छवि का अंतर्निहित पक्षानुपात अनुपस्थित अक्ष निर्धारित नहीं कर सका, क्योंकि निर्धारित आकार डायनेमिक है या उसका कंटेनिंग ब्लॉक अनिश्चित है।"
        }
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "छवि पर असमर्थित CSS mix-blend-mode को अनदेखा किया गया।"
        }
        "htmlImport.warn.media.inline_svg_placeholder" => {
            "एक इनलाइन <svg> एलिमेंट को प्लेसहोल्डर के रूप में आयात किया गया।"
        }
        "htmlImport.warn.media.input_type_fallback" => {
            "असमर्थित <input> प्रकार को अनुमानित किया गया।"
        }
        "htmlImport.warn.media.element_placeholder" => {
            "<{{tag}}> एलिमेंट को प्लेसहोल्डर के रूप में आयात किया गया।"
        }
        "htmlImport.warn.media.picture_undecodable_types" => {
            "केवल डिकोड न हो सकने वाले स्रोत प्रकारों वाले <picture> को अनुमानित किया गया।"
        }
        "htmlImport.warn.table.rowspan_ignored" => "HTML rowspan एट्रिब्यूट को आयात नहीं किया गया।",
        "htmlImport.warn.table.row_groups_unflattened" => {
            "जिस टेबल के पंक्ति समूह CSS ने सपाट नहीं किए, उसकी कॉलम चौड़ाइयों को अनुमानित किया गया।"
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "निश्चित चौड़ाई रहित CSS टेबल की कॉलम चौड़ाइयों को अनुमानित किया गया।"
        }
        "htmlImport.warn.resource.invalid_base_href" => {
            "अमान्य <base href> {{href}} को अनदेखा किया गया।"
        }
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "प्रोजेक्ट मूल के बाहर के <base href> {{href}} को अनदेखा किया गया।"
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => {
            "बाहरी स्टाइलशीट {{url}} उपलब्ध नहीं है।"
        }
        "htmlImport.warn.resource.image_outside_origin" => {
            "प्रोजेक्ट मूल के बाहर की छवि {{url}} को प्लेसहोल्डर के रूप में आयात किया गया।"
        }
        "htmlImport.warn.resource.image_unavailable" => {
            "अनुपलब्ध छवि {{url}} को प्लेसहोल्डर के रूप में आयात किया गया।"
        }
        "htmlImport.warn.resource.css_import_invalid" => {
            "अमान्य CSS @import {{prelude}} को अनदेखा किया गया।"
        }
        "htmlImport.warn.resource.css_import_unresolvable" => {
            "CSS @import {{reference}} उपलब्ध नहीं है।"
        }
        "htmlImport.warn.resource.css_import_cycle" => {
            "चक्रीय CSS @import {{url}} को अनदेखा किया गया।"
        }
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "गहराई {{max_depth}} से आगे के CSS @import {{url}} को अनदेखा किया गया।"
        }
        "htmlImport.warn.resource.css_import_unavailable" => "CSS @import {{url}} उपलब्ध नहीं है।",
        "htmlImport.warn.project.multiple_html_entries" => {
            "{{count}} HTML प्रविष्टियाँ मिलीं; {{entry}} चुनी गई और बाक़ी को अनुमानित किया गया।"
        }
        "htmlImport.warn.snapshot.truncated" => "ब्राउज़र स्नैपशॉट के एक भाग को हटा दिया गया।",
        "htmlImport.warn.snapshot.node_limit" => {
            "नोड सीमा पूरी हो गई; बाक़ी स्नैपशॉट सामग्री को छोड़ दिया गया।"
        }
        "htmlImport.warn.snapshot.tainted_images" => {
            "रिमोट URL के रूप में रखी गईं {{count}} CORS-दूषित छवियाँ उपलब्ध नहीं हैं।"
        }
        "htmlImport.warn.snapshot.invalid_rect" => {
            "अनुपस्थित या अमान्य रेक्ट वाले स्नैपशॉट नोड को हटा दिया गया।"
        }
        "htmlImport.warn.snapshot.unknown_kind" => "अज्ञात प्रकार के स्नैपशॉट नोड को हटा दिया गया।",
        "htmlImport.warn.snapshot.rejected" => "ब्राउज़र स्नैपशॉट ({{reason}}) को हटा दिया गया।",
        "htmlImport.warn.snapshot.unsupported_transform" => {
            "असमर्थित स्नैपशॉट ट्रांसफ़ॉर्म को अनदेखा किया गया।"
        }
        "htmlImport.warn.css.media_empty_query" => "खाली @media क्वेरी को अनदेखा किया गया।",
        "htmlImport.warn.css.media_unsupported_type" => {
            "असमर्थित @media प्रकार '{{name}}' को अनदेखा किया गया।"
        }
        "htmlImport.warn.css.media_unsupported_condition" => {
            "असमर्थित @media शर्त '{{input}}' को अनदेखा किया गया।"
        }
        "htmlImport.warn.css.media_invalid_orientation" => {
            "अमान्य @media अभिविन्यास '{{value}}' को अनदेखा किया गया।"
        }
        "htmlImport.warn.css.media_unsupported_feature" => {
            "असमर्थित @media विशेषता '{{name}}' को अनदेखा किया गया।"
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "असमर्थित @media रेंज '({{input}})' को अनदेखा किया गया।"
        }
        "htmlImport.warn.css.media_invalid_range" => {
            "अमान्य @media रेंज '({{input}})' को अनदेखा किया गया।"
        }
        "htmlImport.warn.css.media_invalid_length" => {
            "अमान्य @media लंबाई '{{value}}' को अनदेखा किया गया।"
        }
        "htmlImport.diagnostics.title" => "HTML आयात पूर्ण हुआ",
        "htmlImport.diagnostics.summary" => "गुणवत्ता-ह्रास वाले आइटम: {{count}}",
        "htmlImport.diagnostics.dismiss" => "खारिज करें",
        "htmlImport.diagnostics.expand" => "विवरण दिखाएँ",
        "htmlImport.diagnostics.collapse" => "विवरण छिपाएँ",
        "htmlImport.diagnostics.more" => "+{{count}} और",
        "dialog.pptxTitle" => "PowerPoint निर्यात करें",
        "dialog.pptxSummary" => "{{count}} स्लाइड यहाँ निर्यात की गईं:",
        "dialog.pptxEmpty" => "इस प्रस्तुति में निर्यात करने योग्य कोई स्लाइड नहीं है।",
        "settings.agents.acpQuickAdd" => "त्वरित जोड़",
        "settings.agents.acpPresetAdd" => "जोड़ें",
        "settings.agents.acpNotInstalled" => "इंस्टॉल नहीं है",
        "assetCenter.title" => "एसेट सेंटर",
        "assetCenter.tab.templates" => "टेम्पलेट",
        "assetCenter.tab.styles" => "शैलियाँ",
        "assetCenter.style.empty" => "कोई मेल खाती शैली नहीं",
        "assetCenter.style.pinned" => "पिन किया गया",
        "assetCenter.style.searchPlaceholder" => "शैलियाँ या टैग खोजें",
        "assetCenter.style.generateHint" => "आपके विषय से नया दस्तावेज़, पिन की गई शैली में।",
        "ai.pinnedStyle" => "शैली: {{name}}",
        "assetCenter.style.import" => "शैली आयात करें",
        "assetCenter.style.mine" => "मेरी शैलियाँ",
        "assetCenter.style.builtIn" => "अंतर्निहित शैलियाँ",
        "assetCenter.style.importTitle" => "DESIGN.md आयात करें",
        "assetCenter.style.importHint" => "पूरा DESIGN.md चिपकाएँ, फिर आयात की पुष्टि करें।",
        "assetCenter.style.importSource" => "आप styles.refero.design जैसी DESIGN.md लाइब्रेरी से शैली कॉपी कर सकते हैं।",
        "assetCenter.style.importConfirm" => "आयात करें",
        "assetCenter.style.importCancel" => "रद्द करें",
        "assetCenter.style.importPickFile" => "फ़ाइल चुनें…",
        "assetCenter.style.importHintFile" => "DESIGN.md फ़ाइल चुनें, या पूरा दस्तावेज़ नीचे चिपकाएँ।",
        "assetCenter.style.importPlaceholder" => "अपना DESIGN.md यहाँ चिपकाएँ",
        "assetCenter.style.importEmpty" => "यह फ़ाइल खाली है, या शैली मार्गदर्शिका के लिए बहुत छोटी है।",
        "assetCenter.style.importNotText" => "यह फ़ाइल Markdown पाठ के रूप में नहीं पढ़ी जा सकती।",
        "assetCenter.style.importTooLarge" => "यह फ़ाइल 512 KB से बड़ी है।",
        "slidesPanel.tabSlides" => "स्लाइड",
        "slidesPanel.tabCards" => "कार्ड",
        "slidesPanel.present" => "प्रस्तुत करें",
        "slidesPanel.exportPdf" => "PDF निर्यात करें",
        "slidesPanel.exportAllSlides" => "सभी स्लाइड निर्यात करें",
        "slidesPanel.exportSelectedSlides" => "चयनित स्लाइड निर्यात करें ({{count}})",
        "settings.tab.ai" => "AI",
        "settings.agents.heroTitle" => "अपना AI प्रदाता कनेक्ट करें",
        "settings.agents.heroSubtitle" => "OpenPencil आपके स्थानीय CLI एजेंट और API प्रदाताओं को चलाता है — डिज़ाइन बनाना शुरू करने के लिए किसी एक को कनेक्ट करें।",
        "settings.agents.statusConnected" => "कनेक्टेड",
        "settings.agents.statusNotConnected" => "कनेक्ट नहीं है",
        "settings.agents.statusChecking" => "स्थिति जाँच रहे हैं…",
        "settings.mcp.heroTitle" => "बाहर से MCP के ज़रिए OpenPencil जोड़ें",
        "settings.mcp.heroSubtitle" => "MCP समझने वाले किसी भी CLI या एडिटर को इस वर्कस्पेस पर लगाइए और वही टूल इस्तेमाल कीजिए जो अंदरूनी एजेंट करता है।",
        "settings.mcp.terminalFootnote" => "* शुरू होते समय चुने गए CLI टूल के लिए MCP अपने आप सेट हो जाता है।",
        "settings.mcp.customConfigTitle" => "कस्टम MCP सर्वर कॉन्फ़िगरेशन",
        "settings.mcp.customConfigDesc" => "इसे किसी भी ऐसे क्लाइंट में पेस्ट करें जो मानक MCP server ब्लॉक पढ़ता हो।",
        "settings.mcp.copyConfig" => "MCP कॉन्फ़िग कॉपी करें",
        "settings.system.heroTitle" => "सिस्टम प्राथमिकताएँ",
        "settings.system.heroSubtitle" => "इस इंस्टॉल के लिए रूप, अपडेट और कैनवास व्यवहार।",
        "settings.system.appearance" => "रूप",
        "settings.system.appearanceLight" => "हल्का",
        "settings.system.appearanceDark" => "गहरा",
        "settings.system.pencilCursor" => "पेंसिल कर्सर",
        "settings.images.heroTitle" => "आपके डिज़ाइन के लिए इमेज",
        "settings.images.heroSubtitle" => "Openverse पर फ़ोटो खोजें, या ज़रूरत पर बनाने के लिए कोई प्रदाता जोड़ें।",
        "settings.fonts.heroTitle" => "इस दस्तावेज़ के फ़ॉन्ट",
        "settings.fonts.heroSubtitle" => "दस्तावेज़ जो फ़ॉन्ट माँगता है पर इस मशीन में नहीं हैं, उन्हें हल करें और आयात किए फ़ॉन्ट प्रबंधित करें।",
        "settings.account.heroTitle" => "आपका खाता",
        "settings.account.heroSubtitle" => "साइन इन करके अपना वर्कस्पेस और लाइसेंस सभी डिवाइस पर सिंक करें।",
        "tooltip.topbar.file" => "फ़ाइल",
        "tooltip.topbar.import" => "आयात",
        "tooltip.topbar.language" => "भाषा",
        "tooltip.topbar.collaboration" => "सहयोग",
        "tooltip.topbar.preview" => "पूर्वावलोकन",
        "tooltip.topbar.exitPreview" => "पूर्वावलोकन बंद करें",
        "tooltip.topbar.account" => "खाता",
        "settings.agents.providerRollMore" => "और {{count}} अन्य",
        "ai.thinking.adaptive" => "सोच: स्वतः",
        "ai.thinking.disabled" => "सोच: बंद",
        "ai.thinking.enabled" => "सोच: चालू",
        "ai.designProgress.detail.repairsApplied" => "{{count}} स्वतः सुधार लागू",
        "ai.designProgress.detail.repairsMore" => "… और {{count}} अधिक (लॉग देखें)",
        "ai.styleCard.builtin" => "अंतर्निहित शैली",
        "ai.styleCard.imported" => "आयातित DESIGN.md",
        "ai.styleCard.documentDesignMd" => "दस्तावेज़ design.md",
        _ => return super::hi_collab::lookup(key),
    })
}
