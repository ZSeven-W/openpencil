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
        _ => return super::hi_collab::lookup(key),
    })
}
