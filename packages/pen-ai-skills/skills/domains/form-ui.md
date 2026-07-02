---
name: form-ui
description: Form, input, and interactive element design guidelines
phase: [generation]
trigger:
  keywords: [
      # English: form-specific
      form,
      contact form,
      feedback form,
      registration form,
      # English: auth flows
      login,
      log in,
      signin,
      sign in,
      signup,
      sign up,
      register,
      registration,
      password,
      # English: e-commerce
      checkout,
      # English: search & input components — multi-word so word-boundary
      # matching doesn't false-trigger on "research" / "input slider" etc.
      search bar,
      search input,
      search field,
      input field,
      text field,
      text input,
      # Chinese: form / auth
      表单,
      登录,
      注册,
      密码,
      # Chinese: search & input components (substring matching path)
      搜索,
      搜索框,
      输入框,
    ]
priority: 30
budget: 1500
category: domain
---

DESIGN GUIDELINES:

- Mobile: 375x812. Web: 1200x800 (single) or 1200x3000-5000 (landing page).
- "mobile"/"移动端" + screen type = ACTUAL 375x812 screen, NOT desktop with phone mockup.
- Buttons: height 44-52px, cornerRadius 8-12, padding [12, 24]. Icon+text: layout="horizontal", gap=8.
- Icon-only buttons: 44x44, justifyContent/alignItems="center", path icon 20-24px.
- Inputs: height 44px, light bg, subtle border, width="fill_container" in forms.
- Cards: cornerRadius 12-16, clipContent: true, subtle shadows.
- CARD ROW ALIGNMENT: sibling cards in horizontal layout ALL use width/height="fill_container".
- Navigation: justifyContent="space_between", 3 groups (logo | links | CTA), padding=[0,80].
- Phone mockup: ONE "frame", width 260-300, height 520-580, cornerRadius 32. NEVER ellipse.
- NEVER use ellipse for decorative shapes. Use frame/rectangle with cornerRadius.
- NEVER use emoji as icons. Use path nodes with Feather icon names.
