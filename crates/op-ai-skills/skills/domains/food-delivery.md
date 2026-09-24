---
name: food-delivery
description: Food delivery / takeout app anatomy - home feed, store page, dish rows, cart bar
phase: [generation]
trigger:
  keywords: [外卖, 点餐, 点单, 餐厅, 饭店, 菜品, 菜单, 奶茶, 美食, 商家, takeout, take-out, food delivery, delivery app, food ordering, restaurant, dishes]
priority: 26
budget: 1500
category: domain
---

FOOD DELIVERY / TAKEOUT — what the leading apps converge on. Sources: Baymard Food Delivery & Takeout research (8 leading apps, 390+ UX parameters), DoorDash Merchant menu data, 美团 / 饿了么 product teardowns, 优设 金刚区 analysis. Where these rules differ from the generic mobile rules (icon tiles, list-row alignment), these win. A restaurant web page uses only the photo, dish-row and store-header rules.

The look comes from FOOD PHOTOGRAPHY, not from painted blocks. Store header images lift sales up to 50%, logos 23%, dish photos 30-44%, dish descriptions 18% (DoorDash). So every store, dish and category visual is an `image` with a specific `imagePrompt` + `imageSearchQuery` ("mapo tofu in clay bowl, overhead"). Dish shots share one angle: overhead, 1:1, warm light. Accent colour is only for prices, add buttons, the checkout CTA, the active tab and promo tag text. At most ONE block per screen has an accent fill.

HOME, top to bottom:
1. Address + search as one block. Address row: pin icon, 12px "送至", 15-16px semibold address, chevron. Then a full-width 40-44px search field whose placeholder carries a benefit ("满30减12 · 搜索美食"; 美团 puts the discount in the search box).
2. 金刚区 categories: 5 per row, 8-10 items, two rows max. Each visual is a 48-52px FOOD image (round cut-out dish or 3D illustration via imagePrompt), NOT a line glyph on a tinted tile: a box around the icon lowers recognition (优设). Label 12px, ≤4 CJK chars. The first row reads strongest (美团 puts the key entries there).
3. Order again: returning users look for reorder on the home page (Baymard; DoorDash "Order Again"). Use a horizontal row of 2-3 recent stores, each with photo, name and a small "再来一单" button.
4. One promo moment: a flash deal with REAL countdown digits as plain text ("02:15:38", tabular cells), a deal price beside a struck-through price, and one dish photo. Not an empty orange slab.
5. Sort/filter row: 综合排序 · 距离 · 销量 · 筛选, chips 32-36px. About half of users filter sometimes (Baymard).
6. Store list rows (no shadow box per row; separate rows with 16-20px spacing or a hairline):
   - 80-88px photo (storefront or signature dish), radius 10-12.
   - Name 16-17px semibold.
   - Meta line 12-13px: ★4.8 · 月售3200+ · 30分钟 · 1.2km. Delivery TIME is the deciding figure: give it weight or the accent.
   - Cost line 12px muted: 起送¥20 · 配送¥3.
   - Up to 3 promo tags: 满30减12, 新客立减5. Red/orange text on a light tint, 18-20px tall, radius 4.
   - On 1-2 featured stores, add a strip of 3 signature-dish thumbnails (56-64px, price under each; 美团 "发现好菜"). It breaks the identical-row rhythm.

STORE PAGE:
- Full-bleed store photo 180-220px under the status bar, with back/favorite/share as 36px translucent circles on it. A 56-64px store logo (radius 12, white 2px border) overlaps the photo's bottom edge.
- Info block: name 20-22px bold; ★ · 月售 · 时间 · 距离; promo tags; one announcement line.
- 商家推荐 / "Most ordered": a horizontal carousel of 3-4 signature dishes, each card 132-140px wide with a 1:1 photo, name and price (饿了么 recommended-dish module).
- Menu layout:
  - Chinese apps use a LEFT CATEGORY RAIL (80-92px wide, tinted background; the selected item is white with a 3px accent bar; labels 13-14px) beside the dish list. Use it for Chinese briefs.
  - Western apps use sticky horizontal category tabs instead.
- Dish row, `alignItems: start`:
  - 88-96px square photo, radius 10.
  - Name 15-16px semibold.
  - One-line description 12-13px muted.
  - "月售 1260" 12px muted.
  - Price 17-18px bold, with an optional struck-through original.
  - 28-32px accent circle "+" at the bottom-right; it becomes a − 1 + stepper once the dish is in the cart. Padding keeps the touch target at least 44px (HIG).
- Cart bar: a floating dark pill, 56-60px tall, radius full, inset 12-16px from the sides and bottom.
  - Cart disc 44-48px overlapping the pill's left edge, with an 18px count badge.
  - Total 18-20px bold white, then "另需配送费¥3" 11-12px.
  - Checkout CTA on the right. Below the minimum order, show "还差¥8起送" in place of the CTA (饿了么 shortfall prompt).

DO NOT: draw line icons on tinted squares for food categories; give every store row an identical white shadow card; paint the whole header orange; leave dish or store photos out; use more than one accent-filled block per screen.
