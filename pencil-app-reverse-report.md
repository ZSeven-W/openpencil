# Pencil.app 静态逆向分析报告

分析对象：`/Applications/Pencil.app`  
分析时间：2026-06-23  
分析方式：解包 Electron `app.asar`、读取未压缩 JS、查询本地 MCP server 的 `tools/list` 元数据、抽取 `.pen` 模板/设计库、检查前端 bundle 和 schema。未做签名绕过、运行时注入或二进制反汇编。

## 1. 应用概况

`Pencil.app` 是一个 Electron 桌面应用。

- Bundle ID：`dev.pencil.desktop`
- 版本：`1.1.34`
- Copyright：`High Agency, Inc.`
- 主包：`/Applications/Pencil.app/Contents/Resources/app.asar`
- 解包目录：`/Applications/Pencil.app/Contents/Resources/app.asar.unpacked`
- 更新源：GitHub `highagency/pencil-desktop-releases`
- 自定义协议：`pencil://`
- 默认配置目录：`~/.pencil`

主要内置内容：

- Electron 主进程代码：`out/main.js`、`out/app.js`、`out/desktop-resource-device.js`
- 前端编辑器：`out/editor/index.html`、`out/editor/assets/index.js`、`index.css`
- CanvasKit/WASM 渲染器：`out/editor/assets/pencil.wasm`
- 本地 MCP server：`out/mcp-server-darwin-arm64`
- Agent SDK：`@anthropic-ai/claude-agent-sdk`、`@openai/codex-sdk`
- 内部包：`@ha/agent`、`@ha/ipc`、`@ha/mcp`、`@ha/schema`、`@ha/ws`
- 内嵌 `.pen` 设计库和模板：`halo`、`lunaris`、`nitro`、`shadcn`、welcome/demo/new 文件

## 2. Tool 清单

这里的 tool 指 Pencil MCP server 暴露给 Claude/Codex/外部 MCP client 的工具。通过直接启动本地 `mcp-server-darwin-arm64 --app desktop --enable_spawn_agents` 并调用 `tools/list` 确认。

### 2.1 MCP Tools

| Tool | 作用 |
|---|---|
| `get_editor_state` | 获取当前激活编辑器、选区、顶层节点、组件等设计上下文 |
| `open_document` | 在无活动编辑器时新建文档，或打开指定 `.pen` 文件 |
| `get_guidelines` | 获取某个主题的设计/实现规则 |
| `get_style_guide_tags` | 获取可用于检索 style guide 的 tag 集合 |
| `get_style_guide` | 按 tags 或 name 从服务端获取 style guide |
| `batch_get` | 批量读取节点，或按 pattern 搜索节点/组件 |
| `batch_design` | 执行批量设计操作 |
| `snapshot_layout` | 返回节点树的计算布局矩形，用于判断层级和空间 |
| `get_screenshot` | 截取某个节点的渲染图，用于视觉验证 |
| `get_variables` | 读取 `.pen` 里的 variables/themes |
| `set_variables` | 添加或更新 variables/themes |
| `find_empty_space_on_canvas` | 按方向和尺寸寻找画布空白区域 |
| `search_all_unique_properties` | 在节点子树内收集唯一属性值 |
| `replace_all_matching_properties` | 在节点子树内批量替换匹配属性 |
| `export_nodes` | 导出节点为 PNG/JPEG/WEBP/PDF |
| `spawn_agents` | 拆分大设计任务，启动多个 designer agents 并行工作 |

`spawn_agents` 只有在 MCP server 以 `--enable_spawn_agents` 启动时可用。主进程里是否启用由 prompt 请求里的 `agentMultiplier > 1` 决定。

### 2.2 `batch_design` 操作 DSL

`batch_design` 不是直接传 JSON 节点树，而是传一段小型操作脚本，每行一个操作：

- `I(parent, nodeData)`：Insert
- `C(nodeId, parent, overrides)`：Copy
- `R(path, nodeData)`：Replace
- `U(path, updateData)`：Update
- `D(nodeId)`：Delete
- `M(nodeId, parent, index)`：Move
- `G(nodeId, "ai" | "stock", prompt)`：生成/搜索图片并作为 fill 应用到 frame/rectangle

