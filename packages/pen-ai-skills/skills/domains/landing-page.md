---
name: landing-page
description: Landing page and marketing site design patterns
phase: [generation]
trigger:
  keywords: [landing, marketing, hero, homepage]
priority: 35
budget: 1500
category: domain
---

LANDING PAGE DESIGN PATTERNS：

STRUCTURE：

- Navigation - Hero - Features - Social Proof - CTA - Footer
- 每个 section：width="fill_container"，height="fit_content"，layout="vertical"
- Root frame：width=1200，height=0（auto-expands），gap=0

NAVIGATION：

- justifyContent="space_between"，3 groups：logo | nav-links | CTA button
- padding=[0,80]，alignItems="center"，height 64-80px
- Links 在 center group 中均匀分布

HERO SECTION：

- padding=[80,80] 或更大，留出充足 whitespace
- 一个 headline（40-56px），一个 subtitle（16-18px），一个 CTA button
- Optional visual：右侧 phone mockup 或 illustration（two-column horizontal layout）
- 每个额外 element 都会稀释 focus — 保持 minimal

FEATURE SECTIONS：

- Section title + 3-4 个 horizontal layout 中的 feature cards
- Cards：width="fill_container"，height="fill_container"，用于均匀 row alignment
- 交替 section backgrounds（#FFFFFF / #F8FAFC），形成自然分隔
- Section vertical padding：80-120px

SOCIAL PROOF：

- Testimonials：带 quote + avatar + name/title 的 card
- Stats：带 stat-cards（number + label）的 horizontal row
- Logos：company logos 的 horizontal row

CTA SECTION：

- Centered content、compelling headline、accent background 或 gradient
- 一个醒目的 button

FOOTER：

- Multi-column layout：brand + link groups + social
- Muted colors，smaller text
- padding=[48,80]

GENERAL：

- 各 sections 使用 centered content container ~1040-1160px，以保持 alignment stability
- Consistent cornerRadius（cards 12-16px）
- 带 images 的 cards 设置 clipContent: true
- Cards 使用 subtle shadows

## Headline Hierarchy

按由强到弱的层级写 headlines：

1. Transformation: "Finally feel in control of your inbox" (strongest)
2. Outcome: "Ship more content, grow your audience"
3. Benefit: "Write 10x faster"
4. Feature: "AI-powered writing assistant" (weakest)

优先使用 transformation 或 outcome。Benefit/feature 只放在 supporting copy 中。

## Image Intent Hierarchy

选择 imagery 时（从上到下优先级递减）：

1. Transformation imagery：处于 "after state" 的人物 — emotion、outcome、identity achieved
2. Contextual use：人在真实环境中使用产品
3. Product-in-environment：产品处于暗示使用/结果的场景中
4. Isolated product：单独的产品 — 谨慎使用

每张 image 都应该像 visitor future life 的一个场景。
自问："Would the visitor think 'I want to feel that way'?"

绝不要使用 AI images 作为 background fills 再把 text 放在上面。
Images 和 text 应该是 siblings，而不是 layers。
