use super::*;

fn node(value: serde_json::Value) -> PenNode {
    serde_json::from_value(value).expect("valid PenNode")
}

/// Apply a role's defaults and return the resulting node as JSON for inspection.
fn applied(role: &str, ctx: &RoleCtx, value: serde_json::Value) -> Value {
    let mut n = node(value);
    apply_role_defaults(&mut n, role, ctx);
    serde_json::to_value(&n).expect("serialize")
}

fn light() -> RoleCtx {
    RoleCtx::root(1200.0, Theme::Light)
}

// ── theme detection ───────────────────────────────────────────────────────

#[test]
fn theme_from_fill_luminance() {
    assert_eq!(detect_theme_from_fill(Some("#0F172A")), Theme::Dark);
    assert_eq!(detect_theme_from_fill(Some("#18181B")), Theme::Dark);
    assert_eq!(detect_theme_from_fill(Some("#FFFFFF")), Theme::Light);
    assert_eq!(detect_theme_from_fill(Some("#F8FAFC")), Theme::Light);
    // 3-digit hex.
    assert_eq!(detect_theme_from_fill(Some("#000")), Theme::Dark);
    // Unresolved ref / missing → default light.
    assert_eq!(detect_theme_from_fill(Some("$color-bg")), Theme::Light);
    assert_eq!(detect_theme_from_fill(None), Theme::Light);
}

// ── apply_role_defaults: set-if-absent ────────────────────────────────────

#[test]
fn navbar_defaults_injected_light() {
    let v = applied(
        "navbar",
        &light(),
        serde_json::json!({"type":"frame","id":"n","name":"Nav","children":[]}),
    );
    assert_eq!(v["layout"], serde_json::json!("horizontal"));
    assert_eq!(v["height"], serde_json::json!(72.0)); // desktop
    assert_eq!(
        v["fill"],
        serde_json::json!([{"type":"solid","color":"#FFFFFF"}])
    );
    assert_eq!(v["justifyContent"], serde_json::json!("space_between"));
    assert!(v["stroke"].is_object(), "navbar gets a bottom border");
}

#[test]
fn navbar_defaults_dark_theme() {
    let dark = RoleCtx::root(1200.0, Theme::Dark);
    let v = applied(
        "navbar",
        &dark,
        serde_json::json!({"type":"frame","id":"n","name":"Nav","children":[]}),
    );
    assert_eq!(
        v["fill"],
        serde_json::json!([{"type":"solid","color":"#111111"}])
    );
}

#[test]
fn navbar_mobile_height_and_padding() {
    let mobile = RoleCtx::root(390.0, Theme::Light);
    let v = applied(
        "navbar",
        &mobile,
        serde_json::json!({"type":"frame","id":"n","name":"Nav","children":[]}),
    );
    assert_eq!(v["height"], serde_json::json!(56.0)); // mobile
    assert_eq!(v["padding"], serde_json::json!([0.0, 16.0]));
}

#[test]
fn ai_explicit_value_is_never_overwritten() {
    // The node already has a fill + height — role defaults must NOT replace them.
    let v = applied(
        "navbar",
        &light(),
        serde_json::json!({
            "type":"frame","id":"n","name":"Nav",
            "fill":[{"type":"solid","color":"#FF0000"}],"height":99,"children":[]
        }),
    );
    assert_eq!(
        v["fill"],
        serde_json::json!([{"type":"solid","color":"#FF0000"}])
    );
    assert_eq!(v["height"], serde_json::json!(99.0));
    // …but absent fields still get filled.
    assert_eq!(v["justifyContent"], serde_json::json!("space_between"));
}

#[test]
fn card_in_horizontal_parent_stretches() {
    let mut ctx = light();
    ctx.parent_layout = Some("horizontal".into());
    let v = applied(
        "card",
        &ctx,
        serde_json::json!({"type":"frame","id":"c","name":"Card","children":[]}),
    );
    assert_eq!(v["width"], serde_json::json!("fill_container"));
    assert_eq!(v["height"], serde_json::json!("fill_container"));
    assert_eq!(v["cornerRadius"], serde_json::json!(12.0));
    assert!(v["effects"].is_array(), "card gets a shadow");
}

#[test]
fn card_in_vertical_parent_no_forced_width() {
    let v = applied(
        "card",
        &light(),
        serde_json::json!({"type":"frame","id":"c","name":"Card","children":[]}),
    );
    // No parent horizontal → width/height not forced.
    assert!(v.get("width").map(Value::is_null).unwrap_or(true));
    assert_eq!(v["cornerRadius"], serde_json::json!(12.0));
    assert_eq!(v["clipContent"], serde_json::json!(true));
}

#[test]
fn input_fill_and_stroke_themed() {
    let v = applied(
        "input",
        &light(),
        serde_json::json!({"type":"frame","id":"i","name":"Input","children":[]}),
    );
    assert_eq!(
        v["fill"],
        serde_json::json!([{"type":"solid","color":"#F8FAFC"}])
    );
    assert_eq!(v["height"], serde_json::json!(48.0));
}

#[test]
fn section_padding_scales_with_canvas_width() {
    let desktop = applied(
        "section",
        &light(),
        serde_json::json!({"type":"frame","id":"s","name":"Section","children":[]}),
    );
    assert_eq!(desktop["padding"], serde_json::json!([60.0, 80.0]));
    let mobile = applied(
        "section",
        &RoleCtx::root(390.0, Theme::Light),
        serde_json::json!({"type":"frame","id":"s","name":"Section","children":[]}),
    );
    assert_eq!(mobile["padding"], serde_json::json!([40.0, 16.0]));
}

#[test]
fn heading_is_cjk_aware() {
    let ascii = applied(
        "heading",
        &light(),
        serde_json::json!({"type":"text","id":"h","content":"Welcome"}),
    );
    assert_eq!(ascii["lineHeight"], serde_json::json!(1.2));
    assert_eq!(ascii["letterSpacing"], serde_json::json!(-0.5));
    let cjk = applied(
        "heading",
        &light(),
        serde_json::json!({"type":"text","id":"h","content":"欢迎光临"}),
    );
    assert_eq!(cjk["lineHeight"], serde_json::json!(1.35));
    assert_eq!(cjk["letterSpacing"], serde_json::json!(0.0));
}

#[test]
fn unknown_role_injects_nothing() {
    let v = applied(
        "totally-unknown-role",
        &light(),
        serde_json::json!({"type":"frame","id":"x","name":"X","children":[]}),
    );
    assert!(v.get("layout").map(Value::is_null).unwrap_or(true));
    assert!(v.get("fill").map(Value::is_null).unwrap_or(true));
}

#[test]
fn divider_orientation_from_name() {
    let horizontal = applied(
        "divider",
        &light(),
        serde_json::json!({"type":"frame","id":"d","name":"Divider","children":[]}),
    );
    assert_eq!(horizontal["height"], serde_json::json!(1.0));
    assert_eq!(horizontal["width"], serde_json::json!("fill_container"));
    let vertical = applied(
        "divider",
        &light(),
        serde_json::json!({"type":"frame","id":"d","name":"Vertical Divider","children":[]}),
    );
    assert_eq!(vertical["width"], serde_json::json!(1.0));
    assert_eq!(vertical["height"], serde_json::json!("fill_container"));
}