规则里要求每次 `batch_design` 最多约 25 个操作；大屏幕/多 section 设计要按逻辑分批。

### 2.3 桌面菜单/产品功能

桌面菜单暴露了这些功能：

- 新建、打开、保存、另存为 `.pen`
- 导入 PNG/JPG/SVG/Figma `.fig`
- Figma 导入说明
- Export Design to Code Guide
- 安装 `pencil` CLI 到 `~/.local/bin`
- 显示/隐藏 UI
- 缩放、全屏、明暗主题切换
- 多窗口 grid 排列
- 设置、更新检查、文档/Prompt Gallery/扩展/Discord 链接

## 3. Styleguide 与 Design Kits

需要区分两类东西：

1. 本地内嵌的 `.pen` design kits/component libraries。
2. 远程按 tags 获取的 style guide。

### 3.1 本地内嵌 Design Kits

本地包里有四套主要设计库：

| Kit | 文件 | reusable 组件数 | 主题/变量 |
|---|---|---:|---|
| Halo | `halo.lib.pen` / `pencil-halo.pen` | 95 | 45 variables，Mode: Light/Dark |
| Lunaris | `lunaris.lib.pen` / `pencil-lunaris.pen` | 100 | 41 variables，Mode: Light/Dark |
| Nitro | `nitro.lib.pen` / `pencil-nitro.pen` | 97 | 42 variables，Mode: Light/Dark |
| shadcn | `shadcn.lib.pen` / `pencil-shadcn.pen` | 87 | 28 variables，Mode/Base/Accent 多轴主题 |

常见 reusable 组件包括：

- Sidebar
- Button variants
- Avatar
- Progress
- Tooltip
- Switch
- Textarea/Input/Select groups
- Dropdown
- Accordion
- Alert
- Pagination
- Icon Label
- Radio Description

shadcn 的主题轴更完整：

- `Mode`: Light, Dark
- `Base`: Neutral, Gray, Stone, Zinc, Slate
- `Accent`: Default, Red, Rose, Orange, Green, Blue, Yellow, Violet

### 3.2 远程 Style Guide 机制

`get_style_guide_tags` 本地返回 tag 词表；`get_style_guide` 会请求 `https://api.pencil.dev/public/style-guide`。也就是说，完整 styleguide 内容不是本地内嵌在 app 里的，而是通过 Pencil API 按 tags/name 获取。

可用 tags 包括这些类型：

- 产品/界面：`webapp`、`mobile`、`dashboard`、`data-dashboard`、`fintech`、`enterprise`、`developer`、`devtools`
- 气质：`premium`、`luxury`、`sophisticated`、`calm`、`playful`、`professional`、`high-end`
- 风格：`brutalist`、`bauhaus`、`swiss`、`editorial`、`industrial`、`scandinavian`、`japanese`
- 色彩/主题：`dark-mode`、`light-mode`、`monochrome`、`gradient`、`purple`、`gold-accent`、`blue-accent`、`orange-accent`、`green-accent`
- 形态/布局：`bento-grid`、`sidebar`、`icon-sidebar`、`floating-nav`、`rounded`、`sharp-corners`、`soft-corners`
- 字体：`serif`、`serif-display`、`monospace`、`condensed-type`、`bold-typography`、`typographic`

测试 tags `mobile + dark-mode + fintech + premium + blue-accent` 时，服务端返回的 styleguide 名称示例是：

- `webapp-03-darkclassy_light`

结论：Pencil 的“哪些 styleguide”不是固定本地枚举，而是 tag 检索 + 远程 styleguide 内容服务。本地能确认的是 tag 体系和调用机制。

## 4. 设计数据模型

Pencil 的 `.pen` schema 支持这些节点类型：

- `frame`
- `group`
- `rectangle`
- `ellipse`
- `line`
- `polygon`
- `path`
- `text`
- `icon_font`
- `ref`
- `note`
- `prompt`
- `context`
- `connection`

核心设计能力：

