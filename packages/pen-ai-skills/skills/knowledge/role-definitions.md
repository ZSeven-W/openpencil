---
name: role-definitions
description: Semantic role system with default property values
phase: [generation]
trigger:
  keywords: [landing, marketing, hero, website, 官网, 首页, 产品页, table, grid, 表格, 表头, dashboard, 数据, admin, testimonial, pricing, footer, stats, 评价, 定价, 页脚, 数据统计]
priority: 35
budget: 2000
category: knowledge
---

SEMANTIC ROLES (add "role" to nodes — system fills unset props based on role):

Layout roles:
- section: layout=vertical, width=fill_container, height=fit_content, gap=24, padding=[60,80] (mobile: [40,16]), alignItems=center
- row: layout=horizontal, width=fill_container, gap=16, alignItems=center
- column: layout=vertical, width=fill_container, gap=16
- centered-content: layout=vertical, width=1080 (mobile: fill_container), gap=24, alignItems=center
- form-group: layout=vertical, width=fill_container, gap=16
- divider: width=fill_container, height=1, layout=none (vertical divider: width=1, height=fill_container)
- spacer: width=fill_container, height=40

Navigation roles:
- navbar: layout=horizontal, width=fill_container, height=72 (mobile: 56), padding=[0,80] (mobile: [0,16]), alignItems=center, justifyContent=space_between
- nav-links: layout=horizontal, gap=24, alignItems=center
- nav-link: textGrowth=auto, lineHeight=1.2

Interactive roles:
- button: padding=[12,24], height=44, layout=horizontal, gap=8, alignItems=center, justifyContent=center, cornerRadius=8. In navbar: padding=[8,16], height=36. In form-group: width=fill_container, height=48, cornerRadius=10
- icon-button: width=44, height=44, layout=horizontal, justifyContent=center, alignItems=center, cornerRadius=8
- badge: layout=horizontal, padding=[6,12], gap=4, alignItems=center, justifyContent=center, cornerRadius=999
- tag: layout=horizontal, padding=[4,10], gap=4, alignItems=center, justifyContent=center, cornerRadius=6
- pill: layout=horizontal, padding=[6,14], gap=6, alignItems=center, justifyContent=center, cornerRadius=999
- input: height=48, layout=horizontal, padding=[12,16], alignItems=center, cornerRadius=8. In vertical layout: width=fill_container
- form-input: width=fill_container, height=48, layout=horizontal, padding=[12,16], alignItems=center, cornerRadius=8
- search-bar: layout=horizontal, height=44, padding=[10,16], gap=8, alignItems=center, cornerRadius=22

Display roles:
- card: layout=vertical, gap=12, cornerRadius=12, clipContent=true. In horizontal layout: width=fill_container, height=fill_container
- stat-card: layout=vertical, gap=8, padding=[24,24], cornerRadius=12. In horizontal layout: width=fill_container, height=fill_container
- pricing-card: layout=vertical, gap=16, padding=[32,24], cornerRadius=16, clipContent=true. In horizontal layout: width=fill_container, height=fill_container
- image-card: layout=vertical, gap=0, cornerRadius=12, clipContent=true
- feature-card: layout=vertical, gap=12, padding=[24,24], cornerRadius=12. In horizontal layout: width=fill_container, height=fill_container

Media roles:
- phone-mockup: width=280, height=560, cornerRadius=32, layout=none
- screenshot-frame: cornerRadius=12, clipContent=true
- avatar: width/height=48, cornerRadius=24, clipContent=true (size adapts to explicit width)
- icon: width=24, height=24

Typography roles:
- heading: lineHeight=1.2 (CJK: 1.35), letterSpacing=-0.5 (CJK: 0). In vertical layout: textGrowth=fixed-width, width=fill_container
- subheading: lineHeight=1.3 (CJK: 1.4), textGrowth=fixed-width, width=fill_container
- body-text: lineHeight=1.5 (CJK: 1.6), textGrowth=fixed-width, width=fill_container
- caption: lineHeight=1.3 (CJK: 1.4), textGrowth=auto
- label: lineHeight=1.2, textGrowth=auto, textAlignVertical=middle

Content roles:
- hero: layout=vertical, width=fill_container, height=fit_content, padding=[80,80] (mobile: [40,16]), gap=24, alignItems=center
- feature-grid: layout=horizontal, width=fill_container, gap=24, alignItems=start
- testimonial: layout=vertical, gap=16, padding=[24,24], cornerRadius=12
- cta-section: layout=vertical, width=fill_container, height=fit_content, padding=[60,80] (mobile: [40,16]), gap=20, alignItems=center
- footer: layout=vertical, width=fill_container, height=fit_content, padding=[48,80] (mobile: [32,16]), gap=24
- stats-section: layout=horizontal, width=fill_container, height=fit_content, padding=[48,80] (mobile: [32,16]), gap=32, justifyContent=center, alignItems=center

Table roles:
- table: layout=vertical, width=fill_container, gap=0, clipContent=true
- table-row: layout=horizontal, width=fill_container, alignItems=center, padding=[12,16]
- table-header: layout=horizontal, width=fill_container, alignItems=center, padding=[12,16]
- table-cell: width=fill_container

Your explicit props ALWAYS override role defaults. Only unset properties get filled in.
