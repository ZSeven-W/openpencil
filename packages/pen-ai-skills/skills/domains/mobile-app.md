---
name: mobile-app
description: Mobile app screen design patterns (375x812)
phase: [generation]
trigger:
  keywords: [mobile, app, phone, ios, android]
priority: 35
budget: 1500
category: domain
---

MOBILE APP DESIGN PATTERNS:

VIEWPORT:
- Root frame: width=375, height=812 (fixed viewport)
- This is an ACTUAL mobile screen, NOT a desktop page with phone mockup
- "mobile"/"移动端"/"手机" + screen type = direct 375x812 screen

STATUS BAR:
- height=44, padding=[0,16], layout="horizontal", alignItems="center"
- Time, signal, battery indicators (text nodes, 12-13px)

HEADER:
- height=56-64, padding=[0,16], layout="horizontal"
- justifyContent="space_between", alignItems="center"
- Back arrow icon + title text + optional action icon

CONTENT AREA:
- padding=[0,16] or [16,16], gap=16-20
- layout="vertical", width="fill_container"
- Scroll-friendly: content flows vertically

TAB BAR (bottom navigation):
- height=80-84 (includes safe area), padding=[8,0,28,0]
- layout="horizontal", justifyContent="space_around", alignItems="center"
- Each tab: frame(layout="vertical", gap=4, alignItems="center") > icon_font + text(10-11px)
- Active tab: accent color, inactive: muted gray

FORM SCREENS (login, signup, settings):
- All inputs width="fill_container", height=48, gap=16
- Primary button width="fill_container", height=48
- Section gap=20-24

CARDS ON MOBILE:
- Full width: width="fill_container", cornerRadius=12-16
- padding=[16,16], gap=12
- Swipeable card rows: horizontal layout with fixed-width cards

LIST ITEMS:
- layout="horizontal", padding=[12,16], gap=12, alignItems="center"
- Leading: avatar/icon (40-48px)
- Content: vertical stack (title 16px + subtitle 14px muted)
- Trailing: chevron-right icon or status indicator

SPACING:
- Touch targets: minimum 44x44px
- Padding: 16px horizontal, 12-16px vertical
- Section gaps: 20-24px
- Safe area bottom: 28px padding
