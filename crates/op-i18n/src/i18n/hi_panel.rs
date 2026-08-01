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
        "sceneTemplate.filter.all" => "सभी",
        "sceneTemplate.scene.tutorial" => "ट्यूटोरियल चित्र",
        "sceneTemplate.scene.comparison" => "तुलना चित्र",
        "sceneTemplate.scene.carousel" => "ज्ञान कार्ड",
        "sceneTemplate.scene.slides" => "PPT",
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
        "fileMenu.newFromTemplate" => "टेम्पलेट से नया बनाएँ",
        "collab.ownerConfirm.title" => "पुष्टि करें कि आप किससे जुड़ रहे हैं",
        "collab.ownerConfirm.hint" => "इस सत्र से अभी तक कुछ भी लोड नहीं हुआ है।",
        "collab.ownerConfirm.account" => "सत्यापित खाता",
        "collab.ownerConfirm.device" => "सत्यापित डिवाइस",
        "collab.ownerConfirm.claimedName" => "इस खाते द्वारा चुना गया नाम (सत्यापित नहीं)",
        "collab.action.confirmOwner" => "इस सत्र में शामिल हों",
        "collab.action.rejectOwner" => "शामिल न हों",
        "collab.error.ownerNotConfirmed" => "आपने होस्ट की पुष्टि नहीं की, इसलिए कुछ भी लोड नहीं हुआ।",
        _ => return super::hi_collab::lookup(key),
    })
}
