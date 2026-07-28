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
        _ => return super::id_collab::lookup(key),
    })
}
