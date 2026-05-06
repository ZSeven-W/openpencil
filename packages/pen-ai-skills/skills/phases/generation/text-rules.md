---
name: text-rules
description: Text sizing, typography, and wrapping rules
phase: [generation]
trigger: null
priority: 15
budget: 1000
category: base
---

TEXT RULES：

- vertical layout 中的 Body/description：width="fill_container" + textGrowth="fixed-width"（自动换行，并自动计算 height）。
- horizontal rows 中的短 labels：width="fit_content" + textGrowth="auto"。防止挤压 siblings。
- 绝不要在 layout frames 内的 text 上使用 fixed pixel width — 会导致 overflow。
- Text >15 chars 必须设置 textGrowth="fixed-width"。绝不要在 text nodes 上设置 explicit pixel height — 省略 height。
- Typography：Display 40-56px，Heading 28-36px，Subheading 20-24px，Body 16-18px，Caption 13-14px。
- lineHeight：headings 1.1-1.2，body 1.4-1.6。letterSpacing：headlines 使用 -0.5，uppercase 使用 0.5-2。