- flex-like layout：`layout: horizontal | vertical | none`
- `gap`、`padding`、`justifyContent`、`alignItems`
- 动态尺寸：`fit_content(...)`、`fill_container`
- variables：`$variable` 字符串引用
- themes：多主题轴
- reusable components：节点设置 `reusable: true`
- instances：`type: "ref"` + `ref: componentId`
- instance override：通过 path/descendants 覆盖组件内部子节点
- fills：color、gradient、image、mesh_gradient
- effects：blur、background_blur、shadow
- icons：`icon_font`，支持 lucide、feather、Material Symbols、phosphor 等字体图标

## 5. 设计编排流程

### 5.1 桌面进程编排

启动后主进程做这些事：

1. 注册 `pencil://` 协议，用来加载生产版 editor。
2. 创建 `WebSocketServerManager`。
3. 创建 `DesktopMCPAdapter`，负责把 Pencil MCP server 注册到外部集成里。
4. 创建 `IPCDeviceManager`，管理多个打开的 `.pen` 文档窗口。
5. 启动 WebSocket server。
6. 调用 `proxyMcpToolCallRequests()`，把 MCP tool request 转到当前 editor。
7. 设置 Claude/Codex 状态检测。
8. 打开最近文件，或默认打开 `pencil-welcome-desktop.pen`。

支持的外部集成：

- `claudeCodeCLI`
- `codexCLI`
- `geminiCLI`
- `openCodeCLI`
- `kiroCLI`
- `claudeDesktop`

### 5.2 Agent 编排

用户在 editor 里发 prompt 后，链路如下：

```text
Renderer chat UI
  -> IPC send-prompt
  -> IPCDeviceManager.invokeAgent()
  -> 拼 system prompt
  -> 注入 Pencil MCP server
  -> 启动 ClaudeAgent 或 CodexAgent
  -> Agent 调 MCP tools
  -> MCP server 通过 WebSocket 转发到桌面进程
  -> IPC 转给当前 editor window
  -> editor 修改 document/scenegraph
  -> 结果回传给 Agent 和 UI
```

system prompt 的组成：

- `.pen` schema
- general design rules
- design workflow rules
- styleguide tags
- 当前 editor state
- 用户选中的 guideline attachments

Agent 类型：

- Claude：使用 `@anthropic-ai/claude-agent-sdk`，允许 `mcp__pencil`、`WebSearch`、`WebFetch`
- Codex：使用 `@openai/codex-sdk`，配置 `mcp_servers.pencil`，开启 web search，工作目录为当前文档 workspace

### 5.3 设计工作流

内嵌 design prompt 推荐的流程：

1. `get_editor_state`
2. 判断是否需要视觉创意方向
3. 创意任务：`get_style_guide_tags` -> `get_style_guide`；组合任务：只拿 `design-system` guidelines
4. `get_variables`
5. `batch_get` 读取组件结构
6. `snapshot_layout` 查看布局
7. `batch_design` 分批生成/修改
8. `get_screenshot` 视觉验证
9. 继续迭代，直到截图检查通过

重要规则：

- 尽量用 design system 里的 reusable components。
- 新建/复制/修改 screen 时使用 `placeholder: true` 标记工作区，完成后移除。
- 组件实例内部修改用 `instanceId/childId` 路径。
- Copy 后不要用旧 descendant id 再 Update，因为复制会生成新 id。
- 图片不是 node 类型，而是 frame/rectangle 的 image fill；必须用 `G()` 操作生成或搜索。
- table 必须是 `Table frame -> Row frame -> Cell frame -> Cell content` 层级。

### 5.4 并行设计

大任务会通过 `spawn_agents` 拆分：

- 主 agent 先创建多个容器/placeholder。
- `spawn_agents` 接收多个子任务，每个子任务给出 prompt、container node、guideline/styleguide。
- 子 agent 不能自行搜索 styleguide，所以主 agent 要把 styleguide tags/name 传进去。
- 主 agent 保留最后一个部分自己完成。
- 建议最多 8-10 个 designer agents。

## 6. 动画与视觉效果

### 6.1 `.pen` 文档本身没有动画模型

