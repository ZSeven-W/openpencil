//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `tr_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Görsel ara…",
        "imagePanel.searching" => "Aranıyor…",
        "imagePanel.noResults" => "Sonuç bulunamadı",
        "imagePanel.searchPrompt" => "Görsel arayın",
        "imagePanel.sourceNotice" => "Görseller {{source}} kaynağından. Özgür lisanslı — kullanmadan önce lisansı doğrulayın.",
        "imagePanel.genNotConfigured" => "Görsel oluşturma yapılandırılmamış",
        "imagePanel.openSettings" => "Ayarları Aç",
        "imagePanel.promptPlaceholder" => "Görseli tanımlayın…",
        "providerProbe.connectedViaCli" => "{{name}} CLI üzerinden bağlanıldı",
        "providerProbe.cliExitedWithError" => "{{name}} CLI bir hatayla sonlandı",
        "providerProbe.cliNoVersionOutput" => "{{name}} CLI sürüm bilgisi üretmedi",
        "providerProbe.modelQueryFailed" => "{{name}} model sorgusu başarısız oldu veya zaman aşımına uğradı",
        "providerProbe.modelQueryFailedRunLogin" => "{{name}} model sorgusu başarısız oldu. Kimlik doğrulamak için {{command}} komutunu bir kez çalıştırın.",
        "providerProbe.modelQueryNeedsAuth" => "{{name}} model sorgusu kimlik doğrulama gerektiriyor. Oturum açmak için {{command}} komutunu bir kez çalıştırın.",
        "providerProbe.unrecognizedModelCatalog" => "{{name}} tanınmayan bir model kataloğu döndürdü",
        _ => return super::tr_collab::lookup(key),
    })
}
