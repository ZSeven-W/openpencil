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
        "promptCenter.title" => "İstem Merkezi",
        "promptCenter.searchPlaceholder" => "İstemlerde ara…",
        "promptCenter.category.all" => "Tümü",
        "promptCenter.category.starter" => "Hızlı başlangıç",
        "promptCenter.category.mobileApp" => "Mobil uygulama",
        "promptCenter.category.webPage" => "Web sayfası",
        "promptCenter.category.dashboard" => "Panolar",
        "promptCenter.category.component" => "Bileşenler",
        "promptCenter.category.modify" => "Düzenleme",
        "promptCenter.category.custom" => "Benimkiler",
        "promptCenter.empty" => "Eşleşen istem bulunamadı",
        "promptCenter.saveCurrent" => "Mevcut girdiyi istem olarak kaydet",
        "promptCenter.saveTitlePlaceholder" => "İstem başlığını girin",
        "promptCenter.save" => "Kaydet",
        "promptCenter.cancel" => "İptal",
        "promptCenter.delete" => "Sil",
        "promptCenter.screens" => "{{count}} ekran",
        "promptCenter.freeform" => "Serbest biçim",
        "promptCenter.item.wander.title" => "Wander · Seyahat planı",
        "promptCenter.item.forage.title" => "Forage · Mevsimlik tarifler",
        "promptCenter.item.still.title" => "Still · Meditasyon ve uyku",
        "promptCenter.item.hearth.title" => "Hearth · Akıllı ev",
        "promptCenter.item.meteo.title" => "Meteo · Sürükleyici hava durumu",
        "promptCenter.item.marginalia.title" => "Marginalia · Okuma ve not alma",
        "promptCenter.item.lingua.title" => "Lingua · Dil öğrenme",
        "promptCenter.item.daybreak.title" => "Daybreak · Kahve siparişi",
        "promptCenter.item.verdant.title" => "Verdant · Bitki bakımı",
        "promptCenter.item.companion.title" => "Companion · Evcil hayvan yaşamı",
        "promptCenter.item.relic.title" => "Relic · Seçkin ikinci el pazarı",
        "promptCenter.item.nocturne.title" => "Nocturne · Yıldız gözlem rehberi",
        "promptCenter.item.marquee.title" => "Marquee · Film izleme listesi",
        "promptCenter.item.ritual.title" => "Ritual · Alışkanlık kazanma",
        "promptCenter.item.ember.title" => "Ember · Duygu günlüğü",
        "promptCenter.item.volt.title" => "Volt · Elektrikli araç asistanı",
        "promptCenter.item.aloft.title" => "Aloft · Uçuş takibi",
        "promptCenter.item.gallery.title" => "Gallery · Sergiler ve kültür",
        "promptCenter.item.nightcap.title" => "Nightcap · Evde kokteyl",
        "promptCenter.item.bloom.title" => "Bloom · Çocuk gelişim günlüğü",
        "promptCenter.item.extremeWeather.title" => "Hava durumu uygulaması · Beni şaşırt",
        "promptCenter.item.extremeNowPlaying.title" => "Şimdi çalıyor · Yayına hazır güzellikte",
        "promptCenter.item.extremeDailyApp.title" => "Her gün açmak isteyeceğin uygulama",
        "promptCenter.item.extremeCalendar.title" => "Takvimi baştan tasarla",
        "promptCenter.item.extremeCalm.title" => "Tek ekranda huzur",
        "promptCenter.item.webOrbit.title" => "Orbit · Yapay zekâ çalışma alanı açılış sayfası",
        "promptCenter.item.webAtelier.title" => "Atelier · Mobilya e-ticareti",
        "promptCenter.item.dashboardPulse.title" => "Pulse · Büyüme analitiği panosu",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · Lojistik operasyonları",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · Kurumsal veri tablosu",
        "promptCenter.item.componentFormLab.title" => "Form Lab · Form bileşeni sistemi",
        "promptCenter.item.modifyPolishCurrent.title" => "Mevcut ekranı iyileştir",
        "promptCenter.item.modifyCompleteStates.title" => "Bileşen durumlarını tamamla",
        "sceneTemplate.title" => "Sahne Şablonları",
        "sceneTemplate.searchPlaceholder" => "Sahne veya şablon ara…",
        "sceneTemplate.empty" => "Eşleşen şablon bulunamadı",
        "sceneTemplate.frames" => "{{count}} sayfa",
        "sceneTemplate.filter.all" => "Tümü",
        "sceneTemplate.scene.tutorial" => "Eğitim görseli",
        "sceneTemplate.scene.comparison" => "Karşılaştırma görseli",
        "sceneTemplate.scene.carousel" => "Bilgi kartları",
        "sceneTemplate.scene.slides" => "PPT",
        "sceneTemplate.item.screenshotTutorial.title" => {
            "Üç adımlı ekran görüntülü eğitim kartı"
        }
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "Kapak, üç işlem adımı ve son harekete geçirici mesaj; ekran görüntülerini ve metni değiştirip yayımlayın."
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "Bilgi ve içgörü karuseli",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "Kapak, üç ana fikir ve özet sayfası; tek bir fikri kaydırılabilir ardışık kartlara bölmek için ideal."
        }
        "sceneTemplate.item.beforeAfter.title" => "Yeniden tasarım öncesi ve sonrası",
        "sceneTemplate.item.beforeAfter.summary" => {
            "Değişiklik notlarıyla yan yana önce/sonra karşılaştırması; geriye dönük değerlendirmeler ve portföy sunumları için ideal."
        }
        "sceneTemplate.item.slideDeck.title" => "Sunum · Altı slayt",
        "sceneTemplate.item.slideDeck.summary" => {
            "Kapak, gündem, ana noktalar, veriler, grafik ve kapanış; 16:9 sunum oranında, metni değiştirip sunmaya hazır."
        }
        "fileMenu.newFromTemplate" => "Şablondan yeni oluştur",
        "collab.ownerConfirm.title" => "Kime katıldığınızı onaylayın",
        "collab.ownerConfirm.hint" => "Bu oturumdan henüz hiçbir şey yüklenmedi.",
        "collab.ownerConfirm.account" => "Doğrulanmış hesap",
        "collab.ownerConfirm.device" => "Doğrulanmış cihaz",
        "collab.ownerConfirm.claimedName" => "Bu hesabın seçtiği ad (doğrulanmadı)",
        "collab.action.confirmOwner" => "Bu oturuma katıl",
        "collab.action.rejectOwner" => "Katılma",
        "collab.error.ownerNotConfirmed" => "Sunucuyu onaylamadınız, bu yüzden hiçbir şey yüklenmedi.",
        _ => return super::tr_collab::lookup(key),
    })
}