schema 中没有发现 `animation`、`keyframe`、`timeline`、`transition` 这类可保存的设计动画字段。`.pen` 主要是静态矢量/布局模型。

支持的视觉效果是静态的：

- `blur`
- `background_blur`
- `shadow`
- opacity
- rotation
- gradient / mesh_gradient / image fill

所以 Pencil 目前更像 Figma/Sketch 风格的静态设计工具，而不是带 prototype timeline 的动效工具。

### 6.2 UI 层动画

前端 CSS 中存在 Tailwind/Radix 类动画：

- `spin`
- `ping`
- `pulse`
- `enter`
- `exit`
- `accordion-down`
- `accordion-up`

实际用途包括：

- loading spinner
- 在线/状态 pulse dot
- dialog/dropdown enter/exit
- accordion 展开/收起
- hover/active transition

部分 dialog 明确用了 `!animate-none !duration-0`，说明某些弹窗刻意禁用了默认动画。

### 6.3 画布刷新机制

画布层通过 CanvasKit/Skia WASM 渲染：

- 前端创建 canvas
- 初始化 Pencil editor engine
- 加载 `assets/pencil.wasm`
- 使用 Pixi manager + CanvasKit renderer
- resize 用 `ResizeObserver`
- 文档/theme 变化后调用 scene manager 的 `requestFrame()` 重绘

这里的“动起来”主要来自渲染循环和交互刷新，而不是文档级动画。

### 6.4 Agent 流式生成的“动画感”

ClaudeAgent 对 `mcp__pencil__batch_design` 做了流式解析：

- 监听 Claude stream 的 `input_json_delta`
- 用 `jsonrepair` 尝试补全 partial JSON
- 一旦解析出新的 operation，就立刻 emit `batch-design`
- editor 逐步执行新增操作

因此用户看到的设计生成可能是逐步出现的。这不是传统动画，而是流式工具调用带来的 progressive rendering。

同样机制也用于 `spawn_agents` 的 partial config。

## 7. 其它值得注意的点

- App 内有 Sentry，上报启用，并且主进程注释显示 `sendDefaultPii: true`。
- renderer bundle 里也有 PostHog/Sentry 相关代码。
- session 存在 `~/.pencil/session-desktop.json`。
- legacy license token 文件：`~/.pencil/license-token.json`。
- backup 文件机制：物理 `.pen` 文件会生成节流备份，约 5 秒节流。
- `.pen` 文件可关联系统打开，扩展名是 `.pen`。
- 支持把普通 `.pen` 转为 `.lib.pen` library。
- 支持本地资源导入，临时文档资源放在 `~/.pencil/resources/<uuid>`。

## 8. 对 OpenPencil 可借鉴的实现点

如果目的是参考 Pencil.app 的产品设计，可以借鉴这些架构：

- MCP server 与 editor 分离：外部 LLM 只调 MCP，真实文档操作通过 WebSocket/IPC 回 editor。
- Prompt 内置严格设计工作流：schema + general rules + domain guidelines + styleguide tags。
- `batch_design` DSL：比逐个工具调用更适合复杂 UI 生成。
- 设计修改必须截图验证：`get_screenshot` 是流程中的闭环。
- component instance override 用 path 表达：适合设计系统复用。
- 大任务并行：先建容器，再用 subagents 分区生成。
- Styleguide 服务化：本地只存 tags，具体风格从服务端按 tags/name 拉取。

## 9. 结论

Pencil.app 的核心不是单纯画布编辑器，而是“静态矢量设计模型 + MCP 工具层 + Claude/Codex agent 编排 + 设计系统/远程 styleguide”的组合。

它的设计生成关键点是：

- 文档模型可被 MCP tools 精准读写。
- Agent 拿到 schema、规则、当前状态和 styleguide 后，以 `batch_design` DSL 批量修改。
- 每轮通过 layout snapshot 和 screenshot 做验证。
- 大任务可通过 `spawn_agents` 并行拆解。
- 动效主要是 UI 层 CSS 和渲染刷新；`.pen` 本身没有独立动画/prototype timeline。
