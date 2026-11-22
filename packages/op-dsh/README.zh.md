<p align="center">
  <img src="./docs/images/dsh-openpencil-logo.png" alt="DSH OpenPencil" width="120" />
</p>

<h1 align="center">DSH OpenPencil</h1>

<p align="center">
  <strong>OpenPencil 的 DeepSeek Harness 插件 —— 在对话中预览、检查并编辑真实的 <code>.op</code> 文档。</strong><br />
  <sub>精确多帧预览 &bull; 交互式画布 &bull; 托管编辑器 &bull; 智能体原生设计工具</sub>
</p>

<p align="center">
  <sub>npm: <a href="https://www.npmjs.com/package/@zseven-w/dsh-openpencil"><code>@zseven-w/dsh-openpencil</code></a> · 当前插件版本：<code>0.1.0-rc.16</code> · 编辑器面板已在 DSH 桌面版 <code>0.2.1-rc.3</code> 验证</sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md"><b>简体中文</b></a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
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
  <img src="./docs/images/dsh-openpencil-overview.png" alt="DSH OpenPencil —— 多帧预览与侧边栏编辑器" width="100%" />
</p>
<p align="center"><sub>带交互式画布与托管编辑器工作台的精确多帧 <code>.op</code> 预览</sub></p>

## 为什么选择 DSH OpenPencil

