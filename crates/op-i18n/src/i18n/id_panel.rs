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
        "promptCenter.item.webKilnform.title" => "Kilnform · Situs infrastruktur desain",
        "promptCenter.item.webReefwright.title" => "Reefwright · Situs pengetahuan dukungan AI",
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
        "sceneTemplate.generate.placeholder" => "Jelaskan topik, AI membuat seluruh presentasi",
        "sceneTemplate.generate.button" => "Buat",
        "sceneTemplate.generate.hint" => "Dokumen baru, dibuat dari topik Anda sebagai presentasi lengkap.",
        "sceneTemplate.generate.promptTemplate" => "Buatkan presentasi (PPT) untuk topik berikut: {{topic}}",
        "sceneTemplate.card.addToCanvas" => "Tambahkan ke kanvas",
        "sceneTemplate.card.generateFrom" => "Buat dari ini",
        "sceneTemplate.generate.basis" => "Berdasarkan: ",
        "sceneTemplate.filter.all" => "Semua",
        "sceneTemplate.scene.tutorial" => "Tutorial",
        "sceneTemplate.scene.comparison" => "Perbandingan",
        "sceneTemplate.scene.carousel" => "Karusel",
        "sceneTemplate.scene.slides" => "Slide",
        "sceneTemplate.scene.card" => "Kartu",
        "sceneTemplate.scene.web" => "Halaman web",
        "sceneTemplate.generate.webPromptTemplate" => "Rancang sebuah halaman arahan web dengan beberapa bagian untuk topik berikut: {{topic}}",
        "sceneTemplate.item.saasLandingOrange.title" => "Halaman Arahan SaaS · Oranye",
        "sceneTemplate.item.saasLandingOrange.summary" => "Halaman pemasaran terang yang dibangun di atas panel nyaris hitam dan satu oranye: navigasi, hero dengan tangkapan layar produk, tiga kartu kemampuan, peragaan alur kerja, testimoni, dan footer berlangganan. Ganti teksnya dan ia sudah jadi situs.",
        "sceneTemplate.item.productLandingLight.title" => "Halaman Arahan Produk · Terang",
        "sceneTemplate.item.productLandingLight.summary" => "Halaman produk seputih kertas bergaya surat kabar: demo hero yang interaktif, kolom kemampuan, papan analitik, perbandingan cara lama dan baru, serta tiga tingkat harga. Untuk situs SaaS dan peluncuran produk.",
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
        "sceneTemplate.item.knowledgeCardVertical.title" => "Kartu pengetahuan · Potret",
        "sceneTemplate.item.knowledgeCardVertical.summary" => "Satu kartu 3:4 dengan judul, empat poin utama, dan baris nama. Ganti teksnya lalu terbitkan.",
        "sceneTemplate.item.knowledgeCardSquare.title" => "Kartu pengetahuan · Persegi",
        "sceneTemplate.item.knowledgeCardSquare.summary" => "Kartu 1:1 dengan tata letak yang sama, cukup ringkas untuk gambar sampul atau unggahan sosial.",
        "sceneTemplate.item.pitchDeckDark.title" => "Pitch deck · Gelap",
        "sceneTemplate.item.pitchDeckDark.summary" => "Sampul, masalah, solusi, angka, peta jalan, dan halaman kontak. Huruf besar di latar gelap, untuk penggalangan dana dan peluncuran.",
        "sceneTemplate.item.lectureDeckLight.title" => "Slide kuliah · Terang",
        "sceneTemplate.item.lectureDeckLight.summary" => "Sampul kursus, tujuan belajar, penjelasan konsep, contoh soal, tabel perbandingan, dan rangkuman. Latar putih kertas, nyaman sepanjang kelas.",
        "sceneTemplate.item.minimalKeynote.title" => "Keynote minimalis",
        "sceneTemplate.item.minimalKeynote.summary" => "Ruang kosong luas, tipografi besar, satu kalimat rata tengah per halaman — sembilan halaman tanpa satu pun kartu, daftar isi hanya garis tipis dan angka. Untuk peluncuran dan keynote.",
        "sceneTemplate.item.gradientTech.title" => "Tech gradien",
        "sceneTemplate.item.gradientTech.summary" => "Latar gradien gelap dengan kartu kaca buram: arsitektur, perbandingan performa, dan dinding klien. Untuk peluncuran produk developer.",
        "sceneTemplate.scene.infographic" => "Infografik",
        "sceneTemplate.item.punchQuoteCard.title" => "Kartu Kutipan · Poster",
        "sceneTemplate.item.punchQuoteCard.summary" => "Kartu 3:4 berlatar nyaris hitam: dua baris raksasa di atas satu pita kuning. Satu kalimat saja, untuk opini dan kutipan.",
        "sceneTemplate.item.journalChecklistCard.title" => {
            "Kartu Checklist · Gaya Basis Pengetahuan"
        }
        "sceneTemplate.item.journalChecklistCard.summary" => "Satu kartu putih di atas latar abu muda: lima tugas untuk dicentang, satu label, dan satu blok kutipan. Untuk rencana mingguan.",
        "sceneTemplate.item.dataReportInfographic.title" => "Infografik Data",
        "sceneTemplate.item.dataReportInfographic.summary" => "Gambar panjang untuk digulir: kepala gelap, tiga angka besar, perbandingan batang, komposisi, dan tiga kesimpulan. Ganti angkanya lalu unggah.",
        "sceneTemplate.item.stepsFlowInfographic.title" => "Infografik Langkah demi Langkah",
        "sceneTemplate.item.stepsFlowInfographic.summary" => "Gambar panjang untuk digulir: lima kartu langkah bernomor dirangkai jadi satu alur, masing-masing dengan perkiraan waktu, plus dua tips. Untuk tutorial.",
        "sceneTemplate.item.eventPosterDeck.title" => "Deck Acara · Poster",
        "sceneTemplate.item.eventPosterDeck.summary" => "Sampul, sorotan, jadwal, rute, tiket, dan penutup. Latar putih galeri dengan blok merah dan biru, tanpa sudut membulat dan tanpa gradasi — untuk pasar kreatif, acara komunitas, dan pembukaan.",
        "sceneTemplate.item.pitfallListInfographic.title" => "Infografik Daftar Kesalahan Umum",
        "sceneTemplate.item.pitfallListInfographic.summary" => "Gambar panjang untuk digulir: enam kesalahan diurutkan menurut seberapa sering terjadi, masing-masing dengan letak salahnya dan gantinya, plus empat baris pemeriksaan sebelum unggah. Hanya hitam, putih, dan abu-abu.",
        "sceneTemplate.item.spineCultureCard.title" => "Kartu Judul Vertikal · Pigmen Mineral",
        "sceneTemplate.item.spineCultureCard.summary" => "Kartu 3:4 berlatar tanah oker gelap: judul Tionghoa tersusun tegak, plester mengelupas, dan butiran pigmen. Untuk budaya, tulisan panjang, dan sampul personal.",
        "sceneTemplate.item.metricSingleCard.title" => "Kartu Angka Tunggal · Grid Hanzi",
        "sceneTemplate.item.metricSingleCard.summary" => "Kartu 1:1: satu angka raksasa di atas putih murni, grid Swiss yang ketat, dan sebuah kotak merah sinyal. Untuk kesimpulan dan capaian.",
        "sceneTemplate.item.quoteFrameCard.title" => "Kartu Kutipan · Sutra Biru-Hijau",
        "sceneTemplate.item.quoteFrameCard.summary" => "Kartu 4:5 berlatar sutra menguning: satu kalimat berbingkai, dengan pegunungan azurit dan malasit di kakinya. Untuk kutipan dan wawancara.",
        "sceneTemplate.item.dailySignCard.title" => "Kartu Harian · Jendela Taman",
        "sceneTemplate.item.dailySignCard.summary" => "Kartu 3:4 berlatar dinding kapur dengan jendela kisi segi enam; di dalamnya tanggal dan satu baris. Ruang kosong adalah hiasannya.",
        "sceneTemplate.item.priceTierCard.title" => "Kartu Daftar Harga · Neon Arkade",
        "sceneTemplate.item.priceTierCard.summary" => "Kartu 1:1 berlatar malam biru tinta: daftar harga tiga tingkat, garis tabung neon, dan pendar cahayanya. Untuk toko, acara, dan paket.",
        "sceneTemplate.item.noticeBoardCard.title" => "Kartu Pengumuman · Huruf Timah",
        "sceneTemplate.item.noticeBoardCard.summary" => "Kartu 4:5 berlatar kertas koran: garis kepala dengan pelat merah bergeser, pasal bernomor, dan cap seri. Untuk pengumuman dan tata tertib.",
        "sceneTemplate.item.milestoneTimelineInfographic.title" => "Infografik Garis Waktu",
        "sceneTemplate.item.milestoneTimelineInfographic.summary" => "Gambar panjang untuk digulir: satu sumbu sepanjang tinggi gambar, penanda tahun di samping kartu tonggak, ditutup dengan langkah berikutnya. Untuk retrospektif, sejarah merek, dan perjalanan proyek.",
        "sceneTemplate.item.conceptContrastInfographic.title" => "Infografik Kontras Konsep",
        "sceneTemplate.item.conceptContrastInfographic.summary" => "Gambar panjang untuk digulir: kesimpulan lebih dulu, lalu kartu definisi tiap konsep, uraian dua kolom per aspek, dan terakhir dasar memilih.",
        "sceneTemplate.item.rankingBoardInfographic.title" => "Infografik Peringkat Top N",
        "sceneTemplate.item.rankingBoardInfographic.summary" => "Gambar panjang untuk digulir: papan rekomendasi emas di atas tinta — lencana besar untuk tiga teratas, lencana garis untuk empat sampai delapan, lengkap dengan kapan dan seberapa sering.",
        "sceneTemplate.item.faqThreadInfographic.title" => "Infografik Tanya Jawab",
        "sceneTemplate.item.faqThreadInfographic.summary" => "Gambar panjang untuk digulir: enam pasang tanya jawab, T pejal dan J bergaris. Tanpa nomor dan tanpa urutan — satu pasang pun berdiri sendiri.",
        "sceneTemplate.item.dataStoryInfographic.title" => "Infografik Cerita Data",
        "sceneTemplate.item.dataStoryInfographic.summary" => "Gambar panjang untuk digulir: empat angka dirangkai jadi satu garis sebab-akibat, tiap tahap ditampilkan sebagai kisi sepuluh kotak, ditutup kesimpulan yang bisa langsung dipakai.",
        "sceneTemplate.item.challengeTrackerInfographic.title" => "Infografik Tantangan 30 Hari",
        "sceneTemplate.item.challengeTrackerInfographic.summary" => "Gambar panjang untuk digulir: kisi tiga puluh kotak, enam kali lima, dengan tonggak hanya di hari 7, 15, dan 30. Simpan lalu coret satu kotak sehari.",
        "sceneTemplate.item.ecosystemMapInfographic.title" => "Infografik Peta Ekosistem",
        "sceneTemplate.item.ecosystemMapInfographic.summary" => "Gambar panjang untuk digulir: empat posisi pada satu rantai disusun dua kali dua, tiga pemain di tiap kotak, dan celahnya ditandai. Kartu putih di atas latar batu tulis.",
        "sceneTemplate.item.doDontComparison.title" => "Dua Kolom Benar / Salah",
        "sceneTemplate.item.doDontComparison.summary" => "Kartu 3:4: dua cara mengerjakan hal yang sama berdampingan, dibedakan lewat tekstur dan ikon alih-alih merah versus hijau, sehingga pembaca buta warna pun bisa membacanya.",
        "sceneTemplate.item.mythTruthComparison.title" => "Mitos dan Fakta",
        "sceneTemplate.item.mythTruthComparison.summary" => "Gambar tinggi: lima pasang “kata orang / kenyataannya”, mitos sempit dan pucat di kiri, fakta lebar dan gelap di kanan.",
        "sceneTemplate.item.pricingTiersComparison.title" => "Perbandingan Tingkat Harga",
        "sceneTemplate.item.pricingTiersComparison.summary" => "Kartu 3:4: Gratis, Pro, dan Tim berdampingan, harga sebagai jangkar, tiap kolom memuat kolom sebelumnya. Untuk halaman harga.",
        "sceneTemplate.item.scenarioGuideComparison.title" => "Panduan Pilihan per Situasi",
        "sceneTemplate.item.scenarioGuideComparison.summary" => "Gambar tinggi: tanpa spesifikasi, hanya tujuh situasi, masing-masing dengan label putusan. Pembaca cukup menemukan barisnya.",
        "sceneTemplate.item.specTableComparison.title" => "Tabel Perbandingan Spesifikasi",
        "sceneTemplate.item.specTableComparison.summary" => "Gambar tinggi: dua kandidat dalam satu tabel sungguhan, baris demi baris, sel pemenang diangkat dengan latar gelap.",
        "sceneTemplate.item.threeWayComparison.title" => "Perbandingan Tiga Opsi",
        "sceneTemplate.item.threeWayComparison.summary" => "Gambar tinggi: tiga opsi berdampingan dengan rekomendasi di tengah; tiap kolom dibuka oleh sebuah situasi, bukan sebuah nama.",
        "sceneTemplate.item.timeShiftComparison.title" => "Setahun Lalu dan Sekarang",
        "sceneTemplate.item.timeShiftComparison.summary" => "Kartu 3:4: tulang punggung label di tengah, setahun lalu di kiri dan sekarang di kanan, dua nilai satu butir jatuh pada baris yang sama.",
        "sceneTemplate.item.tradeoffScaleComparison.title" => "Timbangan Untung Rugi",
        "sceneTemplate.item.tradeoffScaleComparison.summary" => "Kartu 1:1: satu palang dua piringan — kiri yang didapat, kanan yang dibayar, tiap baris didahului kotak centang kosong.",
        "sceneTemplate.item.versionDiffComparison.title" => "Perubahan Antarversi",
        "sceneTemplate.item.versionDiffComparison.summary" => "Kartu 1:1: tanpa dua kolom — tiap baris menuntaskan sendiri “lama → baru”, cukup digulir.",
        "sceneTemplate.item.appOnboardingTriptych.title" => "Triptik Onboarding Aplikasi",
        "sceneTemplate.item.appOnboardingTriptych.summary" => "Kartu 3:4: tiga ponsel berjajar dengan slot gambar kosong. Masukkan tiga layar onboarding Anda, tambahkan teks, siap untuk ditinjau atau diunggah.",
        "sceneTemplate.item.diyBlueprintGuide.title" => "Panduan DIY Bergambar",
        "sceneTemplate.item.diyBlueprintGuide.summary" => "Gambar tinggi dengan tabel bahan dan ukuran seluas bagian langkah — DIY gagal di persiapan, bukan di tangan.",
        "sceneTemplate.item.photoCompositionTutorial.title" => "Komposisi Foto dengan Ponsel",
        "sceneTemplate.item.photoCompositionTutorial.summary" => "3:4, lima bingkai: masing-masing sebuah jendela bidik gelap dengan garis panduan menyala di atas slot foto.",
        "sceneTemplate.item.recipeFourStep.title" => "Kartu Resep Empat Langkah",
        "sceneTemplate.item.recipeFourStep.summary" => "Kartu 4:5 pola 2×2: keempat langkah dalam satu kartu. Tangkap layar lalu masak — di depan kompor membalik halaman itu merepotkan.",
        "sceneTemplate.item.skincareRoutineCards.title" => "Kartu Rutinitas Perawatan Kulit",
        "sceneTemplate.item.skincareRoutineCards.summary" => "4:5, enam bingkai: tiap langkah menetapkan tiga angka — takaran, waktu tunggu, pagi atau malam.",
        "sceneTemplate.item.softwareStepTutorial.title" => {
            "Kartu Langkah Penggunaan Perangkat Lunak"
        }
        "sceneTemplate.item.softwareStepTutorial.summary" => "Kartu 4:5, satu-satunya bernuansa gelap di seri tutorial: slot tangkapan layar dengan instruksi bernomor.",
        "sceneTemplate.item.storageMakeoverSteps.title" => "Langkah Perombakan Penyimpanan",
        "sceneTemplate.item.storageMakeoverSteps.summary" => "3:4, enam bingkai: selain tindakan dan slot gambar, tiap langkah menetapkan syarat selesai dan anggaran waktu.",
        "sceneTemplate.item.weeklyReportLesson.title" => "Pelajaran Laporan Mingguan",
        "sceneTemplate.item.weeklyReportLesson.summary" => "Gambar tinggi: setelah menjelaskan struktur empat bagian, langsung memberi kerangka berisi garis isian.",
        "sceneTemplate.item.workoutBreakdownGuide.title" => "Panduan Uraian Gerakan Latihan",
        "sceneTemplate.item.workoutBreakdownGuide.summary" => "Gambar tinggi: tiap gerakan membawa bilah tetap berisi set, repetisi, dan waktu istirahat.",
        "sceneTemplate.item.bookreviewSilkCarousel.title" => "Karusel Bedah Ulasan Buku / Film",
        "sceneTemplate.item.bookreviewSilkCarousel.summary" => "3:4, lima papan: kail, kutipan beranotasi, tiga wawasan, satu kalimat layak kutip, penutup. Membedah karya menjadi bagian yang bisa dibawa pulang, bukan menceritakan ulang alur.",
        "sceneTemplate.item.cityguideFilmCarousel.title" => "Karusel Panduan Kota",
        "sceneTemplate.item.cityguideFilmCarousel.summary" => "3:4, tujuh papan: tempat dan rute berselang — papan tempat untuk yang bermimpi, rute sehari dan tabel makan-menginap untuk yang merencanakan.",
        "sceneTemplate.item.datareportGridCarousel.title" => "Karusel Laporan Data",
        "sceneTemplate.item.datareportGridCarousel.summary" => "3:4, enam papan: setiap papan data disusul papan non-data, supaya tak ada yang berhenti di grafik ketiga.",
        "sceneTemplate.item.opinionLongformCarousel.title" => "Karusel Opini Panjang",
        "sceneTemplate.item.opinionLongformCarousel.summary" => "3:4, enam papan: satu master visual ketat sepanjang set, nomor halaman dan judul selalu di tempat yang sama.",
        "sceneTemplate.item.qaChalkboardCarousel.title" => "Karusel Tanya Jawab",
        "sceneTemplate.item.qaChalkboardCarousel.summary" => "3:4, enam papan: satu pertanyaan per papan, dengan nomor tanda tanya tulisan tangan di sudut.",
        "sceneTemplate.item.storyNightCarousel.title" => "Karusel Cerita",
        "sceneTemplate.item.storyNightCarousel.summary" => "3:4, tujuh papan: refleksi pengalaman pribadi di atas kerangka waktu — garis waktu di papan kelima adalah dinding penopangnya.",
        "sceneTemplate.item.toolkitNotebookCarousel.title" => "Karusel Kumpulan Perkakas",
        "sceneTemplate.item.toolkitNotebookCarousel.summary" => "3:4, enam papan: enam perkakas satu per papan, papan terakhir mendaftar semuanya lengkap dengan nomor halaman.",
        "sceneTemplate.item.tutorialJournalCarousel.title" => "Karusel Tutorial",
        "sceneTemplate.item.tutorialJournalCarousel.summary" => {
            "3:4, enam papan: satu langkah per papan, jari adalah bilah kemajuannya."
        }
        "sceneTemplate.item.yearreviewMineralCarousel.title" => "Karusel Rekap Tahunan",
        "sceneTemplate.item.yearreviewMineralCarousel.summary" => "3:4, delapan papan: papan angka bernuansa dingin, papan renungan bernuansa hangat, berselang-seling.",
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
        "fileMenu.exportSlideshowHtml" => "Ekspor tayangan slide HTML...",
        "fileMenu.exportPptx" => "Ekspor PowerPoint...",
        "dialog.slideshowHtmlTitle" => "Ekspor tayangan slide",
        "dialog.slideshowHtmlSummary" => "{{count}} slide diekspor ke:",
        "dialog.slideshowHtmlEmpty" => "Presentasi ini tidak memiliki slide yang terlihat untuk diekspor.",
        // HTML import diagnostics — one entry per `ImportWarning::code`.
        "htmlImport.warn.content.empty_input" => "Konten HTML yang dapat diimpor tidak tersedia.",
        "htmlImport.warn.content.empty_body" => {
            "Konten yang dapat diimpor di dalam body HTML tidak tersedia."
        }
        "htmlImport.warn.content.dom_depth_truncated" => {
            "HTML yang bersarang lebih dalam dari {{max_depth}} tingkat dibuang."
        }
        "htmlImport.warn.content.node_limit_truncated" => {
            "Batas node tercapai; sisa konten halaman dilewati."
        }
        "htmlImport.warn.content.node_limit_mapping" => {
            "Batas node tercapai; sebagian pohon HTML dilewati."
        }
        "htmlImport.warn.content.node_limit_inline_row" => {
            "Batas node tercapai; satu baris pemformatan inline dilewati."
        }
        "htmlImport.warn.content.node_limit_pseudo" => {
            "Batas node tercapai; pseudo-elemen yang dibangkitkan dilewati."
        }
        "htmlImport.warn.css.at_rule_depth_limit" => {
            "Aturan CSS yang bersarang lebih dalam dari {{max_depth}} at-rule diabaikan."
        }
        "htmlImport.warn.css.unterminated_rule" => "Aturan CSS yang tidak ditutup diabaikan.",
        "htmlImport.warn.css.marker_rules_unsupported" => "Aturan CSS ::marker tidak diimpor.",
        "htmlImport.warn.css.nesting_unsupported" => "Aturan gaya CSS bersarang diabaikan.",
        "htmlImport.warn.css.invalid_layer_name" => {
            "Nama @layer '{{name}}' yang tidak valid diabaikan."
        }
        "htmlImport.warn.css.unsupported_statement" => {
            "Pernyataan @{{name}} yang tidak didukung diabaikan."
        }
        "htmlImport.warn.css.media_without_viewport" => "Aturan @media tanpa viewport diabaikan.",
        "htmlImport.warn.css.invalid_layer_block_name" => {
            "Nama blok @layer '{{name}}' yang tidak valid diabaikan."
        }
        "htmlImport.warn.css.unsupported_container_block" => "Blok @container diabaikan.",
        "htmlImport.warn.css.unsupported_block" => "Blok @{{name}} yang tidak didukung diabaikan.",
        "htmlImport.warn.font.web_font_not_downloaded" => {
            "Font web @font-face '{{family}}' tidak tersedia."
        }
        "htmlImport.warn.layout.percentage_absolute_offset_inferred" => {
            "Offset persen pada elemen berposisi absolut diaproksimasi."
        }
        "htmlImport.warn.layout.percentage_relative_offset_inferred" => {
            "Offset persen position:relative diaproksimasi."
        }
        "htmlImport.warn.layout.aspect_ratio_no_definite_axis" => {
            "CSS aspect-ratio tanpa sumbu yang pasti diabaikan."
        }
        "htmlImport.warn.layout.aspect_ratio_indefinite_container" => {
            "CSS aspect-ratio di dalam blok penampung yang tidak pasti diabaikan."
        }
        "htmlImport.warn.layout.position_sticky_ignored" => "CSS position:sticky diabaikan.",
        "htmlImport.warn.layout.grid_tracks_approximated" => {
            "Jalur grid CSS yang tidak didukung diaproksimasi."
        }
        "htmlImport.warn.layout.float_ignored" => "CSS float diabaikan.",
        "htmlImport.warn.layout.mix_blend_mode_no_node_equivalent" => {
            "CSS mix-blend-mode pada tingkat node diaproksimasi."
        }
        "htmlImport.warn.layout.overflow_scroll_clipped" => {
            "CSS overflow: auto / scroll diaproksimasi."
        }
        "htmlImport.warn.layout.negative_margins_ignored" => "Margin CSS negatif diabaikan.",
        "htmlImport.warn.layout.margins_on_visual_box_ignored" => {
            "Margin CSS pada kotak visual diabaikan."
        }
        "htmlImport.warn.layout.inline_margin_wrapping_approximated" => "Elemen inline dengan margin CSS dijadikan kotak dan mungkin tidak lagi membungkus antarbaris.",
        "htmlImport.warn.layout.content_box_percentage_approximated" => {
            "Penentuan ukuran persen content-box diaproksimasi."
        }
        "htmlImport.warn.layout.grid_empty_cells_packed" => {
            "Sel grid CSS yang kosong akibat garis awal eksplisit diaproksimasi."
        }
        "htmlImport.warn.layout.grid_span_reflowed" => {
            "Item grid CSS yang rentangnya tidak muat pada garis awalnya diaproksimasi."
        }
        "htmlImport.warn.layout.grid_rows_node_limit" => {
            "Batas node tercapai; pembungkus baris grid CSS dilewati."
        }
        "htmlImport.warn.layout.grid_track_widths_unresolved" => {
            "Lebar jalur grid CSS yang memakai auto-fit / auto-fill diaproksimasi."
        }
        "htmlImport.warn.layout.grid_template_areas_ignored" => {
            "Penempatan CSS grid-template-areas tidak diimpor."
        }
        "htmlImport.warn.layout.grid_row_placement_ignored" => {
            "Penempatan CSS grid-row tidak diimpor."
        }
        "htmlImport.warn.layout.grid_column_unsupported" => {
            "CSS grid-column `{{value}}` diaproksimasi."
        }
        "htmlImport.warn.layout.block_auto_margins_ignored" => {
            "Margin auto CSS pada sumbu blok tidak diimpor."
        }
        "htmlImport.warn.layout.auto_margin_node_limit" => {
            "Batas node tercapai; perataan margin auto CSS dilewati."
        }
        "htmlImport.warn.layout.flow_offset_no_definite_size" => {
            "Offset CSS dalam alur pada elemen tanpa ukuran yang pasti dibuang."
        }
        "htmlImport.warn.layout.flow_offset_node_limit" => {
            "Batas node tercapai; satu offset CSS dalam alur dilewati."
        }
        "htmlImport.warn.layout.flow_offset_approximated" => {
            "Offset CSS dalam alur (inset position:relative, translasi transform) diaproksimasi."
        }
        "htmlImport.warn.layout.flow_offset_no_wrapper" => {
            "Offset CSS dalam alur pada kotak yang tidak dapat memuat pembungkus offset dibuang."
        }
        "htmlImport.warn.layout.flex_wrap_column_not_emulated" => {
            "flex-wrap pada kontainer flex arah kolom tidak diimpor."
        }
        "htmlImport.warn.layout.flex_wrap_reverse_plain" => "flex-wrap:wrap-reverse diaproksimasi.",
        "htmlImport.warn.layout.flex_wrap_indefinite_width" => {
            "flex-wrap pada kontainer tanpa lebar yang pasti diabaikan."
        }
        "htmlImport.warn.layout.flex_align_content_ignored" => {
            "CSS align-content pada kontainer flex yang membungkus baris tidak diimpor."
        }
        "htmlImport.warn.layout.flex_wrap_indeterminate_children" => {
            "flex-wrap dengan ukuran sumbu utama anak yang tidak tentu diabaikan."
        }
        "htmlImport.warn.layout.flex_wrap_node_limit" => {
            "Batas node tercapai; baris flex-wrap dilewati."
        }
        "htmlImport.warn.transform.unsupported_syntax" => {
            "Sintaks CSS transform yang tidak didukung diabaikan."
        }
        "htmlImport.warn.transform.unsupported_function" => {
            "Fungsi CSS transform yang tidak didukung (3D, matrix3d) diabaikan."
        }
        "htmlImport.warn.transform.percentage_translation_dropped" => {
            "Translasi persen CSS transform pada sumbu yang tidak pasti dibuang."
        }
        "htmlImport.warn.transform.non_finite_matrix" => {
            "CSS transform yang menghasilkan matriks non-hingga diabaikan."
        }
        "htmlImport.warn.transform.skew_dropped" => "Kemiringan CSS transform dibuang.",
        "htmlImport.warn.transform.degenerate_scale" => {
            "CSS transform dengan skala nol atau non-hingga diaproksimasi."
        }
        "htmlImport.warn.transform.mirroring_absolute" => {
            "Pencerminan CSS transform diaproksimasi."
        }
        "htmlImport.warn.transform.origin_z_ignored" => {
            "Offset Z pada CSS transform-origin diabaikan."
        }
        "htmlImport.warn.transform.scale_not_baked" => {
            "Skala CSS transform yang tidak dapat dilekatkan ke ukuran node dibuang."
        }
        "htmlImport.warn.transform.scale_baked" => {
            "Skala CSS transform yang dilekatkan ke ukuran node diaproksimasi."
        }
        "htmlImport.warn.transform.scale_auto_size_ignored" => {
            "Skala CSS transform pada elemen berukuran auto diabaikan."
        }
        "htmlImport.warn.visual.background_repeat_approximated" => {
            "CSS background-repeat berarah atau berjarak diaproksimasi."
        }
        "htmlImport.warn.visual.background_tile_size_ignored" => {
            "Ukuran ubin latar CSS yang eksplisit diabaikan."
        }
        "htmlImport.warn.visual.background_size_auto_box" => {
            "CSS background-size pada elemen berukuran auto diaproksimasi."
        }
        "htmlImport.warn.visual.background_size_needs_intrinsic_size" => {
            "CSS background-size yang membutuhkan ukuran intrinsik gambar diaproksimasi."
        }
        "htmlImport.warn.visual.background_position_unsupported" => {
            "CSS background-position yang tidak didukung diabaikan."
        }
        "htmlImport.warn.visual.background_image_url_empty" => {
            "URL gambar latar CSS yang kosong diabaikan."
        }
        "htmlImport.warn.visual.conic_gradient_ignored" => "Gradien kerucut CSS diabaikan.",
        "htmlImport.warn.visual.background_image_layer_unsupported" => {
            "Lapisan CSS background-image yang tidak didukung diabaikan."
        }
        "htmlImport.warn.visual.background_color_unresolved" => {
            "Warna latar CSS yang tidak terselesaikan diabaikan."
        }
        "htmlImport.warn.visual.background_position_dropped" => {
            "CSS background-position diabaikan."
        }
        "htmlImport.warn.visual.border_colors_approximated" => {
            "Warna garis tepi CSS per sisi diaproksimasi."
        }
        "htmlImport.warn.visual.border_styles_approximated" => {
            "Gaya garis tepi CSS per sisi yang beragam diaproksimasi."
        }
        "htmlImport.warn.visual.border_style_complex" => {
            "Gaya garis tepi CSS yang rumit diaproksimasi."
        }
        "htmlImport.warn.visual.border_style_unsupported" => {
            "Gaya garis tepi CSS yang tidak didukung diaproksimasi."
        }
        "htmlImport.warn.visual.border_radius_elliptical" => {
            "Radius sudut CSS berbentuk elips diaproksimasi."
        }
        "htmlImport.warn.visual.border_radius_unsupported" => {
            "Radius sudut CSS yang tidak didukung diabaikan."
        }
        "htmlImport.warn.visual.box_shadow_layer_unsupported" => {
            "Lapisan CSS box-shadow yang tidak didukung diabaikan."
        }
        "htmlImport.warn.visual.gradient_interpolation_ignored" => {
            "Metode interpolasi warna gradien CSS diabaikan."
        }
        "htmlImport.warn.visual.linear_gradient_direction_unsupported" => {
            "Arah CSS linear-gradient yang tidak didukung diabaikan."
        }
        "htmlImport.warn.visual.gradient_color_hints_ignored" => {
            "Petunjuk warna gradien CSS diabaikan."
        }
        "htmlImport.warn.visual.gradient_color_stop_unsupported" => {
            "Titik warna gradien CSS yang tidak didukung diabaikan."
        }
        "htmlImport.warn.visual.gradient_too_few_stops" => {
            "Gradien CSS dengan titik warna berguna kurang dari dua diabaikan."
        }
        "htmlImport.warn.visual.gradient_repeating_approximated" => {
            "Gradien CSS berulang diaproksimasi."
        }
        "htmlImport.warn.visual.gradient_stops_clamped" => {
            "Titik warna gradien CSS di luar rentang diaproksimasi."
        }
        "htmlImport.warn.visual.blur_radius_unsupported" => {
            "Radius buram CSS yang tidak didukung diabaikan."
        }
        "htmlImport.warn.visual.filter_drop_shadow_unsupported" => {
            "CSS filter drop-shadow() yang tidak didukung diabaikan."
        }
        "htmlImport.warn.visual.filter_function_unsupported" => {
            "Fungsi filter CSS yang tidak didukung diabaikan."
        }
        "htmlImport.warn.visual.backdrop_filter_unsupported" => {
            "Fungsi CSS backdrop-filter yang tidak didukung diabaikan."
        }
        "htmlImport.warn.visual.background_blend_mode_unsupported" => {
            "CSS background-blend-mode yang tidak didukung diabaikan."
        }
        "htmlImport.warn.visual.mix_blend_mode_on_fills" => {
            "CSS mix-blend-mode pada isian satuan diaproksimasi."
        }
        "htmlImport.warn.visual.mix_blend_mode_unsupported" => {
            "CSS mix-blend-mode yang tidak didukung diabaikan."
        }
        "htmlImport.warn.visual.property_not_representable" => "CSS {{property}} diabaikan.",
        "htmlImport.warn.visual.gradient_background_size_ignored" => {
            "CSS background-size pada gradien diabaikan."
        }
        "htmlImport.warn.visual.radial_gradient_position_unsupported" => {
            "Posisi CSS radial-gradient yang tidak didukung diabaikan."
        }
        "htmlImport.warn.visual.radial_gradient_elliptical" => {
            "CSS radial-gradient berbentuk elips diaproksimasi."
        }
        "htmlImport.warn.visual.radial_gradient_extent_approximated" => {
            "Kata kunci jangkauan CSS radial-gradient diaproksimasi."
        }
        "htmlImport.warn.visual.radial_gradient_size_unsupported" => {
            "Ukuran CSS radial-gradient yang tidak didukung diabaikan."
        }
        "htmlImport.warn.text.shadow_layer_unsupported" => {
            "Lapisan CSS text-shadow yang tidak didukung diabaikan."
        }
        "htmlImport.warn.text.shadow_extra_layers_ignored" => {
            "Lapisan CSS text-shadow setelah yang pertama diabaikan."
        }
        "htmlImport.warn.text.shadow_on_inline_ignored" => {
            "CSS text-shadow pada elemen inline diabaikan."
        }
        "htmlImport.warn.list.style_image_ignored" => "CSS list-style-image tidak diimpor.",
        "htmlImport.warn.list.marker_position_outside_approximated" => {
            "Penanda menggantung `list-style-position: outside` diaproksimasi."
        }
        "htmlImport.warn.list.style_type_unsupported" => {
            "CSS list-style-type `{{value}}` yang tidak didukung diaproksimasi."
        }
        "htmlImport.warn.media.object_fit_scale_down" => "CSS object-fit:scale-down diaproksimasi.",
        "htmlImport.warn.media.object_fit_none_ignored" => "CSS object-fit:none diabaikan.",
        "htmlImport.warn.media.object_position_ignored" => "CSS object-position diabaikan.",
        "htmlImport.warn.media.image_intrinsic_axis_unresolved" => {
            "Rasio aspek intrinsik gambar tidak dapat menentukan sumbu yang hilang karena ukuran yang ditetapkan bersifat dinamis atau blok penampungnya tidak memiliki ukuran pasti."
        }
        "htmlImport.warn.media.image_mix_blend_mode_unsupported" => {
            "CSS mix-blend-mode pada gambar yang tidak didukung diabaikan."
        }
        "htmlImport.warn.media.inline_svg_placeholder" => {
            "Elemen <svg> inline diimpor sebagai kotak pengganti."
        }
        "htmlImport.warn.media.input_type_fallback" => {
            "Tipe <input> yang tidak didukung diaproksimasi."
        }
        "htmlImport.warn.media.element_placeholder" => {
            "Elemen <{{tag}}> diimpor sebagai kotak pengganti."
        }
        "htmlImport.warn.media.picture_undecodable_types" => {
            "<picture> yang semua tipe sumbernya tidak dapat didekode diaproksimasi."
        }
        "htmlImport.warn.table.rowspan_ignored" => "Atribut rowspan HTML tidak diimpor.",
        "htmlImport.warn.table.row_groups_unflattened" => {
            "Lebar kolom tabel yang grup barisnya tidak diratakan oleh CSS diaproksimasi."
        }
        "htmlImport.warn.table.indefinite_width_approximated" => {
            "Lebar kolom tabel CSS tanpa lebar yang pasti diaproksimasi."
        }
        "htmlImport.warn.resource.invalid_base_href" => {
            "<base href> {{href}} yang tidak valid diabaikan."
        }
        "htmlImport.warn.resource.base_href_outside_origin" => {
            "<base href> {{href}} di luar asal proyek diabaikan."
        }
        "htmlImport.warn.resource.external_stylesheet_skipped" => {
            "Lembar gaya eksternal {{url}} tidak tersedia."
        }
        "htmlImport.warn.resource.image_outside_origin" => {
            "Gambar {{url}} di luar asal proyek diimpor sebagai gambar pengganti."
        }
        "htmlImport.warn.resource.image_unavailable" => {
            "Gambar {{url}} yang tidak tersedia diimpor sebagai gambar pengganti."
        }
        "htmlImport.warn.resource.css_import_invalid" => {
            "CSS @import {{prelude}} yang tidak valid diabaikan."
        }
        "htmlImport.warn.resource.css_import_unresolvable" => {
            "CSS @import {{reference}} tidak tersedia."
        }
        "htmlImport.warn.resource.css_import_cycle" => {
            "CSS @import {{url}} yang melingkar diabaikan."
        }
        "htmlImport.warn.resource.css_import_depth_limit" => {
            "CSS @import {{url}} melampaui kedalaman {{max_depth}} diabaikan."
        }
        "htmlImport.warn.resource.css_import_unavailable" => "CSS @import {{url}} tidak tersedia.",
        "htmlImport.warn.project.multiple_html_entries" => {
            "{{count}} entri HTML ditemukan; {{entry}} dipilih dan sisanya diaproksimasi."
        }
        "htmlImport.warn.snapshot.truncated" => "Sebagian cuplikan peramban dibuang.",
        "htmlImport.warn.snapshot.node_limit" => "Batas node tercapai; sisa isi cuplikan dilewati.",
        "htmlImport.warn.snapshot.tainted_images" => {
            "{{count}} gambar ternoda CORS, dipertahankan sebagai URL jarak jauh, tidak tersedia."
        }
        "htmlImport.warn.snapshot.invalid_rect" => {
            "Node cuplikan dengan kotak batas yang hilang atau tidak valid dibuang."
        }
        "htmlImport.warn.snapshot.unknown_kind" => {
            "Node cuplikan dengan jenis yang tidak dikenal dibuang."
        }
        "htmlImport.warn.snapshot.rejected" => "Cuplikan peramban ({{reason}}) dibuang.",
        "htmlImport.warn.snapshot.unsupported_transform" => {
            "Transformasi cuplikan yang tidak didukung diabaikan."
        }
        "htmlImport.warn.css.media_empty_query" => "Kueri @media yang kosong diabaikan.",
        "htmlImport.warn.css.media_unsupported_type" => {
            "Tipe @media '{{name}}' yang tidak didukung diabaikan."
        }
        "htmlImport.warn.css.media_unsupported_condition" => {
            "Kondisi @media '{{input}}' yang tidak didukung diabaikan."
        }
        "htmlImport.warn.css.media_invalid_orientation" => {
            "Orientasi @media '{{value}}' yang tidak valid diabaikan."
        }
        "htmlImport.warn.css.media_unsupported_feature" => {
            "Fitur @media '{{name}}' yang tidak didukung diabaikan."
        }
        "htmlImport.warn.css.media_unsupported_range" => {
            "Rentang @media '({{input}})' yang tidak didukung diabaikan."
        }
        "htmlImport.warn.css.media_invalid_range" => {
            "Rentang @media '({{input}})' yang tidak valid diabaikan."
        }
        "htmlImport.warn.css.media_invalid_length" => {
            "Panjang @media '{{value}}' yang tidak valid diabaikan."
        }
        "htmlImport.diagnostics.title" => "Impor HTML selesai",
        "htmlImport.diagnostics.summary" => "Item terdegradasi: {{count}}",
        "htmlImport.diagnostics.dismiss" => "Tutup",
        "htmlImport.diagnostics.expand" => "Tampilkan detail",
        "htmlImport.diagnostics.collapse" => "Sembunyikan detail",
        "htmlImport.diagnostics.more" => "+{{count}} lainnya",
        "dialog.pptxTitle" => "Ekspor PowerPoint",
        "dialog.pptxSummary" => "{{count}} slide diekspor ke:",
        "dialog.pptxEmpty" => "Presentasi ini tidak memiliki slide yang terlihat untuk diekspor.",
        "settings.agents.acpQuickAdd" => "Tambah cepat",
        "settings.agents.acpPresetAdd" => "Tambah",
        "settings.agents.acpNotInstalled" => "Belum terpasang",
        "assetCenter.title" => "Pusat Aset",
        "assetCenter.tab.templates" => "Templat",
        "assetCenter.tab.styles" => "Gaya",
        "assetCenter.style.empty" => "Tidak ada gaya yang cocok",
        "assetCenter.style.pinned" => "Disematkan",
        "assetCenter.style.searchPlaceholder" => "Cari gaya atau tag",
        "assetCenter.style.generateHint" => "Dokumen baru dari topik Anda, dengan gaya yang disematkan.",
        "ai.pinnedStyle" => "Gaya: {{name}}",
        "assetCenter.style.import" => "Impor gaya",
        "assetCenter.style.mine" => "Gaya saya",
        "assetCenter.style.builtIn" => "Gaya bawaan",
        "assetCenter.style.importTitle" => "Impor DESIGN.md",
        "assetCenter.style.importHint" => "Tempel seluruh DESIGN.md, lalu konfirmasi impor.",
        "assetCenter.style.importSource" => "Anda bisa menyalin gaya dari pustaka DESIGN.md seperti styles.refero.design.",
        "assetCenter.style.importConfirm" => "Impor",
        "assetCenter.style.importCancel" => "Batal",
        "assetCenter.style.importPickFile" => "Pilih berkas…",
        "assetCenter.style.importHintFile" => "Pilih berkas DESIGN.md, atau tempel seluruh dokumen di bawah.",
        "assetCenter.style.importPlaceholder" => "Tempel DESIGN.md Anda di sini",
        "assetCenter.style.importEmpty" => "Berkas itu kosong, atau terlalu pendek untuk menjadi panduan gaya.",
        "assetCenter.style.importNotText" => "Berkas itu tidak terbaca sebagai teks Markdown.",
        "assetCenter.style.importTooLarge" => "Berkas itu lebih besar dari 512 KB.",
        "slidesPanel.tabSlides" => "Slide",
        "slidesPanel.tabCards" => "Kartu",
        "slidesPanel.present" => "Presentasikan",
        "slidesPanel.exportPdf" => "Ekspor PDF",
        "slidesPanel.exportAllSlides" => "Ekspor semua slide",
        "slidesPanel.exportSelectedSlides" => "Ekspor slide terpilih ({{count}})",
        "settings.tab.ai" => "AI",
        "settings.agents.heroTitle" => "Hubungkan penyedia AI Anda",
        "settings.agents.heroSubtitle" => "OpenPencil menjalankan agen CLI lokal dan penyedia API Anda — hubungkan salah satunya untuk mulai membuat desain.",
        "settings.agents.statusConnected" => "Terhubung",
        "settings.agents.statusNotConnected" => "Belum terhubung",
        "settings.agents.statusChecking" => "Memeriksa status…",
        "settings.mcp.heroTitle" => "Hubungkan OpenPencil dari luar lewat MCP",
        "settings.mcp.heroSubtitle" => "Arahkan CLI atau editor apa pun yang mendukung MCP ke workspace ini, lalu kendalikan kanvas dengan perkakas yang sama seperti agen bawaan.",
        "settings.mcp.terminalFootnote" => "* Saat mulai, MCP otomatis disiapkan untuk perkakas CLI yang dipilih.",
        "settings.mcp.customConfigTitle" => "Konfigurasi Server MCP Kustom",
        "settings.mcp.customConfigDesc" => "Tempel ini ke klien mana pun yang membaca blok MCP server standar.",
        "settings.mcp.copyConfig" => "Salin konfigurasi MCP",
        "settings.system.heroTitle" => "Preferensi sistem",
        "settings.system.heroSubtitle" => "Tampilan, pembaruan, dan perilaku kanvas untuk pemasangan ini.",
        "settings.system.appearance" => "Tampilan",
        "settings.system.appearanceLight" => "Terang",
        "settings.system.appearanceDark" => "Gelap",
        "settings.system.pencilCursor" => "Kursor pensil",
        "settings.images.heroTitle" => "Gambar untuk desain Anda",
        "settings.images.heroSubtitle" => "Cari foto di Openverse, atau hubungkan penyedia untuk membuatnya sesuai kebutuhan.",
        "settings.fonts.heroTitle" => "Font di dokumen ini",
        "settings.fonts.heroSubtitle" => "Selesaikan font yang diminta dokumen tetapi tidak ada di mesin ini, dan kelola font yang Anda impor.",
        "settings.account.heroTitle" => "Akun Anda",
        "settings.account.heroSubtitle" => "Masuk untuk menyinkronkan workspace dan lisensi Anda di semua perangkat.",
        "tooltip.topbar.file" => "Berkas",
        "tooltip.topbar.import" => "Impor",
        "tooltip.topbar.language" => "Bahasa",
        "tooltip.topbar.collaboration" => "Kolaborasi",
        "tooltip.topbar.preview" => "Pratinjau",
        "tooltip.topbar.exitPreview" => "Keluar dari pratinjau",
        "tooltip.topbar.account" => "Akun",
        "settings.agents.providerRollMore" => "dan {{count}} lainnya",
        "ai.thinking.adaptive" => "Berpikir: otomatis",
        "ai.thinking.disabled" => "Berpikir: mati",
        "ai.thinking.enabled" => "Berpikir: aktif",
        "ai.designProgress.detail.repairsApplied" => "{{count}} perbaikan otomatis diterapkan",
        "ai.designProgress.detail.repairsMore" => "… dan {{count}} lainnya (lihat log)",
        "ai.styleCard.builtin" => "Gaya bawaan",
        "ai.styleCard.imported" => "DESIGN.md yang diimpor",
        "ai.styleCard.documentDesignMd" => "design.md dokumen",
        _ => return super::id_collab::lookup(key),
    })
}
