
> [English](./BUILD_WINDOWS.md) · [简体中文](./BUILD_WINDOWS.zh.md) · [繁體中文](./BUILD_WINDOWS.zh-TW.md) · [日本語](./BUILD_WINDOWS.ja.md) · [한국어](./BUILD_WINDOWS.ko.md) · [Français](./BUILD_WINDOWS.fr.md) · [Español](./BUILD_WINDOWS.es.md) · [Deutsch](./BUILD_WINDOWS.de.md) · [Português](./BUILD_WINDOWS.pt.md) · [Русский](./BUILD_WINDOWS.ru.md) · **हिन्दी** · [Türkçe](./BUILD_WINDOWS.tr.md) · [ไทย](./BUILD_WINDOWS.th.md) · [Tiếng Việt](./BUILD_WINDOWS.vi.md) · [Bahasa Indonesia](./BUILD_WINDOWS.id.md)

# OpenPencil Windows बिल्ड गाइड

## 1. स्रोत कोड डाउनलोड करें

1. openpencil रिपॉज़िटरी का स्रोत कोड डाउनलोड करें

2. openpencil के `vendor` निर्देशिका में 3 निर्भरता सबमॉड्यूल हैं (jian, casement, agent) जिन्हें अलग से डाउनलोड करना होगा

    - 2.1 jian (UI फ्रेमवर्क): [https://github.com/ZSeven-W/jian](https://github.com/ZSeven-W/jian) → `vendor\jian` में निकालें

    - 2.2 casement (विंडो रैपर): [https://github.com/ZSeven-W/casement](https://github.com/ZSeven-W/casement) → `vendor\casement` में निकालें

    - 2.3 agent (Agent रनटाइम): [https://github.com/ZSeven-W/agent-rs](https://github.com/ZSeven-W/agent-rs) → `vendor\agent` में निकालें

3. निकालने के बाद, निम्नलिखित पथों के मौजूद होने की पुष्टि करें:

    ```plaintext
    vendor\jian\Cargo.toml
    vendor\casement\Cargo.toml
    vendor\agent\Cargo.toml
    ```

## 2. वातावरण तैयार करें — *Windows 10 x64 (MSVC टूलचेन)*

1. Visual Studio इंस्टॉल करते समय **"C++ के साथ डेस्कटॉप डेवलपमेंट"** वर्कलोड चुनें

2. Rust टूलचेन `stable-x86_64-pc-windows-msvc` इंस्टॉल करें

    - 2.1 rustup मिरर कॉन्फ़िगर करें (डाउनलोड गति बढ़ाने के लिए)

        ```powershell
        [Environment]::SetEnvironmentVariable("RUSTUP_DIST_SERVER", "https://rsproxy.cn", "User")
        [Environment]::SetEnvironmentVariable("RUSTUP_UPDATE_ROOT", "https://rsproxy.cn/rustup", "User")
        ```

    - 2.2 Cargo निर्भरता मिरर कॉन्फ़िगर करें। अपने उपयोगकर्ता निर्देशिका (`C:\Users\<username>\`) में जाएं, `.cargo` फ़ोल्डर खोजें या बनाएं, और उसके अंदर `config.toml` फ़ाइल बनाएं।

        `C:\Users\<username>\.cargo\config.toml` में निम्नलिखित सामग्री पेस्ट करें:

        ```toml
        [source.crates-io]
        replace-with = 'rsproxy-sparse'

        [source.rsproxy]
        registry = "https://rsproxy.cn/crates.io-index"

        [source.rsproxy-sparse]
        registry = "sparse+https://rsproxy.cn/index/"

        [registries.rsproxy]
        index = "https://rsproxy.cn/crates.io-index"

        [net]
        git-fetch-with-cli = true
        ```

    - 2.3 rustup.rs से `rustup-init.exe` डाउनलोड करें और इसे चलाएं। PowerShell में निम्नलिखित कमांड चलाएं। **`openpencil 0.8.4` से मेल खाता है**:

        ```powershell
        rustup toolchain install 1.94
        rustup default 1.94-x86_64-pc-windows-msvc
        ```

    - 2.4 WASM-JS बाइंडिंग जनरेशन के लिए wasm-bindgen-cli इंस्टॉल करें। PowerShell में निम्नलिखित कमांड चलाएं। **`openpencil 0.8.4` से मेल खाता है**:

        ```powershell
        cargo install -f wasm-bindgen-cli --version 0.2.117
        ```

## 3. बिल्ड

1. रिपॉज़िटरी रूट में PowerShell खोलें और चलाएं:

    ```powershell
    cargo build --workspace --release
    ```

    > आपको `error: failed to run custom build command for skia-bindings v0.97.2` त्रुटि मिल सकती है
    
    - 1.1 GL बैकएंड पैकेज मैन्युअल रूप से डाउनलोड करें: [skia-binaries-0.97.2 (GL)](https://github.com/rust-skia/skia-binaries/releases/download/0.97.2/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz)
    
    - 1.2 एनवायरनमेंट वेरिएबल सेट करें (अपने वास्तविक पथ से बदलें):

         ```powershell
        $env:SKIA_BINARIES_URL="file:///path/to/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz"
        ```
    
    - 1.3 पुनः बिल्ड के लिए चरण 3.1 फिर से चलाएं

2. वेब बंडल बिल्ड करें

    - 2.1 WASM लाइब्रेरी कंपाइल करें। रिपॉज़िटरी रूट में PowerShell खोलें और चलाएं:

        ```powershell
        cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
        ```

    - 2.2 JS बाइंडिंग फ़ाइलें जनरेट करें

        ```powershell
        wasm-bindgen target\wasm32-unknown-unknown\release\op_host_web.wasm --out-dir target\release\web-bundle --target web
        ```

    - 2.3 CanvasKit रेंडरिंग संसाधनों को `crates\op-host-web\assets\canvaskit` से `web-bundle` में कॉपी करें:

        ```powershell
        Copy-Item -Recurse "crates\op-host-web\assets\canvaskit" -Destination "target\release\web-bundle\canvaskit" -Force
        ```

    - 2.4 अंतिम निर्देशिका संरचना

        ```plaintext
        target\release\
        ├── openpencil-desktop.exe
        ├── op-host-web-server.exe
        ├── op.exe
        └── web-bundle\
            ├── canvaskit\
            │   ├── canvaskit.js
            │   └── canvaskit.wasm
            ├── snippets\
            ├── op_host-web.d.ts
            ├── op_host_web.js
            ├── op_host_web_bg.wasm
            └── op_host_web_bg.wasm.d.ts
        ```

## 4. dsh-openpencil प्लगइन के लिए एनवायरनमेंट वेरिएबल कॉन्फ़िगर करें

1. एडमिनिस्ट्रेटर के रूप में PowerShell चलाएं और निम्नलिखित कमांड चलाएं (पथ को वास्तविक रिपॉज़िटरी रूट से बदलें):

    ```powershell
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_EDITOR_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    ```

## 5. सत्यापन

1. openpencil-desktop.exe

    ```powershell
    cd target\release
    ./openpencil-desktop.exe --serve-web 3100
    # विंडो एक पल के लिए दिखेगी और बंद हो जाएगी — सेवा बैकग्राउंड में 3100 पोर्ट सुनती है
    ```

2. op-host-web-server.exe  <i>` (dsh प्लगइन के लिए आवश्यक नहीं) `</i>

    ```powershell
    cd target\release
    ./op-host-web-server.exe --serve-web 3100
    # आपको PowerShell में दो "openpencil-desktop --serve-web:" पंक्तियाँ और 3100 पोर्ट दिखेगा
    ```

---

# समस्या निवारण

1. WASM बिल्ड करते समय `--features` पैरामीटर पर ध्यान दें

    ```powershell
    cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
    ```

    > `--features "web"` के बजाय `--features "web canvaskit"` का उपयोग करें।
    > 
    > `--features "web"` से कंपाइल सफल होती है, लेकिन जनरेट की गई JS फ़ाइल `mount_ck` फ़ंक्शन एक्सपोर्ट नहीं करती।
    > 
    >

2. सर्वर पूरे `web-bundle` निर्देशिका को HTTP `/pkg/` रूट पर मैप करता है

    > डिस्क पथ `web-bundle/op_host_web.js` → ब्राउज़र URL `/pkg/op_host_web.js`
    > 
    > यदि पेज बताता है कि `/pkg/op_host_web.js` नहीं मिला, तो `web-bundle/` में फ़ाइल गायब है।

3. यह दस्तावेज़ `openpencil 0.8.4` संस्करण के लिए लिखा गया है। rustup टूलचेन संस्करण `1.94`、wasm-bindgen-cli संस्करण `0.2.117` और skia-bindings संस्करण `0.97.2` इस संस्करण के अनुरूप हैं — अन्य संस्करण भिन्न हो सकते हैं।
