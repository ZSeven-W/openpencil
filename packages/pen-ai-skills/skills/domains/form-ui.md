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

DESIGN GUIDELINES：

- Mobile：375x812。Web：1200x800（single）或 1200x3000-5000（landing page）。
- "mobile"/"移动端" + screen type = 实际 375x812 screen，不是带 phone mockup 的 desktop。
- Buttons：height 44-52px，cornerRadius 8-12，padding [12, 24]。Icon+text：layout="horizontal"，gap=8。
- Icon-only buttons：44x44，justifyContent/alignItems="center"，path icon 20-24px。
- Inputs：height 44px，light bg，subtle border，forms 中 width="fill_container"。
- Cards：cornerRadius 12-16，clipContent: true，subtle shadows。
- CARD ROW ALIGNMENT：horizontal layout 中的 sibling cards 全部使用 width/height="fill_container"。
- Navigation：justifyContent="space_between"，3 groups（logo | links | CTA），padding=[0,80]。
- Phone mockup：一个 "frame"，width 260-300，height 520-580，cornerRadius 32。绝不要用 ellipse。
- 绝不要把 ellipse 用作 decorative shapes。使用带 cornerRadius 的 frame/rectangle。
- 绝不要使用 emoji 作为 icons。使用带 Feather icon names 的 path nodes。
