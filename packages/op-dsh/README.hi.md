<p align="center">
  <img src="./docs/images/dsh-openpencil-logo.png" alt="DSH OpenPencil" width="120" />
</p>

<h1 align="center">DSH OpenPencil</h1>

<p align="center">
  <strong>OpenPencil के लिए DeepSeek Harness प्लगइन — बातचीत के अंदर वास्तविक <code>.op</code> दस्तावेज़ों का पूर्वावलोकन, निरीक्षण और संपादन।</strong><br />
  <sub>सटीक मल्टी-फ़्रेम पूर्वावलोकन &bull; इंटरैक्टिव कैनवास &bull; प्रबंधित संपादक &bull; एजेंट-नेटिव डिज़ाइन टूल</sub>
</p>

<p align="center">
  <sub>npm: <a href="https://www.npmjs.com/package/@zseven-w/dsh-openpencil"><code>@zseven-w/dsh-openpencil</code></a> · वर्तमान प्लगइन रिलीज़: <code>0.1.0-rc.7</code> · DSH <code>0.1.1-rc.2</code> तक परीक्षित</sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md"><b>हिन्दी</b></a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/@zseven-w/dsh-openpencil"><img src="https://img.shields.io/npm/v/%40zseven-w%2Fdsh-openpencil?style=flat&color=cfb537" alt="npm" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/dsh-check.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/dsh-check.yml?label=CI" alt="CI" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat&color=cfb537" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil?color=64748b" alt="License" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/badge/Discord-Join%20chat-5865F2?logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <img src="./docs/images/dsh-openpencil-overview.png" alt="DSH OpenPencil — मल्टी-फ़्रेम पूर्वावलोकन और साइडबार संपादक" width="100%" />
</p>
<p align="center"><sub>इंटरैक्टिव कैनवास और प्रबंधित संपादक वर्कबेंच के साथ सटीक मल्टी-फ़्रेम <code>.op</code> पूर्वावलोकन</sub></p>

## DSH OpenPencil क्यों

