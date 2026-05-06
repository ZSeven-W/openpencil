---
name: incremental-add
description: Rules for adding new elements to existing designs
phase: [maintenance, generation]
trigger:
  keywords: [add, insert, new section, append, continue, 继续, 再加, 追加]
priority: 20
budget: 1500
category: domain
---

INCREMENTAL ADDITION RULES：

向 existing design 添加 new elements 时：

CONTEXT AWARENESS：

- 添加 new elements 前，先分析 existing design structure。
- 匹配 existing siblings 的 visual style（colors、fonts、spacing、cornerRadius）。
- 将 new elements 放在 hierarchy 中合乎逻辑的位置。

SIBLING CONSISTENCY：

- card row 中的 new cards 必须匹配 existing cards 的 width/height strategy（通常是 fill_container）。
- form 中的 new inputs 必须匹配 existing inputs 的 width 和 height。
- new sections 必须使用与 existing sections 相同的 padding 和 gap patterns。

INSERTION RULES：

- 使用 "\_parent" 指定 new node 在 tree 中的位置。
- New sections 默认追加到最后一个 existing section 之后。
- list/grid 内的 new items 追加到最后一个 existing item 之后。
- 保留 z-order：overlay elements（badges、indicators）放在 content 之前。

COMMON PATTERNS：

- "Add a section" -> new frame with width="fill_container", height="fit_content", layout="vertical", matching section padding.
- "Add a card" -> new frame matching sibling card structure (same children pattern, same styles).
- "Add an input" -> new frame with role="input" or "form-input", width="fill_container", matching sibling inputs.
- "Add a button" -> new frame with role="button", matching existing button style.
- "Add a row" -> new frame with layout="horizontal", appropriate gap and alignment.

ID GENERATION：

- 为 new nodes 使用唯一且描述性的 IDs（例如 "new-feature-card", "contact-section"）。
- 绝不要复用 existing IDs。
