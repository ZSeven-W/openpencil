---
name: landing-page-predesign
description: Mandatory pre-design steps for landing pages — concept extraction, superfan simulation, transformation mapping
phase: [planning]
trigger:
  keywords: [landing, website, 官网, 营销, marketing, promotional, homepage]
priority: 8
budget: 1500
category: domain
---

LANDING PAGE PRE-DESIGN（在内部执行这些步骤，然后把结果作为 JSON fields 放入你的输出）：

## Step 1: Concept Extraction

识别这个页面必须传达的 core concepts：

- Domain concepts：产品所属的 space/category，以及它做什么
- Qualitative concepts：体验应该给人什么感觉

将每个 concept 标记为 primary 或 secondary。
把每个 concept 映射到具体 design decision：

- Content：这个 concept 要说什么？
- Layout：它如何组织？
- Color：什么 palette 支撑它？
- Typography：什么 type treatment 合适？

## Step 2: Superfan Simulation

模拟一次与 product superfan 的简短 research interview。提取 2-5 条 insights：

- 他们喜欢产品的什么？
- 什么地方让他们觉得 magical？
- 他们会向别人讲述什么故事？
- 什么 visuals 对他们来说显得 authentic？

将这些 insights 应用到：

- Hero messaging（什么 headline 能产生共鸣？）
- Content hierarchy（什么先出现？）
- Section priorities（什么最重要？）
- Visual direction（什么感觉是对的？）

## Step 3: Transformation Mapping

定义页面的 emotional arc：

- Before State：visitor 此刻感受到的 pain、frustration 或 limitation
- After State：使用产品后生活是什么样子（强调情感，不只是功能）
- Bridge：产品如何把他们从 Before → After
- Feeling：页面应唤起的一个 dominant emotion
  (confidence / liberation / belonging / power / calm / mastery)

每个 section 都应该隐约回答："Here's where we're taking you."

## Output

将结果作为 "preDesignContext" field 放进你的 JSON plan object 内部（与 rootFrame、styleGuideName、subtasks 同级）：
"preDesignContext":{"primaryConcepts":["..."],"superfanInsights":["..."],"transformation":{"before":"...","after":"...","bridge":"...","feeling":"..."}}

不要把这些 steps 作为 prose 输出。不要解释你的 reasoning。只以上面的 JSON field 形式包含它们。

这个 context 会指导 subtask decomposition 和 style guide selection。