DSH OpenPencil [DeepSeek Harness](https://github.com/deepseek-ai/DSH) को [OpenPencil](https://github.com/ZSeven-W/openpencil) से जोड़ता है, ताकि कोई एजेंट जनित छवि लौटाने के बजाय एक वास्तविक, संपादन-योग्य, इंटरैक्टिव डिज़ाइन कैनवास को चलाए।

<table>
<tr>
<td width="50%">

### 🖼️ सटीक मल्टी-फ़्रेम पूर्वावलोकन

इंस्टॉल किया गया OpenPencil हेडलेस एक्सपोर्टर डिज़ाइन-वफ़ादार पूर्वावलोकन प्रस्तुत करता है: पहला टॉप-लेवल फ़्रेम एक बड़ी रीप्ले-सुरक्षित PNG के रूप में, साथ ही क्षैतिज रूप से स्क्रॉल होने वाली थंबनेल पट्टी, क्लिक-टू-सिलेक्ट, और मल्टी-फ़्रेम दस्तावेज़ों के लिए पिछला/अगला नेविगेशन।

</td>
<td width="50%">

### 🗺️ इंटरैक्टिव कैनवास

"इंटरैक्टिव कैनवास खोलें" पैन, ज़ूम और फ़िट के साथ रीड-ओनली OpenPencil Web SDK को आलसी ढंग से माउंट करता है — बातचीत से बाहर निकले बिना किसी भी पेज, नेस्टेड नोड या निष्क्रिय पेज का निरीक्षण करें।

</td>
</tr>
<tr>
<td width="50%">

### ✏️ प्रबंधित संपादक

`editable: true` के साथ, संपादन क्रिया प्रबंधित OpenPencil संपादक खोलती है — चयन, परतें, गुण, ड्रॉइंग टूल, पूर्ववत/पुनः करना, और स्पष्ट सेव सेमैंटिक्स — पूर्ण-स्क्रीन विकल्प के साथ एक आकार-बदलने योग्य दाएँ-हाथ वर्कबेंच में।

</td>
<td width="50%">

### 🤖 एजेंट-नेटिव डिज़ाइन टूल

पाँच direct-canvas टूल और छह `openpencil_pipeline_*` टूल एजेंट को managed OpenPencil runtimes के माध्यम से वास्तविक कैनवास बनाने, जाँचने, निखारने, प्रकाशित करने, बदलने और पढ़ने देते हैं।

</td>
</tr>
<tr>
<td width="50%">

### 🔐 क्षमता-गेटेड अनुदान

छवि और दस्तावेज़ अनुदान हस्ताक्षरित, हैश-बद्ध क्षमताएँ हैं। ब्राउज़र मेटाडेटा कभी भी कोई मनमाना होस्ट पथ उजागर नहीं करता, और हस्ताक्षरित पूर्वावलोकन/संपादक क्षमताएँ कभी भी कैनोनिकल टूल परिणाम या मॉडल संदर्भ में प्रवेश नहीं करतीं।

</td>
<td width="50%">

### ⚡ ट्रांज़ैक्शनल सुरक्षा

पूर्ण pipeline का दस्तावेज़ सभी native और DSH quality gates पार करने तक निजी, अप्रकाशित draft रहता है। प्रकाशन किसी मौजूदा पथ को अधिलेखित नहीं करता, और abort या असफल batch कोई खाली target नहीं छोड़ता।

</td>
</tr>
<tr>
<td width="50%">

### 🌍 DSH के लुक एंड फील का पालन करता है

टूल कार्ड और प्रबंधित संपादक संपादन सत्र को पुनः लोड किए बिना DSH के चीनी/अंग्रेज़ी लोकेल और लाइट/डार्क थीम का पालन करते हैं।

</td>
<td width="50%">

### 🎯 एक संपूर्ण वर्कफ़्लो

"आवश्यकता → sidebar में private live canvas → दो semantic batches और user-visible exact PNG → native/DSH deterministic quality gates → atomic प्रकाशन" — DSH के भीतर एक पूर्ण चक्र।

</td>
</tr>
</table>

## DSH में इंस्टॉल करें

DSH अलग पैकेज है। अगर पहले से नहीं है तो एक बार इंस्टॉल करें:

```sh
npm install -g @deepseek-ai/dsh@latest
```

फिर प्लगइन को किसी प्रोफ़ाइल में जोड़ें और वेब ऐप शुरू करें:

```sh
dsh plugin --profile web add @zseven-w/dsh-openpencil@next
dsh web
```

स्थानीय विकास के लिए इस checkout को build करें, इसके absolute path को Web प्रोफ़ाइल में link करें और फिर DSH को पूरी तरह restart करें:

```sh
pnpm run build
dsh plugin --profile web add link:/absolute/path/to/dsh-openpencil
dsh web
```

`link:` dependency बाद के rebuild को इसी checkout से सीधे उपलब्ध कराती है। लेकिन profile dependency बदलने के बाद DSH को पूरी तरह restart करना आवश्यक है, क्योंकि शामिल Web profile डिफ़ॉल्ट रूप से host bundle को hot-reload नहीं करती।

DSH को ग्लोबली इंस्टॉल नहीं करना चाहते? वही दो चरण `pnpm dlx` से चलाएँ:

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh plugin --profile web add @zseven-w/dsh-openpencil@next
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

> OpenPencil प्लगइन सार्वजनिक है और इसके लिए किसी npm टोकन की आवश्यकता नहीं है। यदि DSH प्री-रिलीज़ को स्वयं रजिस्ट्री प्रमाणीकरण की आवश्यकता होती है, तो उस क्रेडेंशियल को चेकआउट के बाहर उपयोगकर्ता-स्तरीय या अस्थायी npm कॉन्फ़िग में रखें। इस रिपॉज़िटरी में जानबूझकर कोई रजिस्ट्री क्रेडेंशियल नहीं है।

## डिज़ाइन टूल

वैकल्पिक शैली क्रम `openpencil_style_plan` → `openpencil_style_confirm` → `openpencil_pipeline_begin` है; मूल brief और `style_selection` दें। कैटलॉग और सटीक गाइड चयन को मॉडल अनुशंसा की जरूरत नहीं है; प्रदाता डिफ़ॉल्ट रूप से बंद है। यह हस्तांतरण script को समर्थन देता है, app-agent और commerce को नहीं। सामान्य app-agent निर्माण उपलब्ध रहता है।

| टूल | यह क्या करता है |
| --- | --- |
| `openpencil_new` | सरल कामों के लिए संगत तेज़ पथ: एक transactional QuickJS `batch_design` script चलाता है, target न होने पर ही प्रकाशित करता है और editable presentation लौटाता है। production design के लिए नीचे की पूर्ण pipeline चुनें। |
| `openpencil_pipeline_begin` | owner-session का निजी draft और उसका एकमात्र root शुरू करता है, उसी live canvas को sidebar में तुरंत खोलता है और target `.op` को अप्रकाशित रखता है। |
| `openpencil_pipeline_context` | केवल तब एक सीमित guideline, style, theme या UI-kit detail लोड करता है जब begin contract में वह सचमुच न हो; यह startup refresh loop नहीं है। |
| `openpencil_pipeline_batch` | अधिकतम दो सीधे QuickJS generation scripts चलाता है। हर सफल transaction user-visible exact PNG दिखाने की कोशिश करती है; tool लौटते ही उसके `next` पर आगे बढ़ें। |
| `openpencil_pipeline_inspect` | केवल user के स्पष्ट diagnostic अनुरोध पर manual inspection देता है; सामान्य generation इसे preview या model image-review step की तरह उपयोग नहीं करती। |
| `openpencil_pipeline_finish` | native और DSH deterministic gates चलाकर exact post-final PNG render करता है और `createIfAbsent` से atomically publish करता है। केवल `needs_correction`, `canContinue: true`, complete non-empty `repairTargets` और `omitted: 0` वाला result ही एक U-only repair और एक अंतिम finish अधिकृत करता है; बाकी हर unpublished result terminal है। |
| `openpencil_pipeline_abort` | target file बनाए बिना अप्रकाशित draft छोड़ देता है। |
| `openpencil_create` | किसी मौजूदा लाइव कैनवास पर नोड उत्पन्न करने या पुनर्संरचना करने के लिए एक ट्रांज़ैक्शनल `batch_design` प्रोग्राम लागू करता है। |
| `openpencil_edit` | एक स्पष्ट नोड या उपयोगकर्ता द्वारा चयनित एकमात्र नोड को संशोधित करता है। |
| `openpencil_render` | एक अपरिवर्तनीय, कंटेंट-एड्रेस्ड `.op` स्नैपशॉट बनाता है और सक्रिय पेज पर हर टॉप-लेवल फ़्रेम प्रस्तुत करता है — वैकल्पिक `scale` और `editable`। |
| `openpencil_selection` | लाइव संपादक कैनवास में चयनित सटीक नोड्स को पढ़ता है। |

## एजेंट डिज़ाइन वर्कफ़्लो

सामान्य generation के लिए तय छोटा पथ अपनाएँ: `openpencil_pipeline_begin` → दो सीधे QuickJS `openpencil_pipeline_batch` → एक `openpencil_pipeline_finish`। Begin एकमात्र root बनाकर निजी live canvas को sidebar में तुरंत खोल देता है; माँगा गया workspace path सफल publication से पहले मौजूद नहीं होता। हर सफल begin या batch के बाद अगली आवश्यक call बिना narration, planning, comparison, inspection या unrelated tool के तुरंत करें। Default में पूरे design में ठीक एक image हो; हर Hero/Product/Art/Media frame में ठीक एक primary visual हो, image और placeholder icon साथ न हों।

दोनों सफल batches user को exact PNG preview दिखाने की कोशिश करते हैं। Tool लौटने के बाद तुरंत उसके `next` का पालन करें। यदि `next` में `previewUnavailable` हो, तो script पहले ही live canvas पर commit हो चुकी है: batch को दोबारा न चलाएँ और `openpencil_pipeline_inspect` या `read_image` न बुलाएँ। `openpencil_pipeline_inspect` केवल user द्वारा स्पष्ट रूप से माँगे गए diagnostics के लिए है।

Finish OpenPencil की native finalization, lint, contrast और layout checks तथा DSH deterministic quality gates चलाता है, फिर उसी healthy call में exact final PNG render करके target को atomically publish करता है। Repair केवल तभी अधिकृत है जब result में एक साथ `stage: "needs_correction"`, `canContinue: true`, complete non-empty `repairTargets` और `omitted: 0` हों, तथा हर target में `operation: "U"`, exact non-empty `nodeId` और non-empty `patch` हो। सभी targets को ठीक एक U-only batch में साथ लागू करें और बिना narration finish को ठीक एक अंतिम बार चलाएँ। कोई अन्य unpublished result, error या `canContinue: false` terminal है: उसे एक बार report करें और retry, inspect, image/context read, abort या replacement draft न करें। Gate विफल होने या `openpencil_pipeline_abort` पर target अनुपस्थित रहता है। Published result की exact final PNG और live editor पहले से authoritative हैं; उसे लौटाकर तुरंत समाप्त करें। Sidebar idle होने पर ही auto-open होता है और explicit switch के लिए **कैनवास संपादित करें** उपलब्ध रहता है।

उसी चल रही DSH service में browser बदलने या reload करने के बाद, strictly parsed durable publication को `openpencil_new` या `openpencil_pipeline_finish` से exact PNG और स्पष्ट **कैनवास संपादित करें** action के रूप में restore किया जा सकता है। Historical card sidebar को अपने-आप नहीं खोलता; user को वही action क्लिक करना होगा। सामान्य historical `openpencil_render` read-only रहता है, और non-loopback connection को editor grant कभी नहीं मिलता।

बंडल किया हुआ `openpencil-design` skill scripting और quality guide बना रहता है, और managed runtime desktop binary पर निर्भर नहीं है। `openpencil_new` संगत single-batch fast path के रूप में उपलब्ध रहता है, लेकिन production-quality generation में पूर्ण pipeline को प्राथमिकता दें।

`openpencil_create` और `openpencil_edit` का उपयोग केवल किसी मौजूदा लाइव कैनवास के लिए करें। संपादक की सेव क्रिया तक उनके संपादन असेवित रहते हैं।

## वेब व्यूअर एसेट

DSH क्लाइंट प्लगइन के लिए केवल `client.js` परोसता है, इसलिए OpenPencil ESM SDK, उसका WASM और CanvasKit को स्पष्ट समान-मूल (same-origin) एसेट के रूप में स्टेज किया जाता है:

```sh
pnpm run sync:viewer-assets
```

सिंक कमांड डिफ़ॉल्ट रूप से मूल OpenPencil मोनोरिपो रूट (`../..`) का उपयोग करता है और सहोदर `../openpencil` चेकआउट पर फ़ॉलबैक करता है। इसे `OPENPENCIL_ROOT` या `--openpencil-root` से ओवरराइड करें। `DSHPLUGIN_OPENPENCIL_VIEWER_SOURCE` से एक संपूर्ण पूर्व-निर्मित एसेट निर्देशिका चुनी जा सकती है। `DSHPLUGIN_OPENPENCIL_VIEWER_ASSET_DIR` से रनटाइम लुकअप को ओवरराइड किया जा सकता है।

व्यूअर एसेट उपयोगकर्ता के कैनवास खोलने के बाद ही आलसी-लोड होती हैं। यदि वे अनुपस्थित या अमान्य हैं, तो PNG पूर्वावलोकन उपलब्ध रहता है और कोई कैनवास बटन विज्ञापित नहीं किया जाता।

## प्रबंधित संपादक

संपादन-योग्य सत्र OpenPencil के प्रबंधित वेब होस्ट का उपयोग करते हैं — वही आर्किटेक्चर जो `op-vscode` द्वारा उपयोग किया जाता है। प्लगइन होस्ट को केवल एक अधिकृत उपयोगकर्ता क्रिया के बाद शुरू करता है, डेमन टोकन को मेमोरी में रखता है, iframe स्रोत और ओरिजिन को मान्य करता है, और संपादक सत्र समाप्त होने पर प्रक्रिया को बंद कर देता है। संपादक सतह चुनिंदा रूप से चुनी जाती है: जब होस्ट उस सीम की घोषणा करता है तो नेटिव Tool विवरण, अन्यथा रीसाइज़ और फ़ुल-स्क्रीन नियंत्रणों के साथ प्लगइन का दाएँ-हाथ वर्कबेंच।

स्टार्टअप धीमे mount के लिए सुरक्षित listening handshake का उपयोग करता है: readiness probe तभी शुरू होती हैं जब साथ दिया गया host अपना bind किया हुआ address घोषित कर दे। डेस्कटॉप OpenPencil इंस्टॉल करना आवश्यक नहीं है।

प्रकाशित इंस्टॉलेशन छह मूल प्लेटफ़ॉर्म पैकेजों — `darwin-arm64`, `darwin-x64`, `linux-arm64`, `linux-x64`, `win32-arm64` और `win32-x64` — में से वर्तमान OS/CPU के लिए उपयुक्त पैकेज चुनते हैं; दोनों Linux पैकेज glibc को लक्षित करते हैं। रूट पैकेज इन्हें सटीक संस्करण वाली `optionalDependencies` के रूप में घोषित करता है, ताकि पैकेज मैनेजर सही विकल्प चुने (उदाहरण के लिए `@zseven-w/dsh-openpencil-darwin-arm64`)। यह पैकेज `op-host-web-server`, संपादक वेब बंडल और CanvasKit को एक संगत रनटाइम के रूप में साथ देता है। इसलिए प्रबंधित संपादक `/Applications/OpenPencil.app`, `PATH` में `openpencil-desktop` या OpenPencil स्रोत checkout पर निर्भर नहीं करता।

यदि कैनवास गंदा (dirty) होने पर DSH प्लगइन को पुनः लोड या अनलोड करता है, तो होस्ट सात दिनों तक एक अपारदर्शी स्थानीय रिकवरी ड्राफ़्ट रखता है। उसी स्रोत को फिर से खोलने पर उसे लाइव कैनवास में पुनर्स्थापित करने से पहले पूछा जाता है; जब तक उपयोगकर्ता स्पष्ट रूप से सेव नहीं करता, रिकवरी कभी भी `.op` फ़ाइल को अधिलेखित नहीं करती।

सभी छह प्लेटफ़ॉर्म के आधिकारिक पैकेजों में सुरक्षित release build के दौरान चीन और ग्लोबल सहयोग bootstrap endpoints डाले और सत्यापित किए जाते हैं; सत्यापन के बाद ही उन्हें प्रकाशित किया जाता है। इस injection के बिना स्थानीय self-build में DSH शुरू करने से पहले `OPENPENCIL_COLLAB_BOOTSTRAP_URL=https://<your-host>/api/v1/collaboration/bootstrap` से bootstrap को override किया जा सकता है। मान में `https` होना चाहिए और path ठीक `/api/v1/collaboration/bootstrap` होना चाहिए।

डिवाइसों के बीच कैनवास सिंक के लिए PC/DSH नेटिव runtime और मोबाइल App दोनों को उसी OpenPencil release line पर अपडेट करना आवश्यक है जिसमें वर्तमान collaboration queue fix शामिल हो। पुराने मोबाइल App को नए PC runtime के साथ मिलाने पर remote cursor दिखाई दे सकते हैं, लेकिन canvas commits प्राप्त नहीं हो सकते।

इस रिपॉज़िटरी से विकास करते समय DSH शुरू करने से पहले क्रमशः editor Web bundle और native host build करें, फिर उस मेल खाते runtime को stage करें।

`pnpm run build:editor-web` OpenPencil के आधिकारिक रूप से समर्थित WASM bundle gate को चलाता है। इसके लिए Bash, `wasm32-unknown-unknown` target वाला Cargo/Rust, `wasm-bindgen` CLI, Binaryen का `wasm-opt`, Node.js और `gzip` आवश्यक हैं; CanvasKit को EMSDK की आवश्यकता नहीं है। Web build collaboration bootstrap build variables का उपयोग नहीं करता। `pnpm run build:editor-runtime` से पहले `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN` और `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL` दोनों सेट करें। ये केवल native Cargo build में उपयोग होते हैं, जो किसी एक के अनुपस्थित होने पर fail closed होगा। दोनों build सफल होने के बाद अंतिम command से runtime को stage करें।

```sh
pnpm run build:editor-web
pnpm run build:editor-runtime
pnpm run stage:editor-runtime
```

स्पष्ट रनटाइम ओवरराइड केवल एक पूर्ण और परस्पर संगत सेट के रूप में स्वीकार किए जाते हैं:

- `DSHPLUGIN_OPENPENCIL_EDITOR_BINARY` `op-host-web-server` के लिए;
- `DSHPLUGIN_OPENPENCIL_EDITOR_WEB_BUNDLE_DIR` निर्मित संपादक वेब बंडल के लिए;
- `DSHPLUGIN_OPENPENCIL_EDITOR_CANVASKIT_DIR` CanvasKit एसेट के लिए।

सेट का केवल एक भाग देना अमान्य है; प्लगइन कस्टम पथों को पैकेज किए गए रनटाइम एसेट के साथ नहीं मिलाता।

सेव एक आशावादी स्रोत हैश, एक परमाणु प्रतिस्थापन और एक उत्तराधिकारी क्षमता का उपयोग करते हैं। यदि स्रोत संपादक के बाहर बदलता है, तो प्लगइन उसे अधिलेखित करने के बजाय संघर्ष की रिपोर्ट करता है।

## परिणाम मेटाडेटा

मॉडल-दृश्यमान परिणाम सादा JSON ही रहता है। केवल-ब्राउज़र `presentationMeta.$dshOpenPencil` इनके लिए अतिरिक्त अनुदान वहन करता है:

- `image`: PNG पथ, पूर्वावलोकन/डाउनलोड URL, और वास्तविक चौड़ाई/ऊँचाई;
- `frames`: सक्रिय-पेज क्रम में हर सटीक-रेंडर किया गया टॉप-लेवल फ़्रेम, जिसमें उसका नोड id/नाम/इंडेक्स और हस्ताक्षरित PNG URL शामिल हैं;
- `document`: स्रोत क्रिया पथ के साथ अपरिवर्तनीय स्नैपशॉट URL, बाइट्स और SHA-256;
- `viewer`: जब एसेट रूट संलग्न हो तो संशोधित-संस्करणित SDK/WASM/CanvasKit URL;
- `editor`: जब `editable: true` अधिकृत हो तो स्कोप्ड लॉन्च/रिफ़्रेश क्षमताएँ।

परिणाम `renderer`, `rendererBinary`, `fidelity` और कोई भी चेतावनी भी दर्ज करता है। मौजूदा केवल-PNG schema-v1 संदेश रेंडर करने योग्य बने रहते हैं।

DSH `0.1.1-rc.2` PTC/Code Mode के अंतर्गत नेस्टेड टूल के लिए ब्राउज़र प्रस्तुति मेटाडेटा को बनाए नहीं रखता। प्लगइन उस केवल-UI प्रक्षेपण को एक समान-मूल, सत्र-बद्ध एंडपॉइंट के माध्यम से पुनर्प्राप्त करता है: ब्राउज़र केवल सत्र id, कॉल id और अपरिवर्तनीय दस्तावेज़ SHA-256 भेजता है, जबकि होस्ट स्थायी DSH सत्र लॉग से प्रामाणिक परिणाम हल करता है और हाल के लाइव संपादन को अधिकृत करने के लिए केवल एक अल्पकालिक इन-प्रोसेस मार्कर का उपयोग करता है। हस्ताक्षरित पूर्वावलोकन/संपादक क्षमताएँ कभी भी कैनोनिकल टूल परिणाम या मॉडल संदर्भ में प्रवेश नहीं करतीं। सामान्य `openpencil_render` का durable history read-only रहता है। `openpencil_new` या `openpencil_pipeline_finish` की strictly parsed durable publication को editor grant केवल loopback पर और user के explicit click के बाद मिल सकता है; sidebar auto-open केवल recent, trusted live results के लिए है।

सीमित रीप्ले के लिए, नेस्टेड मेटाडेटा रिकवरी 128 तक टॉप-लेवल फ़्रेम स्वीकार करती है; बड़े Code Mode परिणाम अपने कैनोनिकल JSON फ़ॉलबैक के माध्यम से उपलब्ध रहते हैं।

## वर्तमान सीमाएँ

- किसी मौजूदा कैनवास पर अनुवर्ती संपादनों के लिए पहले से खुले प्रबंधित संपादक की आवश्यकता होती है। जब तक उपयोगकर्ता उसकी सेव क्रिया नहीं चलाता, परिवर्तन असेवित रहते हैं।
- हल्का Web SDK कैनवास रीड-ओनली है; पूर्ण संपादन अलग प्रबंधित संपादक सतह का उपयोग करता है। DSH `0.1.1-rc.2` पर, प्लगइन फ़ुल-स्क्रीन विकल्प के साथ आकार-बदलने योग्य दाएँ वर्कबेंच का उपयोग करता है।
- सटीक गैलरी सक्रिय पेज पर टॉप-लेवल फ़्रेम को कवर करती है; निष्क्रिय पेजों और नेस्टेड नोड्स के निरीक्षण का तरीका इंटरैक्टिव कैनवास ही रहता है।
- रेंडर और स्नैपशॉट कैश को अभी भी उत्पाद-स्तरीय प्रतिधारण नीति की आवश्यकता है।

## प्रोजेक्ट संरचना

```text
packages/op-dsh/
├── src/                       Plugin sources (TypeScript)
│   ├── index.ts               Host plugin entry — Cordis service, tools, assets
│   ├── tool.ts / design-tools.ts / new-tool.ts   Host-side design tools
│   ├── renderer.ts            Exact OpenPencil renderer + Jian fallback
│   ├── editor-host.ts / editor-recovery.ts       Managed editor lifecycle + drafts
│   ├── viewer-assets.ts       Web SDK / WASM / CanvasKit asset staging
│   ├── mcp-client.ts          OpenPencil MCP connection
│   └── client/                Browser client — React workbench, gallery, selection dock
├── lib/                       Compiled output (published to npm)
├── scripts/                   Build helpers — viewer asset sync, client build, host tests
├── tests/                     Node test suites (client, host API, MCP, viewer assets)
├── docs/images/               Documentation screenshots
├── cordis.patch.yml           DSH bundle patch that mounts the plugin
├── tsconfig.json              Host / Node TypeScript config
└── tsconfig.client.json       Browser client TypeScript config
```

## बिल्ड और सत्यापन

```sh
pnpm run sync:viewer-assets
pnpm run build
pnpm run test:viewer-assets
pnpm run test:client
pnpm run test:host /absolute/path/to/design.op 375 1091
```

बिल्ड के लिए Node 24.11 या उससे नया संस्करण और pnpm आवश्यक है। DSH होस्ट/क्लाइंट पैकेज लक्षित DSH प्रोफ़ाइल द्वारा प्रदान की गई पीयर निर्भरताएँ हैं। बिल्ड टूल स्थानीय डेव निर्भरताओं, सक्रिय लिंक किए गए DSH चेकआउट या इंस्टॉल किए गए DSH स्रोत बंडल से हल किए जाते हैं; `DSH_SOURCE_ROOT` स्पष्ट रूप से एक स्रोत चेकआउट चुन सकता है। जब उस वातावरण को अलग से प्रावधानित किया जाता है, तो लॉकफ़ाइल स्टैंडअलोन सार्वजनिक बिल्ड टूलिंग को पिन करती है।

निजी DSH प्री-रिलीज़ के लिए, जारी किया गया npm क्रेडेंशियल इस रिपॉज़िटरी के बाहर रखें (उदाहरण के लिए उपयोगकर्ता-स्तरीय या अस्थायी `.npmrc` में) और अनुरोधित संस्करण सीधे चलाएँ:

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

`.npmrc`, `NPM_TOKEN` या कॉपी किए गए रजिस्ट्री क्रेडेंशियल कभी कमिट न करें। यह रिपॉज़िटरी डिफ़ॉल्ट रूप से स्थानीय npm कॉन्फ़िगरेशन को अनदेखा करती है।

`test:host` एक वास्तविक सटीक रेंडर करता है, PNG IHDR ज्यामिति और SHA-256 को मान्य करता है, HTTP पर अपरिवर्तनीय छवि/दस्तावेज़ क्षमताओं का परीक्षण करता है, और जाँचता है कि व्यूअर एसेट अनुदेय (grantable) हैं। अपेक्षित आयाम फिक्स्चर-विशिष्ट होते हैं।

## इकोसिस्टम

DSH OpenPencil **[OpenPencil](https://github.com/ZSeven-W/openpencil)** के लिए DeepSeek Harness प्लगइन है — दुनिया का पहला ओपन-सोर्स AI-नेटिव वेक्टर डिज़ाइन टूल — और शुद्ध-Rust, AI-नेटिव टूल के **[ZSeven-W](https://github.com/ZSeven-W)** परिवार का हिस्सा है।

| प्रोजेक्ट | यह क्या है |
| ------- | ---------- |
| **[OpenPencil](https://github.com/ZSeven-W/openpencil)** | वह डिज़ाइन टूल जिसे यह प्लगइन चलाता है — प्रॉम्प्ट-टू-कैनवास जनरेशन, समवर्ती एजेंट टीमें, डिज़ाइन-एज़-कोड `.op` फ़ाइलें और एक अंतर्निहित MCP सर्वर। यहाँ के सटीक पूर्वावलोकन, इंटरैक्टिव कैनवास और प्रबंधित संपादक OpenPencil द्वारा ही संचालित हैं। |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | LLM एजेंट भेजने के लिए एक शुद्ध-Rust async रनटाइम — मल्टी-प्रोवाइडर, एंड-टू-एंड टूल-सक्षम, संरचित अनुमतियाँ, वास्तविक MCP, शून्य `unsafe`। OpenPencil के अंतर्निहित एजेंट रनटाइम को शक्ति देता है। |
| **[jian](https://github.com/ZSeven-W/jian)** | शुद्ध-Rust, GPU-Skia UI फ्रेमवर्क — एक ही स्टैक में विजेट, लेआउट, इवेंट और हॉट रीलोड। OpenPencil का UI फ्रेमवर्क, और इस प्लगइन के फ़ॉलबैक रेंडरर का स्रोत। |
| **[Zode](https://github.com/ZSeven-W/zode)** | आपके टर्मिनल के लिए ओपन-सोर्स, AI-नेटिव कोडिंग सहायक — आपका कोड पढ़ता है, कमांड चलाता है और MCP के माध्यम से OpenPencil चलाता है। |
| **[noema](https://github.com/ZSeven-W/noema)** | कोडिंग एजेंट के लिए लोकल-फ़र्स्ट, नॉन-वेक्टर मेमोरी सिस्टम — निरीक्षण-योग्य फ़ाइलों के रूप में स्थायी मेमोरी, रनटाइम में काम करती है। |
| **[openpencil-skill](https://github.com/ZSeven-W/openpencil-skill)** | LLM स्किल प्लगइन जो AI एजेंटों को `op` के साथ डिज़ाइन करना सिखाता है — इस DSH प्लगइन का साथी। |

DSH के अन्य प्लगइन:

- [DSH Android](https://github.com/ZSeven-W/dsh-android) — बातचीत के भीतर लाइव Android एमुलेटर या USB डिवाइस — पूरी तरह adb से संचालित
- [DSH Crew](https://github.com/ZSeven-W/dsh-crew) — Claude Code / Codex से DSH एजेंट को काम सौंपें
- [DSH iOS](https://github.com/ZSeven-W/dsh-ios) — बातचीत के भीतर चलता iOS सिम्युलेटर — और USB से जुड़ा iPhone
- [DSH Noema](https://github.com/ZSeven-W/dsh-noema) — DSH के लिए दीर्घकालिक स्मृति

## योगदान

योगदान का स्वागत है! Fork और clone करें, एक ब्रांच बनाएँ, `pnpm run build` और टेस्ट सुइट चलाएँ, [Conventional Commits](https://www.conventionalcommits.org/) के साथ कमिट करें, और `main` के विरुद्ध एक PR खोलें।

## समुदाय

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="https://raw.githubusercontent.com/ZSeven-W/openpencil/main/screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> हमारे Discord से जुड़ें</strong>
</a>
— प्रश्न पूछें, डिज़ाइन साझा करें, सुविधाएँ सुझाएँ।

**मान्यता प्राप्त समुदाय: [LINUX DO](https://linux.do/)**

## लाइसेंस

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

तृतीय-पक्ष घटक [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md) में सूचीबद्ध हैं।
