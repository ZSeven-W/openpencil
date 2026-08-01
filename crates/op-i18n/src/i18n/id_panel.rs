//! Overflow-shard strings for this locale.
//!
//! The main table sits at the repo's 800-line file cap, so `id_git`
//! falls through here for the `imagePanel.*` popover keys and the
//! `providerProbe.*` keys the Antigravity / Grok Build CLI probes emit.

pub fn lookup(key: &str) -> Option<&'static str> {
    Some(match key {
        "imagePanel.searchPlaceholder" => "Cari gambar…",
        "imagePanel.searching" => "Mencari…",
        "imagePanel.noResults" => "Tidak ada hasil",
        "imagePanel.searchPrompt" => "Cari gambar",
        "imagePanel.sourceNotice" => {
            "Gambar dari {{source}}. Berlisensi bebas — periksa lisensi sebelum digunakan."
        }
        "imagePanel.genNotConfigured" => "Pembuatan gambar belum dikonfigurasi",
        "imagePanel.openSettings" => "Buka Pengaturan",
        "imagePanel.promptPlaceholder" => "Deskripsikan gambar…",
        "providerProbe.connectedViaCli" => "Terhubung melalui CLI {{name}}",
        "providerProbe.cliExitedWithError" => "CLI {{name}} keluar dengan galat",
        "providerProbe.cliNoVersionOutput" => "CLI {{name}} tidak menghasilkan informasi versi",
        "providerProbe.modelQueryFailed" => "Kueri model {{name}} gagal atau kehabisan waktu",
        "providerProbe.modelQueryFailedRunLogin" => {
            "Kueri model {{name}} gagal. Jalankan {{command}} sekali untuk mengautentikasi."
        }
        "providerProbe.modelQueryNeedsAuth" => {
            "Kueri model {{name}} memerlukan autentikasi. Jalankan {{command}} sekali untuk masuk."
        }
        "providerProbe.unrecognizedModelCatalog" => {
            "{{name}} mengembalikan katalog model yang tidak dikenali"
        }
        "promptCenter.title" => "Pusat Prompt",
        "promptCenter.searchPlaceholder" => "Cari prompt…",
        "promptCenter.category.all" => "Semua",
        "promptCenter.category.starter" => "Mulai cepat",
        "promptCenter.category.mobileApp" => "Aplikasi seluler",
        "promptCenter.category.webPage" => "Halaman web",
        "promptCenter.category.dashboard" => "Dasbor",
        "promptCenter.category.component" => "Komponen",
        "promptCenter.category.modify" => "Ubah desain",
        "promptCenter.category.custom" => "Milik saya",
        "promptCenter.empty" => "Tidak ada prompt yang cocok",
        "promptCenter.saveCurrent" => "Simpan masukan saat ini sebagai prompt",
        "promptCenter.saveTitlePlaceholder" => "Masukkan judul prompt",
        "promptCenter.save" => "Simpan",
        "promptCenter.cancel" => "Batal",
        "promptCenter.delete" => "Hapus",
        "promptCenter.screens" => "{{count}} layar",
        "promptCenter.freeform" => "Bebas",
        "promptCenter.item.wander.title" => "Wander · Perencanaan perjalanan",
        "promptCenter.item.forage.title" => "Forage · Resep musiman",
        "promptCenter.item.still.title" => "Still · Meditasi dan tidur",
        "promptCenter.item.hearth.title" => "Hearth · Rumah pintar",
        "promptCenter.item.meteo.title" => "Meteo · Cuaca imersif",
        "promptCenter.item.marginalia.title" => "Marginalia · Membaca dan anotasi",
        "promptCenter.item.lingua.title" => "Lingua · Belajar bahasa",
        "promptCenter.item.daybreak.title" => "Daybreak · Pesan kopi",
        "promptCenter.item.verdant.title" => "Verdant · Perawatan tanaman",
        "promptCenter.item.companion.title" => "Companion · Kehidupan hewan peliharaan",
        "promptCenter.item.relic.title" => "Relic · Pasar barang bekas pilihan",
        "promptCenter.item.nocturne.title" => "Nocturne · Panduan melihat bintang",
        "promptCenter.item.marquee.title" => "Marquee · Daftar tontonan film",
        "promptCenter.item.ritual.title" => "Ritual · Membangun kebiasaan",
        "promptCenter.item.ember.title" => "Ember · Jurnal suasana hati",
        "promptCenter.item.volt.title" => "Volt · Pendamping kendaraan listrik",
        "promptCenter.item.aloft.title" => "Aloft · Pelacak penerbangan",
        "promptCenter.item.gallery.title" => "Gallery · Pameran dan budaya",
        "promptCenter.item.nightcap.title" => "Nightcap · Meracik minuman di rumah",
        "promptCenter.item.bloom.title" => "Bloom · Jurnal tumbuh kembang anak",
        "promptCenter.item.extremeWeather.title" => "Aplikasi cuaca · Buat saya terpukau",
        "promptCenter.item.extremeNowPlaying.title" => "Sedang diputar · Indah dan siap rilis",
        "promptCenter.item.extremeDailyApp.title" => "Aplikasi yang ingin dibuka setiap hari",
        "promptCenter.item.extremeCalendar.title" => "Ciptakan ulang aplikasi kalender",
        "promptCenter.item.extremeCalm.title" => "Ketenangan dalam satu layar",
        "promptCenter.item.webOrbit.title" => "Orbit · Halaman landing ruang kerja AI",
        "promptCenter.item.webAtelier.title" => "Atelier · E-commerce furnitur",
        "promptCenter.item.dashboardPulse.title" => "Pulse · Dasbor analitik pertumbuhan",
        "promptCenter.item.dashboardSentinel.title" => "Sentinel · Operasi logistik",
        "promptCenter.item.componentDataGrid.title" => "Gridworks · Tabel data perusahaan",
        "promptCenter.item.componentFormLab.title" => "Form Lab · Sistem komponen formulir",
        "promptCenter.item.modifyPolishCurrent.title" => "Sempurnakan layar saat ini",
        "promptCenter.item.modifyCompleteStates.title" => "Lengkapi status komponen",
        "sceneTemplate.title" => "Templat Adegan",
        "sceneTemplate.searchPlaceholder" => "Cari adegan atau templat…",
        "sceneTemplate.empty" => "Tidak ada templat yang cocok",
        "sceneTemplate.frames" => "{{count}} halaman",
        "sceneTemplate.filter.all" => "Semua",
        "sceneTemplate.scene.tutorial" => "Gambar tutorial",
        "sceneTemplate.scene.comparison" => "Gambar perbandingan",
        "sceneTemplate.scene.carousel" => "Kartu pengetahuan",
        "sceneTemplate.scene.slides" => "PPT",
        "sceneTemplate.item.screenshotTutorial.title" => {
            "Kartu tutorial tangkapan layar 3 langkah"
        }
        "sceneTemplate.item.screenshotTutorial.summary" => {
            "Berisi sampul, tiga langkah panduan, dan ajakan bertindak di bagian akhir. Ganti tangkapan layar serta penjelasannya, lalu siap diterbitkan."
        }
        "sceneTemplate.item.knowledgeCarousel.title" => "Karusel pengetahuan dan wawasan",
        "sceneTemplate.item.knowledgeCarousel.summary" => {
            "Berisi sampul, tiga poin utama, dan halaman rangkuman, cocok untuk memecah satu gagasan menjadi rangkaian kartu yang dapat digeser."
        }
        "sceneTemplate.item.beforeAfter.title" => "Perbandingan sebelum dan sesudah desain ulang",
        "sceneTemplate.item.beforeAfter.summary" => {
            "Tampilan sebelum dan sesudah diletakkan berdampingan, dilengkapi catatan perubahan; cocok untuk retrospektif dan portofolio."
        }
        "sceneTemplate.item.slideDeck.title" => "Presentasi · 6 slide",
        "sceneTemplate.item.slideDeck.summary" => {
            "Berisi sampul, agenda, poin utama, data, grafik, dan penutup dalam rasio presentasi 16:9. Cukup ganti teksnya, lalu siap dipresentasikan."
        }
        "fileMenu.newFromTemplate" => "Buat dari templat",
        "collab.ownerConfirm.title" => "Konfirmasi siapa yang Anda ikuti",
        "collab.ownerConfirm.hint" => "Belum ada apa pun dari sesi ini yang dimuat.",
        "collab.ownerConfirm.account" => "Akun terverifikasi",
        "collab.ownerConfirm.device" => "Perangkat terverifikasi",
        "collab.ownerConfirm.claimedName" => "Nama pilihan akun ini (belum terverifikasi)",
        "collab.action.confirmOwner" => "Gabung sesi ini",
        "collab.action.rejectOwner" => "Jangan gabung",
        "collab.error.ownerNotConfirmed" => {
            "Anda tidak mengonfirmasi host, jadi tidak ada yang dimuat."
        }
        _ => return super::id_collab::lookup(key),
    })
}
