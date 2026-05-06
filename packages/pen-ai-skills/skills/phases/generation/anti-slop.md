---
name: anti-slop
description: Prevent generic AI aesthetics and enforce visual diversity across generations
phase: [generation]
trigger:
  keywords: [landing, website, marketing, 页面, 网站, promotional, homepage, 官网, 营销]
priority: 15
budget: 600
category: domain
---

ANTI-SLOP 规则（每个设计都必须遵守）：

## 视觉多样性

1. 主要 section 不要使用平铺的纯色背景。
   至少使用：微妙 gradient、noise texture、geometric pattern 或 layered fills。

2. Card layout 不得完全相同。
   每个 card group 至少需要 1 个差异化元素（size、image placement、accent）。

3. 交替 section 节奏：text-heavy <-> visual、dark <-> light。
   不要连续堆叠多个纯文本 section。

4. Hero section：绝不要使用 AI-generated images 作为 background fills 再把文本叠在上面。
   Images 和 text 应该是 siblings，而不是 layers。

## 跨生成多样性

Recent generation history（用于避免重复）：
{{recentHistory}}

规则：

- Heading font 必须不同于上面列出的 recent generations
- Color palette 不得重复最近一次 generation

## 创意变化（必需）

确定 baseline direction 后，引入 1-3 个小的 creative variations（每个约 10%）：

- 某个 section 使用 asymmetric layout
- 不常规的 image cropping 或 placement
- Typography personality shift（weight、case、spacing）
- Depth/layering effect

这些 variations 不得在不同 generations 间重复。

重要：不要输出 prose 或 explanations。你的输出必须始终只保持为 valid JSON/JSONL。
通过你的 design choices 静默应用这些规则。
