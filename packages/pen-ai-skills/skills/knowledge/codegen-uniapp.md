---
name: codegen-uniapp
description: UniApp code generation rules — multi-file Vue-based project structure for cross-platform mini-app workflows
phase: [generation]
trigger:
  flags: [isCodeGen]
priority: 20
budget: 2200
category: knowledge
---

# UniApp 代码生成

生成可用于 UniApp 项目的多文件输出，优先产出结构正确、可维护的项目骨架和页面代码，而不是简单地把 Vue 输出改名。

## 输出目标

- `App.vue`
- `main.ts`
- `pages.json`
- `manifest.json`
- `uni.scss`
- 页面文件，例如 `pages/index/index.vue`
- 组件文件，例如 `components/<name>.vue`
- 静态资源文件，例如 `static/<asset>`

## 关键原则

- 生成结果必须是多文件项目，而不是单文件组件。
- `pages.json` 必须和生成页面保持一致。
- `App.vue`、`main.ts`、`uni.scss` 需要作为项目入口的一部分。
- 页面结构优先采用 Vue SFC。
- 组件应可复用，避免把所有内容塞进单个页面文件。
- 若存在图片或图标资源，应以相对路径引用，避免内联 base64。

## 文件映射建议

- 根容器 → 页面根 `<view>` 或 `<scroll-view>`
- 纵向布局 → `display: flex; flex-direction: column`
- 横向布局 → `display: flex; flex-direction: row`
- 间距 → `gap` 或 `margin`
- 填充容器 → `width: 100%`
- 文本样式 → 统一抽到 `uni.scss` 变量和 class

## 页面约定

- 页面需要语义清晰的主标题、内容区和主要操作。
- 移动端页面应考虑安全区和顶部状态区域。
- 页面内容应适配 H5、App 和小程序的通用能力。
- 第一阶段不要求覆盖所有平台专属 API。

## `pages.json` 约定

- 必须包含生成页面的路由定义。
- 首页页面应作为第一个页面或默认页面。
- 若存在多个页面，应按照用户界面结构输出。

## `manifest.json` 约定

- 只输出生成项目的必要字段。
- 不要捏造不需要的平台能力。
- 若平台配置不明确，用最小可运行骨架。

## 组件拆分

- 把可重复块拆成独立组件。
- 组件命名应简洁、可读。
- 复杂布局优先拆成页面 + 多个组件文件。

## 设计变量

- `$variable` 引用应映射成 `var(--variable-name)` 或 UniApp 可接受的样式变量写法。
- 颜色、间距、字号等优先从统一变量派生。

## 输出规则

- 代码生成时输出单个文件内容仍可用于 chunk 级结果，但最终 assembly 必须拼成多文件 bundle。
- 结果中若存在多个页面，应确保每个页面文件路径正确。
- 输出中不要出现与 UniApp 无关的框架特有语法残留。