DSH OpenPencil 将 [DeepSeek Harness](https://github.com/deepseek-ai/DSH) 与 [OpenPencil](https://github.com/ZSeven-W/openpencil) 连接起来，让智能体（Agent）驱动一个真实、可编辑、可交互的设计画布，而不是返回一张生成的图片。

<table>
<tr>
<td width="50%">

### 🖼️ 精确多帧预览

已安装的 OpenPencil 无头导出器会渲染忠实于设计的预览：第一个顶层帧以大型可回放 PNG 呈现，另有一条可水平滚动的缩略图栏，支持点击选择以及多帧文档的上一个/下一个导航。

</td>
<td width="50%">

### 🗺️ 交互式画布

「打开交互式画布」会按需挂载只读的 OpenPencil Web SDK，支持平移、缩放与适应视图 —— 无需离开对话即可检查任意页面、嵌套节点或非活动页面。

</td>
</tr>
<tr>
<td width="50%">

### ✏️ 托管编辑器

启用 `editable: true` 后，编辑操作会打开托管的 OpenPencil 编辑器 —— 包含选择、图层、属性、绘图工具、撤销/重做以及明确的保存语义 —— 呈现在一个可调整大小的右侧工作台中，并支持全屏选项。

</td>
<td width="50%">

### 🤖 智能体原生设计工具

五个直接操作画布的工具，加上六个 `openpencil_pipeline_*` 工具，让智能体能通过托管的 OpenPencil 运行时创建、检查、细化、发布、修改和读取真实画布。

</td>
</tr>
<tr>
<td width="50%">

### 🔐 能力门控授权

图像与文档授权是经过签名、与哈希绑定的能力凭据。浏览器元数据永远不会暴露任意的宿主机路径，签名的预览/编辑器能力也永远不会进入规范的工具结果或模型上下文。

</td>
<td width="50%">

### ⚡ 事务性安全

完整管线中的文档会一直保留在私有、未发布的草稿中，直到通过全部原生与 DSH 质量门禁。发布绝不会覆盖已有路径，中止或批次失败也不会留下空的目标文件。

</td>
</tr>
<tr>
<td width="50%">

### 🌍 遵循 DSH 外观与风格

工具卡片与托管编辑器会跟随 DSH 的中文/英文语言环境以及浅色/深色主题，无需重新加载编辑会话。

</td>
<td width="50%">

### 🎯 一个完整的工作流

「需求 → 私有草稿 → 语义化批次 → live 用户预览 → 确定性的结构/布局/质量校验 → 最终 PNG 完整性保护下的原子发布」—— 全部在 DSH 内完成闭环。

</td>
</tr>
</table>

## 安装到 DSH

DSH 是独立的包。若尚未安装，先装一次：

```sh
npm install -g @deepseek-ai/dsh@latest
```

然后把插件装进某个 profile 并启动 Web 应用：

```sh
dsh plugin --profile web add @zseven-w/dsh-openpencil@latest
dsh web
```

插件目前仍处于预发布阶段，发布版本使用 npm 的 `latest` 标签；从 `latest` 安装即可获取当前版本。较旧的 `next` 标签已不是当前发布通道。

本地开发时，先构建当前检出目录，再把其绝对路径链接进 Web profile，随后完整重启 DSH：

```sh
pnpm run build
dsh plugin --profile web add link:/absolute/path/to/dsh-openpencil
dsh web
```

`link:` 依赖会让后续重新构建的产物直接从当前检出目录生效；但替换 profile 依赖后必须完整重启 DSH，因为随附的 Web profile 默认不会热重载宿主插件包。

不想全局安装 DSH？用 `pnpm dlx` 跑同样的两步：

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh plugin --profile web add @zseven-w/dsh-openpencil@latest
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

> OpenPencil 插件是公开的，无需 npm token。如果 DSH 预发布版本本身需要 registry 身份验证，请将该凭据保存在仓库检出目录之外的用户级或临时 npm 配置中。本仓库刻意不包含任何 registry 凭据。

## 设计工具

可选风格流程为 `openpencil_style_plan` → `openpencil_style_confirm` → `openpencil_pipeline_begin`，begin 保留原设计 brief，并传入 `style_selection`。目录浏览和精确选择无需模型推荐，推荐提供方默认关闭。该交接支持 script 引擎；app-agent 与电商风格交接暂不支持，普通 app-agent 生成入口保持可用。

| 工具 | 作用 |
| --- | --- |
| `openpencil_new` | 面向简单任务的兼容快速路径：运行一份事务性 QuickJS `batch_design` 脚本，以“仅在不存在时创建”的语义发布并返回可编辑呈现。生产级设计应优先使用下方完整管线。 |
| `openpencil_pipeline_begin` | 启动仅归当前会话所有的私有草稿，在内部创建唯一 root，返回 `rootNodeId`、`continuationStyle` 与紧凑的运行时匹配构建契约，并立即在侧边栏打开同一个 live canvas；目标文件仍未发布。 |
| `openpencil_pipeline_context` | 只在 begin 契约确实缺少某个指定 guideline、style、theme 或 UI kit 时加载一次定向上下文；不是启动时的固定步骤。 |
| `openpencil_pipeline_batch` | 最多运行两段直接原生 `I(...)`/`K(...)` generation script：第一段严格执行 begin `next` 返回的有界首个可见视口脚本，第二段一次补齐全部剩余区域。展示第二次预览前会执行 authored-structure 门禁；空分类/商品 helper 会被拒绝并原子恢复第一批快照，只允许重发一次修正后的第二批，且不消耗两段式预算。每次成功事务只会尝试展示给用户的精确 PNG，随后必须不加叙述地立即执行返回的 `next`。第三段 generation script 会被拒；只有 finish 返回的完整修复门禁可以授权一段有界 `U(...)` QuickJS 修复脚本。 |
| `openpencil_pipeline_inspect` | 仅在用户明确要求时提供手动诊断；普通生成不会把它当作预览或模型看图步骤。 |
| `openpencil_pipeline_finish` | 健康状态下一次调用就完成最终化、验证、最终 root PNG 自动预览与 `createIfAbsent` 原子发布。只有结果同时为 `needs_correction`、`canContinue: true`、含完整非空 `repairTargets` 且 `omitted: 0` 时，才授权一次纯 `U(...)` 修复和最后一次 finish；其他未发布结果一律终止。 |
| `openpencil_pipeline_abort` | 丢弃未发布草稿，不创建目标文件。 |
| `openpencil_create` | 在现有的活动画布上应用事务性 `batch_design` 程序来生成或重构节点。 |
| `openpencil_edit` | 修改显式指定的节点或用户选中的单个节点。 |
| `openpencil_render` | 创建不可变、内容寻址的 `.op` 快照，并渲染活动页面上的每个顶层帧 —— 可选 `scale` 与 `editable` 参数。 |
| `openpencil_selection` | 读取实时编辑器画布中当前选中的确切节点。 |

## 智能体设计工作流
### 设计回合的模型档位

生成质量与驱动流水线回合的模型直接相关。两批次契约在快档模型上可以工作,
但排版判断、文案质量与契约遵从度在更强或开启推理的模型上有显著提升。
以设计为主的 profile 建议在 `agent-default-model` 中使用非 flash 档
(例如 `deepseek-v4` 并开启 `reasoningEffort`,替代 `deepseek-v4-flash`);
仅聊天为主、设计只是附带能力的 profile 可保留快档。


桌面电商 Hero 的几何结构会在第二批前验证：右侧每个视觉子节点和叠加形状都必须留在 Hero 的固定内宽高中。超大视觉会连同第一批原子回滚，不再撑大 live canvas 或流入最终发布。Generation receipt 只暴露已提交的节点映射与预览；原生诊断延后到 finish 聚合，由它一次给出精确修复事务，不再诱发生成中途的猜测循环。

普通一句话需求直接调用 `openpencil_pipeline_begin`，不再花一个模型步骤加载可选 skill，然后走固定短路径：严格执行 begin `next` 给出的有界第一段直接 JS → 第二段也是最后一段直接 JS，一次补齐全部剩余区域 → 一次 `openpencil_pipeline_finish`。当前第一批只渲染**首个可见视口**：按 brief 构建完整 navigation/header 与精修 primary hero/content，限制 **不超过 32 次 `I`/`K` 调用、脚本不超过 8 KiB**。这个预算保留真实结构与视觉层级，同时仍避免重新落入无界生成循环；次要 cards、detail 与 `below-fold` 内容延后到第二批。电商 begin 会在绘制前选定与 App 对齐的 `ecommerce-modern-light` 方案：白色基底、暖色区段节奏、克制的橙色操作色、1120px 居中内容、56px Hero 大标题，并让可见文案跟随用户需求语言（中文需求保持中文文案，只允许可选的简短 ASCII 品牌名）。桌面电商 Hero 是带 `padding:[64,160]` 的全宽水平 frame，内层严格由 512px 文案、64px 间距和 448px 商品视觉组成；禁止再把 Hero 固定为 1120px 后叠加这组 padding。标题和副标题均使用 `width:"fill_container"`。通用电商把主视觉直接挂在 Hero 下：`I(hero,{type:"image",name:"Hero product image",width:448,height:360,imageSearchQuery:"gray loveseat isolated photo"})`。`imageSearchQuery` 是节点的直接字段，禁止写成 `image:{...}`，禁止再套 wrapper、和装饰形状混用，或被任一商品卡复用；第一批提交后它会立即富化。只有用户明确要求插画或不使用照片时，才允许改用显式 `layout:"none"` 的 4–6 层定位构图，其中至少包含 ellipse/path。纯粹堆叠圆角矩形、空白右栏、溢出或过小装饰块都会在第二批前被门禁原子回滚。平台和 viewport 只从最近一条真实用户请求中确定；模型扩写的 `brief` 不能把未指定的平台擅自改成移动端。用户未指定品牌时，直接根据 brief 使用中性店铺名或简短占位，不花时间比较命名方案。未提供 logo asset 时只使用纯文本品牌；禁止发明字母徽章，也不要给 text node 添加背景、固定高度或 effects。begin 会创建唯一 root 并打开私有 live canvas；每次 begin 或 batch 成功后，都必须不加叙述、规划、比较、检查或无关工具调用地立即执行下一步。健康 finish 会先执行原生定稿，再富化定稿后真实存在的图片槽，并在同一次调用中渲染最终用户预览与发布。

与 App 对齐的电商预设会把 64px Header 固定为 `padding:[0,160]`：Header 使用 `navbar`，Nav 是由 44px `nav-link` 项组成的 `nav-links` 集合，Header actions 则是由 44×44 `icon-button` frame 构成的 `toolbar`，不再直接放裸图标。160×48 主 CTA 使用 `button`、把 label 放进自己的 binding，并采用符合 AA 的 `#C2410C`/`#FFFFFF`。三张商品卡保持同一个连贯系列。通用家居需求固定使用运行时 4/4 实测通过的组合：`gray armchair isolated photo`、`artemide tolomeo lamp photo` 和 `potted plant isolated photo`；第三张卡必须写成盆栽，不能继续标成花瓶。

begin 返回的 `rootNodeId`、`continuationStyle`、`canvas` 与 `buildContract` 就是权威运行契约。除非用户明确指定文件名，否则省略 `path`，由插件生成具体且防冲突的 `.op` 文件名，避免把模板语法写进路径后再 begin/abort 重试。直接原样实现 `next` 指定的有界第一段脚本，不扩大或重新解释范围，也不插入面向用户的 reasoning；自动预览返回后，立即用第二段也是最后一段脚本通过数组与循环补齐全部区域。这条直接的两段式路径让执行保持有界并及时返回。

两段 generation script 都使用直接 `I(...)`/`K(...)` QuickJS，以及 begin 返回的精确 `rootNodeId`、`continuationStyle` 与 `buildContract`。在 `run_code` 内用 `String.raw` tagged template 构造内嵌脚本，保证转义符和预期文本换行原样进入 QuickJS。必须使用下面这个多行 wrapper：

```js
const draftId = "<exact begin.draftId>";
const script = String.raw`...`;
const r = await tools.openpencil_pipeline_batch({ draftId, script });
return r;
```

先把精确的 `begin.draftId` 加引号写入独立 `draftId` 字符串，再声明 `script`。固定的工具参数对象只能包含 `draftId` 和 `script`；禁止追加 `canvasWidth`、其他字段或把 return 写进对象。只返回 `r`，不要 log、print 或 stringify。每一批 QuickJS 都是 fresh scope，本地 binding 不会跨批次存在。`I`/`K` 返回的是不透明 node-id 字符串，不是可修改的节点对象：binding 只能作为后续 `I`/`K` 的父级，禁止赋值 `binding.x`、`binding.y` 或任何成员。第二批禁止重建 Page、App Content、Header 或 Hero；优先把新建且已绑定的 section rail 直接挂到 begin 的 `rootNodeId`。如果第一批创建了共享 content wrapper，只能使用第一批返回 binding 的精确 nodeId 继续挂载，绝不能按同名再建一个 wrapper。

`rootNodeId` 本身就是页面 frame，顶层区域直接挂到它上面，禁止再创建 Page/root wrapper；只有 frame/group binding 可以作为父级，包含图标的圆形视觉必须使用 frame 加 `cornerRadius`，不能拿 ellipse 当父级。每个语义容器（Header、Nav、Search、Hero、Card、Section、Toolbar、Button 或 CTA）都必须接住 `I`/`K` 返回的 binding，并立即通过该 binding 加入全部预期可见 children；禁止留下空语义容器，也禁止把预期 children 放成它的 siblings。最短有效 Header 示例为 `const h=I(root,{type:"frame",name:"Header",layout:"horizontal"}); I(h,{type:"text",content:"Shop",fontFamily:"Inter, system-ui, sans-serif"});`。移动端只有全宽 chrome/full-bleed section 可以直接挂 root；裸 text、icon、小 control 与 section title 必须放进一个已绑定且左右 gutter 为 24px 的 rail/section。每个分类卡都要有真实的视觉 tile 和 label；不同商品卡不能复用同一个 glyph 冒充商品图，应使用匹配且不同的 icon/shape，找不到合适视觉时直接省略 media wrapper。移动端分类项中的视觉 tile 固定为 56×56 tile frame。桌面电商分类栏以 `justifyContent:"space_between"` 铺满 1120px 内容宽度；商品栏固定为三张等宽 `fill_container` 卡片、24px 间距，右侧不能留下大段空白。每个 Button/CTA frame 创建后，必须立即通过其返回 binding 插入可见 text/icon child，不能把 label/icon 放成 sibling。最小图标示例为 `{type:'icon_font',name:'Search icon',iconFontName:'search',width:20,height:20}`：`name` 是图层标签，真正选择 glyph 的是 `iconFontName`，且只能使用这组已验证的 Lucide 名称：`home`、`search`、`shopping-bag`、`shopping-cart`、`user`、`heart`、`star`、`plus`、`arrow-right`、`sparkles`、`sun`、`apple`、`snowflake`、`droplet`、`cookie`、`leaf`、`coffee`、`package`、`gift`、`baby`、`spray-can`、`lamp`、`sofa`、`armchair`、`shirt`；没有合适图标时用形状构图。非电商设计默认只使用一个带具体英文 `imageSearchQuery` 的 `type:"image"` leaf，除非用户明确要求多图；电商固定使用三张不同商品图。每个 query 不超过四个英文词，只描述一个具体商品，禁止 lifestyle、collection 或宽泛 category；通用家居严格使用上面的实测组合。带 `isolated` 的 query 必须有 isolated/cutout/white-background 等正向元数据，重试也不能退回普通房间图；多词 query 至少命中两个有意义的元数据 token。禁止在大型固定商品媒体框里只放一个很小的 icon。每个 Hero/Product/Art/Media frame 只能有一个主视觉：一张 image，或一个实质性的 composed-shape visual，禁止 image 与占位 icon 并存；image node 是叶节点，不能作为父级。每个生成文本节点都必须显式使用 `fontFamily: "Inter, system-ui, sans-serif"`，普通字号和行高分别为 `fontSize: 16`、`lineHeight: 1.5`。桌面 App 继续使用内置 Inter；Web 宿主不会内置 Inter，因此自动走通用字体回退且不会弹出缺失字体提示。禁止只写裸 `Inter`。CJK 的 `lineHeight < 1.3` 会提升到 `1.5`，数值型 text height 也会自动改为 `fit_content`，除非 `textGrowth` 明确为 `fixed-width-height`。仅标题或特殊排版显式覆盖字号和行高。其余当前节点、样式、脚本与修复规则仍以运行时契约为唯一来源。第三段 generation script 会被拒；只有下文所述的完整修复门禁，才能授权第三段也是最后一段有界 QuickJS 修复脚本，并且只对指定目标使用 `U(...)`。

分类横栏必须为每个 label 使用不同且语义匹配的 icon。每个 `<label> icon tile` card 内先创建嵌套的 56×56 `Category glyph surface` frame，把 `icon_font` 放进 surface，并把文字 label 直接放在 card 下；禁止使用包含 `art`、`media` 或 `image` 的 wrapper 名，避免 native finalization 把图标 surface 误判成图库图片槽。数码分类使用 `smartphone`/`camera`，食品分类使用 `utensils`/`sandwich`/`croissant`，不能用 `lamp` 或 `coffee` 错配。同一商品横栏必须先完成每张商品卡的 media（或明确省略）、名称和价格，再开始下一张卡；发布门会折叠空 media 壳，半截执行的脚本不会再发布空白商品区。第二批结构门禁会在预览与 finish 前回滚不完整分类/商品结构、过高分类或商品横栏、混合卡片宽度或紧凑画布溢出，并且最多只允许一次完整替换脚本。每张商品图的英文 query 不超过四个词，格式为 `<具体商品> isolated photo`，只描述一个商品，禁止 lifestyle、collection 或宽泛 category。子节点后处理会保留作者指定的横栏高度，不再套用文档根节点的高度扩展。规范的 post-final 图片富化不再被更早的 context 富化占用；零结果重试会保留末尾的具体商品主体，照片查询会拒绝 illustration、drawing、engraving、painting、catalog 等明显非照片元数据，而 isolated 查询在重试后仍必须保留独立主体证据；只要任一请求的商品图最终仍未解析，发布就会停止，避免交付空图或不对称商品卡。移动端 finalization 会在 clipped rail 中保留全部分类节点，桌面分类横栏也不会再被移动端规则改写。

用户未指定品牌时，自造店名必须使用简短 ASCII；用户要求的 Unicode 文本应直接写字符，禁止手工拼 JSON/JS escape 序列。`text_input` 是叶节点，只能使用自身 placeholder；需要“图标 + 提示文案”时，必须创建命名 Search frame，并把 icon 与 text 挂到该 wrapper。

live canvas 侧边栏会在 begin 时立即打开，并持续绑定私有草稿；标题栏现在提供明确的“关闭”按钮，dirty 草稿仍保留原有保存/确认保护。两次成功 batch 都会尝试附带给用户的精确 PNG 预览卡片。每次桌面电商 batch 提交成功后，只要已提交文档仍有未解析 query，宿主都会先做一次最长 8 秒的 best-effort 图片富化，再生成 live preview：第一批即可出现真实 Hero 商品图，第二批再填充商品栏，而不是全部图片只能等到 finish。如果 `next` 返回 `previewUnavailable`，脚本已经提交到 live canvas：继续执行 `next`，禁止重跑 batch，也不要调用 `openpencil_pipeline_inspect` 或 `read_image`。inspect 只用于用户明确要求的诊断。

完成阶段仅在权威文档存在非空 `imageSearchQuery` 时自动执行一次图片丰富化，无需额外 context 步骤，icon/shape art 绝不会触发；最多三路图库搜索在同一个不变的 20 秒总期限内并发执行，写回仍按文档顺序确定，因此三商品卡不会再被第一张图片的 provider ladder 饿死。随后执行 OpenPencil 原生最终化、lint、布局检查与 DSH 质量门禁，并在同一次健康 finish 中自动渲染最终 root PNG、原子发布。在同一个 `run_code` 内保留 finish 返回对象。只有结果同时满足 `stage: "needs_correction"`、`canContinue: true`、含完整非空 `repairTargets`、`checks.dsh.repairTargetSummary.omitted: 0`，并且每个 target 都有 `operation: "U"`、精确非空 `nodeId` 与非空 `patch` 时，才授权修复：一次性在唯一一段纯 `U(...)` batch 中应用所有目标，然后不加叙述地只再调用一次 finish。经过这次修复后，任何不是 `published: true` 的结果都必须终止；抛错、`canContinue: false` 或其他未发布 finish 结果同样必须只报告一次并终止，禁止重试、inspect、读取 image/context、abort 或另起草稿。`lint_document` 的 Info 与以下五类 Warning 不阻断：`invisible-container`、`mixed-sibling-padding`、`sibling-inconsistency`、`text-effect`、`text-explicit-height`。其他 Warning（包括 `widget-a11y`、`excessive-frame-effects`、`empty-path`）仍会阻断，所有 Error 也会阻断。script mode 无法使用 `G(...)` 时，`imageSlots` 仍只是观察性提示。原生 hard quality、contrast、layout 诊断与 DSH hard gates 也继续阻断。若保留对象为 `published: true`，其中已经包含精确最终 PNG 与 live editor，应立即向用户收尾，禁止再调用 `openpencil_render`、`read_image` 或 `openpencil_pipeline_inspect`。失败门禁或 `openpencil_pipeline_abort` 不会创建目标文件。文档级可编辑授权只在侧边栏空闲时自动打开，并始终保留 **编辑画布** 供显式切换；嵌套结果也能水合精确 PNG，不再显示“没有可用预览通道”。

在同一持续运行的 DSH 服务内，切换浏览器或重载后，经过严格解析的 `openpencil_new` 或 `openpencil_pipeline_finish` 持久化 publication 可以恢复为精确 PNG 与明确的 **编辑画布** 操作。历史卡片绝不会自动打开侧边栏，必须由用户点击该操作。普通历史 `openpencil_render` 始终保持只读，非 loopback 连接也绝不会获得编辑器授权。

随包提供的 `openpencil-design` skill 仍是可选的轻量参考；普通创建无需先加载它，完整的紧凑契约由 begin 直接返回。托管运行时也不依赖桌面版二进制。`openpencil_new` 继续作为兼容的单批次快速路径，但生产级设计生成应优先使用完整管线。

仅在已有的活动画布上使用 `openpencil_create` 与 `openpencil_edit`。在编辑器执行保存（Save）操作之前，它们的编辑都保持未保存状态。

## Web 查看器资源

DSH 仅为客户端插件提供 `client.js`，因此 OpenPencil ESM SDK、其 WASM 以及 CanvasKit 被放置为显式的同源资源：

```sh
pnpm run sync:viewer-assets
```

同步命令默认使用当前 OpenPencil 单仓根目录（`../..`），也可回退到同级目录下的 `../openpencil` 检出。可通过 `OPENPENCIL_ROOT` 或 `--openpencil-root` 覆盖。可通过 `DSHPLUGIN_OPENPENCIL_VIEWER_SOURCE` 选择完整的预构建资源目录。可通过 `DSHPLUGIN_OPENPENCIL_VIEWER_ASSET_DIR` 覆盖运行时查找。

查看器资源仅在用户打开画布后才按需加载。如果这些资源缺失或无效，PNG 预览仍然可用，并且不会展示画布按钮。

## 托管编辑器

可编辑会话使用 OpenPencil 的托管 Web 宿主 —— 与 `op-vscode` 相同的架构。插件仅在经过授权的用户操作之后启动该宿主，将守护进程令牌保存在内存中，校验 iframe 来源与源站，并在编辑器会话结束时关闭进程。编辑器界面采用渐进式选择：若宿主声明该原生接缝，则使用原生 Tool 详情；否则使用插件带调整大小与全屏控件的右侧工作台。

若 DSH 桌面代理省略 `Origin`，插件仅在 DSH 验证请求身份、且本地连接与监听地址检查通过后接受请求。显式浏览器来源仍须通过同源检查；缺少宿主认证服务时，没有 `Origin` 的请求仍会被拒绝。

编辑器面板已在官方 DSH 桌面版 `0.2.1-rc.3` 上完成验证：打开画布、修改图层名、保存并回读文件、重新打开编辑器，以及重启 DSH 后打开新卡片。本次实机验证范围为编辑器面板流程。

启动过程使用可安全应对慢速挂载的 listening handshake：只有在随包宿主报告已绑定地址后才开始就绪探测。无需安装桌面版 OpenPencil。

托管编辑器的设置与用户资源保存在 `DSH_HOME` 下的私有目录，并按固定的运行时版本和源码修订隔离；不会读取或改写独立 OpenPencil 应用的设置文件。

发布版提供六个原生平台包目标：`darwin-arm64`、`darwin-x64`、`linux-arm64`、`linux-x64`、`win32-arm64` 与 `win32-x64`；两个 Linux 包均以 glibc 为目标。根包将所有平台包声明为精确版本的 `optionalDependencies`，由 npm 根据 OS 与 CPU 选择匹配的包。每个平台包都将 `op-host-web-server`、编辑器 Web 打包产物与 CanvasKit 作为一套相互匹配的原子运行时随包提供。新包采用守护进程原生支持的部署布局：可执行文件位于 `bin/`，wasm-bindgen bundle 位于 `bin/web-bundle/`，CanvasKit 位于 `bin/web-bundle/canvaskit/`。发布 smoke 会显式移除两个资源发现环境变量后直接启动可执行文件，确保包本身即可启动。因此，托管编辑器默认不依赖 `/Applications/OpenPencil.app`、`PATH` 中的 `openpencil-desktop`，也不依赖 OpenPencil 源码检出。

如果画布处于未保存状态时 DSH 重新加载或卸载插件，宿主会保留一份不透明的本地恢复草稿，最长七天。重新打开同一来源时，会先询问是否将其恢复到活动画布中；在用户明确保存之前，恢复过程绝不会覆盖 `.op` 文件。

正式六平台包会在受保护的 release 构建中注入并校验中国区/全球区协作 bootstrap 端点，通过校验后才发布。未注入该配置的本地自建版本，可在启动 DSH 前使用 `OPENPENCIL_COLLAB_BOOTSTRAP_URL=https://<your-host>/api/v1/collaboration/bootstrap` 覆盖；该值必须使用 `https`，且路径必须严格为 `/api/v1/collaboration/bootstrap`。

跨设备画布同步要求 PC/DSH 原生运行时与移动端 App 都更新到包含当前协作队列修复的同一 OpenPencil 发布线。旧移动端与新 PC 运行时混用时，仍可能只看到远端光标而收不到画布提交。

在本仓库中开发时，启动 DSH 前须依次构建编辑器 Web bundle、构建原生宿主，再暂存这套相互匹配的运行时。

`pnpm run build:editor-web` 会运行 OpenPencil 官方支持的 WASM bundle gate。它要求 Bash、带 `wasm32-unknown-unknown` target 的 Cargo/Rust、`wasm-bindgen` CLI、Binaryen 的 `wasm-opt`、Node.js 与 `gzip`；CanvasKit 不需要 EMSDK。Web 构建不使用协作 bootstrap 构建变量。运行 `pnpm run build:editor-runtime` 前必须同时设置 `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN` 与 `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL`；它们仅用于原生 Cargo 构建，缺少任一变量都会 fail closed。两项构建都成功后，再执行最后一条命令暂存运行时。

```sh
pnpm run build:editor-web
pnpm run build:editor-runtime
pnpm run stage:editor-runtime
```

如果本地没有受保护的 release bootstrap 地址，可直接构建原生宿主并在 Web bundle 完成后暂存，用于非生产 smoke。这样会验证精确预览、MCP 与托管编辑器路径；协作仍需要显式的运行时 bootstrap 覆盖或正式发布包。

```sh
cargo build --manifest-path ../../Cargo.toml --locked --release -p op-host-web-server
pnpm run stage:editor-runtime
```

若要显式覆盖运行时，以下三项必须作为完整且相互匹配的一套同时提供：

- `DSHPLUGIN_OPENPENCIL_EDITOR_BINARY` 用于 `op-host-web-server`；
- `DSHPLUGIN_OPENPENCIL_EDITOR_WEB_BUNDLE_DIR` 用于已构建的编辑器 Web 打包产物；
- `DSHPLUGIN_OPENPENCIL_EDITOR_CANVASKIT_DIR` 用于 CanvasKit 资源。

只提供其中一部分属于无效配置；插件不会将自定义路径与随包运行时资源拼接使用。

较早的 `0.1.0-rc.5` 平台包采用 `web/pkg` 加 `web/canvaskit` 的旧布局。若绕过插件直接启动该版本守护进程，必须同时传入两个原生资源变量：

```sh
OPENPENCIL_WEB_BUNDLE_DIR="<runtime-root>/web/pkg" \
OPENPENCIL_CANVASKIT_DIR="<runtime-root>/web/canvaskit" \
"<runtime-root>/bin/op-host-web-server" --serve-web
```

保存采用乐观源哈希、原子替换以及后继能力凭据。如果来源在编辑器之外发生了变化，插件会报告冲突而不是覆盖它。

## 结果元数据

模型可见的结果保持为纯 JSON。仅浏览器可见的 `presentationMeta.$dshOpenPencil` 携带附加的授权，包括：

- `image`：PNG 路径、预览/下载 URL 以及真实宽高；
- `frames`：按活动页面顺序排列的每个精确渲染的顶层帧，包括其节点 id/名称/索引以及签名的 PNG URL；
- `document`：源操作路径以及不可变快照 URL、字节数与 SHA-256；
- `viewer`：在资源路由挂载时提供带版本号的 SDK/WASM/CanvasKit URL；
- `editor`：在 `editable: true` 获得授权时提供作用域限定的启动/刷新能力。

结果还会记录 `renderer`、`rendererBinary`、`fidelity` 以及任何警告。现有的仅 PNG 的 schema-v1 消息仍然可以渲染。

DSH `0.1.1-rc.2` 不会为嵌套在 PTC/Code Mode 下的工具持久化浏览器展示元数据。插件会通过同源、会话绑定的端点恢复该仅 UI（UI-only）投影：浏览器仅发送 session id、call id 以及不可变文档的 SHA-256，而宿主则从持久的 DSH 会话日志中解析权威结果，并仅使用一个短暂存活的进程内标记（in-process marker）来授权近期的实时编辑。签名的预览/编辑器能力永远不会进入规范的工具结果或模型上下文。普通 `openpencil_render` 的持久历史始终只读。经过严格解析的 `openpencil_new` 或 `openpencil_pipeline_finish` 持久化 publication，只有在 loopback 连接且用户显式点击后才可能获得编辑器授权；自动打开侧边栏仅保留给最近、可信的实时结果。

为了限制回放范围，嵌套元数据恢复最多接受 128 个顶层帧；更大的 Code Mode 结果仍可通过其规范 JSON 回退获得。

## 当前限制

- 对现有画布的后续编辑需要一个已打开的托管编辑器。在用户调用其保存（Save）操作之前，更改保持未保存状态。
- 轻量级 Web SDK 画布是只读的；完整编辑使用独立的托管编辑器界面。在 DSH `0.1.1-rc.2` 上，插件使用带全屏选项的可调整大小右侧工作台。
- 精确图库涵盖活动页面上的顶层帧；交互式画布仍是检查非活动页面与嵌套节点的方式。
- 渲染与快照缓存仍需要产品级的保留策略。

## 项目结构

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

## 构建与验证

```sh
pnpm run sync:viewer-assets
pnpm run build
pnpm run test:viewer-assets
pnpm run test:client
pnpm run test:host /absolute/path/to/design.op 375 1091
```

构建需要 Node 24.11 或更高版本以及 pnpm。DSH host/client 包是目标 DSH profile 提供的对等依赖（peer dependencies）。构建工具从本地开发依赖、当前链接的 DSH 检出或已安装的 DSH 源码包中解析；`DSH_SOURCE_ROOT` 可以显式指定源码检出。当该环境单独配置时，lockfile 会锁定独立的公共构建工具。

对于私有 DSH 预发布版本，请将签发的 npm 凭据保存在本仓库之外（例如用户级或临时的 `.npmrc` 中），并直接运行所需版本：

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

切勿提交 `.npmrc`、`NPM_TOKEN` 或复制的 registry 凭据。本仓库默认忽略本地 npm 配置。

`test:host` 会执行一次真实的精确渲染，校验 PNG IHDR 几何信息与 SHA-256，通过 HTTP 验证不可变图像/文档能力，启动已暂存的托管编辑器，推送实时选择，执行一次 MCP 修改，保存，并验证重新打开的是最新字节。预期尺寸随测试夹具而异。

## 生态系统

DSH OpenPencil 是 **[OpenPencil](https://github.com/ZSeven-W/openpencil)** 的 DeepSeek Harness 插件 —— 全球首款开源、AI 原生的矢量设计工具 —— 也是 **[ZSeven-W](https://github.com/ZSeven-W)** 纯 Rust、AI 原生工具家族的一员。

| 项目 | 简介 |
| ------- | ---------- |
| **[OpenPencil](https://github.com/ZSeven-W/openpencil)** | 本插件所驱动的设计工具 —— 提示词到画布的生成、并发智能体团队、以代码为设计的 `.op` 文件，以及内置的 MCP 服务器。本文中的精确预览、交互式画布与托管编辑器均由 OpenPencil 本身提供支持。 |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | 用于交付 LLM 智能体的纯 Rust 异步运行时 —— 多提供商、端到端工具能力、结构化权限、真正的 MCP、零 `unsafe`。为 OpenPencil 的内置智能体运行时提供动力。 |
| **[jian](https://github.com/ZSeven-W/jian)** | 纯 Rust、GPU-Skia UI 框架 —— 小部件、布局、事件与热重载集成于同一技术栈。OpenPencil 的 UI 框架，也是本插件回退渲染器的来源。 |
| **[Zode](https://github.com/ZSeven-W/zode)** | 面向终端、开源、AI 原生的编程助手 —— 阅读你的代码、运行命令，并通过 MCP 驱动 OpenPencil。 |
| **[noema](https://github.com/ZSeven-W/noema)** | 面向编码智能体的本地优先、非向量记忆系统 —— 以可检查文件形式提供持久记忆，可跨运行时工作。 |
| **[openpencil-skill](https://github.com/ZSeven-W/openpencil-skill)** | 教会 AI 智能体如何使用 `op` 进行设计的 LLM skill 插件 —— 本 DSH 插件的配套项目。 |

同系列的 DSH 插件：

- [DSH Android](https://github.com/ZSeven-W/dsh-android) —— 在对话中运行 Android 模拟器或 USB 真机，全部由 adb 驱动
- [DSH Crew](https://github.com/ZSeven-W/dsh-crew) —— 从 Claude Code / Codex 把任务派给 DSH agent
- [DSH iOS](https://github.com/ZSeven-W/dsh-ios) —— 在对话中运行 iOS 模拟器与 USB 连接的真机
- [DSH Noema](https://github.com/ZSeven-W/dsh-noema) —— DSH 的长期记忆

## 参与贡献

欢迎贡献！Fork 并克隆仓库，创建分支，运行 `pnpm run build` 与测试套件，使用 [Conventional Commits](https://www.conventionalcommits.org/) 规范提交，并向 `main` 分支发起 PR。

## 社区

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="https://raw.githubusercontent.com/ZSeven-W/openpencil/main/screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> 加入我们的 Discord</strong>
</a>
—— 提出问题、分享设计、建议功能。

**社区认可：[LINUX DO](https://linux.do/)**

## 许可证

[MIT](./LICENSE) —— 版权所有（c）2026 ZSeven-W

第三方组件列于 [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)。
