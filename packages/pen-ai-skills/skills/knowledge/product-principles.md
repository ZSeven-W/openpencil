---
name: product-principles
description: Product-level design principles for functional UI quality
phase: [generation]
trigger: null
priority: 5
budget: 800
category: base
---

PRODUCT DESIGN PRINCIPLES（应用到每个 screen）：

1. PURPOSE FIRST
   每个 screen 必须有一个明确定义的 primary purpose 和一个 primary action。
   如果多个 goals 相互竞争，把它们拆成不同 surfaces。

2. DOMINANT REGION
   每个 screen 必须包含一个 dominant visual region。
   Visual weight 必须反映重要性。避免 equal-weight layouts 和 competing focal points。

3. ACTION HIERARCHY
   每个 screen 或 section 只有一个 primary action。Secondary actions 视觉上弱化。
   Destructive actions 明确区分。Rare actions 放在 overflow menus 中。
   不要给所有 actions 同等强调。

4. ENTITY INTEGRITY
   表示 entity（user、record、document、asset）时：
   突出显示其 name，清晰呈现 status，展示 key metadata，并让 actions 明显。

5. DENSITY INTENTIONALITY
   每个 screen 选择一种 density mode 并保持一致：

- Compact: high data environments (tables, dashboards)
- Medium: balanced default (most screens)
- Airy: low-complexity workflows (onboarding, settings)
  不要在一个 screen 内混用 density modes。

6. CONSTRAINT OVER DECORATION
   如果某个 element 不支持 navigation、understanding、decision-making 或 action-taking，
   它就不应该存在。尽可能少地设计。

7. STRUCTURAL CONSISTENCY
   相似问题必须有相似解法。Navigation logic 必须保持稳定。
   Layout rhythm 应该有 system-driven 的感觉。Spacing 必须遵循 consistent scale。

8. SYSTEM STATUS VISIBILITY
   每个 data-driven surface 都必须支持：loading state、empty state、error state、
   success confirmation。不要 silent failure。不要 blank ambiguity。
