---
name: mobile-app
description: Mobile app three-section architecture with enforced Blueprint
phase: [generation]
trigger:
  keywords: [mobile, phone, ios, android, 移动, 手机]
priority: 25
budget: 1500
category: domain
---

MOBILE APP — 强制 THREE-SECTION ARCHITECTURE：

每个 mobile screen 都由正好三个 sections 组成的 vertical stack 构成。
生成任何 content 前，你必须先定义这三层。

## 1) STATUS BAR（OS-controlled）— PRE-INSERTED

status bar（time、signal、wifi、battery）会由 orchestrator 作为 root frame 的第一个 child **自动预先插入**。它是一个固定 62px 高的 frame，包含 hardcoded path icons。

- **不要生成 status bar** — 它已经存在
- **不要删除或修改** pre-inserted status bar
- 你的第一个 section 应该从 status bar 下方开始（它占据约 62px）

## 2) APP CONTENT（你的 layout）

所有 content elements 都必须位于一个 wrapper container（vertical stack）内部。

Wrapper 提供：

- 一致的 left/right padding：16-20px（只在 wrapper level 应用一次）
- 基于 gap 的 sections 间 vertical spacing（使用 gap，不要使用 margins）
- padding-bottom 等于 gap value，用于 bottom space（不要使用 spacer elements）

wrapper 内部的 content stacking order：

1. Top context：title / navigation header / search / filters
2. Primary content：这个 screen 的主要 "job to be done"
3. Supporting content：secondary modules、help text、empty states
4. Floating actions（optional）：FAB 或 sticky CTA

规则：

- 每个 screen 只有一个 primary intent。其他内容都从属于它。
- 前 1-2 个 elements 必须回答 "where am I" + "what can I do here"
- Title font size 在 app 的所有 screens 中必须统一
- 为 one-handed use 设计：primary actions 放在 lower half
- 单一 vertical scroll（避免 nested scrolls）
- Touch targets：最小 44x44px

不要：

- 添加 per-section horizontal padding（wrapper 会处理）
- 为 bottom space 使用 spacer elements（使用 padding-bottom）
- 在 above the fold 中塞入多个互相竞争的 sections

## 3) BOTTOM TAB BAR — PILL STYLE

Tab Bar Container：

- Full screen width
- Padding：[12, 21, 21, 21]（包含 home-indicator safe area）
- Fill：gradient overlay（顶部 transparent → 30% 处 solid background）

Pill（menu items wrapper）：

- Height：62px，width：fill_container
- Corner radius：36px
- Border：1px solid（theme border color）
- Inner padding：4px

Tab Items（3-5 tabs，仅 top-level destinations）：

- Width：fill_container，height：fill_container
- Corner radius：26px
- Layout：vertical，gap：4，两个轴都居中
- Icon：18px
- Label：10px，weight 500-600，UPPERCASE，letterSpacing：0.5

Active state：solid fill（accent color）+ contrasting icon/label color
Inactive state：transparent background + muted icon/label color

规则：

- Labels 必须 uppercase
- Tab switching 保留每个 tab 的 navigation state
- App content 绝不能被 Tab Bar 遮挡

## BLUEPRINT（internal planning）

生成 nodes 前，在脑中验证这三层都已考虑：

1. Status Bar：standard 还是 edge-to-edge？
2. App Content：header、primary content、action placement、scroll behavior 是什么？
3. Bottom Bar：None 还是 Pill Tab Bar（哪些 tabs）？

不要把这个 blueprint 作为文本输出。通过你的 node structure 静默应用它。
你的输出必须始终只保持为 valid JSON/JSONL。
