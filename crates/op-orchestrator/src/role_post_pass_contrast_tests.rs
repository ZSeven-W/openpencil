//! Tests for the contrast + surface-structure clusters:
//! `fix_button_foreground_contrast`, `fix_orphan_container_contrast`, the
//! orphaned-shadow strip, `fix_structural_wrapper_transparency`,
//! `fix_surface_color_discipline` and `fix_section_alternation`.

use super::*;
use serde_json::json;

// ── fixButtonForegroundContrast ──────────────────────────────────────────

#[test]
fn button_dark_bg_gets_white_text() {
    let mut btn = json!({
        "type":"frame","role":"button","fill":[{"type":"solid","color":"#1E40AF"}],
        "children":[{"type":"text","id":"t","content":"Go"}]
    });
    fix_button_foreground_contrast(&mut btn);
    assert_eq!(
        btn["children"][0]["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}])
    );
}

#[test]
fn accent_token_button_flips_dark_icon_to_white() {
    // Regression: a `$--primary` (or `$--primary`) button binds its bg
    // hex only at render time, so the contrast pass could not read its
    // luminance and left the model's default-dark icon on the orange accent.
    let mut btn = json!({
        "type":"frame","role":"icon-button",
        "fill":[{"type":"solid","color":"$--primary"}],
        "children":[{"type":"icon_font","iconFontName":"sliders",
            "fill":[{"type":"solid","color":"#0F172A"}]}]
    });
    fix_button_foreground_contrast(&mut btn);
    assert_eq!(
        btn["children"][0]["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}]),
        "dark icon on an accent-token button must flip to white"
    );

    // An already-light icon on the accent stays put.
    let mut ok = json!({
        "type":"frame","role":"icon-button",
        "fill":[{"type":"solid","color":"$--primary"}],
        "children":[{"type":"icon_font","fill":[{"type":"solid","color":"#FFFFFF"}]}]
    });
    fix_button_foreground_contrast(&mut ok);
    assert_eq!(
        ok["children"][0]["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}])
    );

    // A NON-accent token bg (surface) still can't be resolved → pass skips,
    // no accidental white on a light button.
    let mut surface = json!({
        "type":"frame","role":"icon-button",
        "fill":[{"type":"solid","color":"$--card"}],
        "children":[{"type":"icon_font","fill":[{"type":"solid","color":"#0F172A"}]}]
    });
    fix_button_foreground_contrast(&mut surface);
    assert_eq!(
        surface["children"][0]["fill"],
        json!([{"type":"solid","color":"#0F172A"}])
    );
}

#[test]
fn button_light_bg_gets_dark_text() {
    let mut btn = json!({
        "type":"frame","role":"button","fill":[{"type":"solid","color":"#FFFFFF"}],
        "children":[{"type":"text","id":"t","content":"Go"}]
    });
    fix_button_foreground_contrast(&mut btn);
    assert_eq!(
        btn["children"][0]["fill"],
        json!([{"type":"solid","color":"#0F172A"}])
    );
}

#[test]
fn button_icon_copies_sibling_text_color() {
    // text has an explicit white fill → the unfilled icon should match it.
    let mut btn = json!({
        "type":"frame","role":"button","fill":[{"type":"solid","color":"#1E40AF"}],
        "children":[
            {"type":"text","fill":[{"type":"solid","color":"#FFFFFF"}],"content":"Go"},
            {"type":"icon_font","id":"i"}
        ]
    });
    fix_button_foreground_contrast(&mut btn);
    assert_eq!(
        btn["children"][1]["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}])
    );
}

#[test]
fn button_transparent_fill_skipped() {
    // No visible bg → nothing to compute contrast against; text untouched.
    let mut btn = json!({
        "type":"frame","role":"button","fill":[{"type":"solid","color":"transparent"}],
        "children":[{"type":"text","id":"t","content":"Go"}]
    });
    fix_button_foreground_contrast(&mut btn);
    assert!(btn["children"][0].get("fill").is_none());
}

#[test]
fn button_unresolved_non_accent_ref_bg_skipped() {
    // A non-accent design token can't resolve to a hex here AND isn't a known
    // saturated-accent family, so the pass can't pick a safe fg → skip. (An
    // accent token like $--primary now DOES flip children to white — see
    // `accent_token_button_flips_dark_icon_to_white`.)
    let mut btn = json!({
        "type":"frame","role":"button","fill":[{"type":"solid","color":"$--card-raised"}],
        "children":[{"type":"text","id":"t","content":"Go"}]
    });
    fix_button_foreground_contrast(&mut btn);
    assert!(btn["children"][0].get("fill").is_none());
}

// ── fixOrphanContainerContrast ───────────────────────────────────────────

#[test]
fn orphan_card_gets_fill_and_shadow() {
    let mut card = json!({
        "type":"frame","role":"card","cornerRadius":12,
        "children":[{"type":"text","content":"x"}]
    });
    // Parent exists (Some) but has no fill (Null) → orphan fix fires.
    fix_orphan_container_contrast(&mut card, Some(&Value::Null));
    assert_eq!(card["fill"], json!([{"type":"solid","color":"$--card"}]));
    assert!(card["effects"].is_array());
}

#[test]
fn orphan_skipped_when_parent_has_fill() {
    let mut card = json!({
        "type":"frame","role":"card","cornerRadius":12,
        "children":[{"type":"text","content":"x"}]
    });
    let parent_fill = json!([{"type":"solid","color":"#EEEEEE"}]);
    fix_orphan_container_contrast(&mut card, Some(&parent_fill));
    assert!(
        card.get("fill").is_none(),
        "child on a filled parent stays transparent"
    );
}

#[test]
fn orphan_skipped_for_root_and_structural_roles() {
    // Root (no parent) → skip.
    let mut card =
        json!({"type":"frame","role":"card","cornerRadius":12,"children":[{"type":"text"}]});
    fix_orphan_container_contrast(&mut card, None);
    assert!(card.get("fill").is_none());
    // Structural role (section) → skip even with cornerRadius + no parent fill.
    let mut sect =
        json!({"type":"frame","role":"section","cornerRadius":12,"children":[{"type":"text"}]});
    fix_orphan_container_contrast(&mut sect, Some(&Value::Null));
    assert!(sect.get("fill").is_none());
}

#[test]
fn orphan_skipped_without_corner_radius() {
    let mut card = json!({"type":"frame","role":"card","children":[{"type":"text"}]});
    fix_orphan_container_contrast(&mut card, Some(&Value::Null));
    assert!(
        card.get("fill").is_none(),
        "no cornerRadius → not a card silhouette"
    );
}

#[test]
fn orphan_skipped_for_roleless_container() {
    // A roleless rounded container is NOT assumed to be a card. Rust role
    // inference is thinner than TS, so roleless wrappers (header / section /
    // banner) are common; white-washing them is what produced the spurious
    // panels behind the header / banner / search row.
    let mut wrap = json!({
        "type":"frame","cornerRadius":16,
        "children":[{"type":"frame","role":"section","children":[{"type":"text"}]}]
    });
    fix_orphan_container_contrast(&mut wrap, Some(&Value::Null));
    assert!(
        wrap.get("fill").is_none(),
        "roleless container is not white-washed into a card"
    );
}

#[test]
fn orphan_skipped_when_child_container_paints_surface() {
    // glm wraps an orange promo banner in a bare `feature-card` frame. The child
    // (Promo Card) carries the full-bleed gradient, so the wrapper must NOT be
    // whitewashed — doing so leaks the injected drop-shadow out as a gray ghost
    // box around the orange child (the user's "mysterious bg + rounded border").
    let mut wrap = json!({
        "type":"frame","role":"feature-card","cornerRadius":12,
        "width":320,"height":180,"layout":"vertical",
        "children":[{
            "type":"frame","role":"card",
            "width":"fill_container","height":"fill_container",
            "fill":[{"type":"linear_gradient","angle":135,"stops":[
                {"offset":0.0,"color":"#FB923C"},{"offset":1.0,"color":"#F97316"}]}],
            "children":[{"type":"text","content":"50% Off"}]
        }]
    });
    fix_orphan_container_contrast(&mut wrap, Some(&Value::Null));
    assert!(
        wrap.get("fill").is_none(),
        "wrapper around a filled card stays transparent"
    );
    assert!(
        wrap.get("effects").is_none(),
        "no ghost shadow injected onto the bare wrapper"
    );
}

#[test]
fn orphan_wrapper_skips_near_full_bleed_surface_with_overlay_badge() {
    let mut wrap = json!({
        "type":"frame","role":"feature-card","name":"Promo Wrapper",
        "width":320,"height":180,"layout":"none","cornerRadius":16,
        "children":[
            {
                "type":"frame","role":"card","name":"Promo Card",
                "x":0,"y":0,"width":320,"height":180,
                "fill":[{"type":"solid","color":"#F97316"}],
                "children":[{"type":"text","content":"50% Off"}]
            },
            {
                "type":"frame","role":"badge","name":"Promo Badge",
                "x":16,"y":16,"width":"fit_content","height":24,
                "fill":[{"type":"solid","color":"#FFFFFF"}],
                "children":[{"type":"text","content":"NEW"}]
            }
        ]
    });

    fix_orphan_container_contrast(&mut wrap, Some(&Value::Null));
    assert!(
        wrap.get("fill").is_none(),
        "a full-bleed painted card remains the wrapper surface even with an overlay badge"
    );
    assert!(
        wrap.get("effects").is_none(),
        "the transparent wrapper must not receive a ghost shadow"
    );
}

#[test]
fn orphan_card_with_only_small_filled_controls_still_gets_surface() {
    // 0724-1-gm's front vocabulary card has three structural rows. A tiny tag
    // and audio button paint their own fills, but neither is the card surface;
    // their presence must not leave the outer rounded card transparent.
    let mut card = json!({
        "type":"frame","role":"card","name":"Front Card","cornerRadius":20,
        "children":[
            {"type":"frame","name":"Front Top Row","children":[
                {"type":"icon_font","iconFontName":"bookmark"}
            ]},
            {"type":"frame","name":"Tag Pill",
             "fill":[{"type":"solid","color":"#F3F4F6"}],
             "children":[{"type":"text","content":"今日核心词"}]},
            {"type":"frame","name":"Front Center Block","children":[
                {"type":"text","content":"Resilient"}
            ]},
            {"type":"frame","role":"button","name":"Audio Button",
             "fill":[{"type":"solid","color":"#FF6B6B"}],
             "children":[{"type":"icon_font","iconFontName":"volume-2"}]},
            {"type":"frame","name":"Front Bottom Row","children":[
                {"type":"text","content":"adj. 保持韧性的；有适应力的"}
            ]}
        ]
    });

    fix_orphan_container_contrast(&mut card, Some(&Value::Null));
    assert_eq!(
        card["fill"],
        json!([{"type":"solid","color":"$--card"}]),
        "small filled descendants are controls, not a replacement card surface"
    );
    assert!(
        card["effects"].is_array(),
        "the restored card surface should carry the standard elevation"
    );
}

#[test]
fn padded_flex_wrapper_does_not_treat_fill_child_as_full_bleed() {
    let mut card = json!({
        "type":"frame","role":"card","name":"Padded Card",
        "width":320,"height":180,"layout":"vertical","padding":16,"cornerRadius":16,
        "children":[{
            "type":"frame","role":"card","name":"Inset Content Card",
            "width":"fill_container","height":"fill_container",
            "fill":[{"type":"solid","color":"#F97316"}],
            "children":[{"type":"text","content":"Inset content"}]
        }]
    });

    fix_orphan_container_contrast(&mut card, Some(&Value::Null));
    assert_eq!(
        card["fill"],
        json!([{"type":"solid","color":"$--card"}]),
        "fill_container spans the padded content box, not the outer card bounds"
    );
}

#[test]
fn orphan_card_keeps_authored_effects_when_surface_is_restored() {
    let authored_effects = json!([
        {
            "type":"shadow","offsetX":0,"offsetY":8,"blur":24,
            "spread":0,"color":"#FF6B6B33"
        }
    ]);
    let mut card = json!({
        "type":"frame","role":"card","name":"Front Card",
        "width":345,"height":148,"cornerRadius":20,
        "stroke":{"thickness":1,"fill":[{"type":"solid","color":"#E5E5E5"}]},
        "effects": authored_effects.clone(),
        "children":[
            {
                "type":"frame","name":"Front Top Row",
                "width":"fill_container","height":"fit_content",
                "children":[{
                    "type":"frame","name":"Tag Pill",
                    "width":"fit_content","height":"fit_content",
                    "fill":[{"type":"solid","color":"#F3F4F6"}]
                }]
            },
            {
                "type":"frame","name":"Front Center Block",
                "width":"fill_container","height":"fit_content",
                "children":[{
                    "type":"frame","role":"button","name":"Audio Button",
                    "width":36,"height":36,
                    "fill":[{"type":"solid","color":"#FF6B6B"}]
                }]
            }
        ]
    });

    fix_orphan_container_contrast(&mut card, Some(&Value::Null));
    assert_eq!(card["fill"], json!([{"type":"solid","color":"$--card"}]));
    assert_eq!(
        card["effects"], authored_effects,
        "restoring the missing card fill must preserve authored elevation"
    );
}

#[test]
fn orphan_card_preserves_explicit_empty_effects() {
    let mut card = json!({
        "type":"frame","role":"card","cornerRadius":12,"effects":[],
        "children":[{"type":"text","content":"Flat card"}]
    });

    fix_orphan_container_contrast(&mut card, Some(&Value::Null));
    assert_eq!(card["fill"], json!([{"type":"solid","color":"$--card"}]));
    assert_eq!(
        card["effects"],
        json!([]),
        "an explicit flat-card effect choice must not be replaced by default elevation"
    );
}

// ── orphaned-shadow strip (ghost-box cleanup) ────────────────────────────

#[test]
fn orphaned_shadow_stripped_from_fill_less_frame() {
    // A drop-shadow with no surface (no fill, no stroke) is a gray ghost box.
    let mut node = json!({
        "type":"frame","cornerRadius":12,
        "effects":[{"type":"shadow","offsetX":0,"offsetY":1,"blur":3,"spread":0,"color":"#0000001A"}],
        "children":[{"type":"frame","role":"card","fill":[{"type":"solid","color":"#FB923C"}]}]
    });
    fix_surface_color_discipline(&mut node, false);
    assert!(
        node.get("effects").is_none(),
        "shadow on a fill-less, stroke-less frame is stripped"
    );
}

#[test]
fn shadow_kept_when_node_has_visible_fill() {
    // A real card (visible fill) keeps its elevation shadow.
    let mut node = json!({
        "type":"frame","role":"card","cornerRadius":12,
        "fill":[{"type":"solid","color":"#FFFFFF"}],
        "effects":[{"type":"shadow","offsetX":0,"offsetY":1,"blur":3,"spread":0,"color":"#0000001A"}],
    });
    fix_surface_color_discipline(&mut node, false);
    assert!(node["effects"].is_array(), "a filled card keeps its shadow");
}

// ── fixStructuralWrapperTransparency ─────────────────────────────────────

#[test]
fn structural_wrapper_white_fill_stripped() {
    // A section wrapper that glm gave an explicit white fill → forced transparent.
    let mut sect = json!({
        "type":"frame","role":"section","fill":[{"type":"solid","color":"#FFFFFF"}],
        "children":[{"type":"text","content":"x"}]
    });
    fix_structural_wrapper_transparency(&mut sect);
    assert_eq!(
        sect["fill"],
        json!([]),
        "white structural wrapper → transparent"
    );
}

#[test]
fn structural_wrapper_keeps_card_fill() {
    // card is in CARD_LIKE_ALLOWLIST — an intentional surface, fill stays.
    let mut card = json!({
        "type":"frame","role":"card","fill":[{"type":"solid","color":"#FFFFFF"}],
        "children":[{"type":"text","content":"x"}]
    });
    fix_structural_wrapper_transparency(&mut card);
    assert_eq!(
        card["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}]),
        "card keeps its white surface"
    );
}

#[test]
fn redundant_colored_wrapper_strips_light_surface() {
    // feature-card with a $--card fill wrapping a single full-bleed
    // gradient banner child → its surface is a redundant box, strip it.
    let mut wrap = json!({
        "type":"frame","role":"feature-card","cornerRadius":12,
        "fill":[{"type":"solid","color":"$--card"}],
        "children":[
            {
                "type":"frame","role":"card","width":"fill_container",
                "fill":[{"type":"linear_gradient","angle":135,"stops":[
                    {"offset":0.0,"color":"$--chart-6"},{"offset":1.0,"color":"#FB923C"}]}],
                "children":[{"type":"text","content":"50% Off"}]
            },
            {"type":"text","content":"-30%"}
        ]
    });
    fix_structural_wrapper_transparency(&mut wrap);
    assert_eq!(
        wrap["fill"],
        json!([]),
        "redundant wrapper around a full-bleed colored card → transparent"
    );
}

#[test]
fn card_with_small_colored_child_keeps_fill() {
    // A real card whose colored child is NOT full-bleed (a small icon tile) is
    // a genuine surface — must keep its fill.
    let mut card = json!({
        "type":"frame","role":"card","fill":[{"type":"solid","color":"#FFFFFF"}],
        "children":[
            {
                "type":"frame","role":"icon-tile","width":48,
                "fill":[{"type":"solid","color":"#FB923C"}],
                "children":[]
            },
            {"type":"text","content":"Title"}
        ]
    });
    fix_structural_wrapper_transparency(&mut card);
    assert_eq!(
        card["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}]),
        "card with a small colored tile keeps its surface"
    );
}

#[test]
fn structural_wrapper_keeps_dark_fill() {
    // A dark section fill (luminance well below 0.85) is a deliberate band → kept.
    let mut sect = json!({
        "type":"frame","role":"section","fill":[{"type":"solid","color":"#1A1A1A"}],
        "children":[{"type":"text","content":"x"}]
    });
    fix_structural_wrapper_transparency(&mut sect);
    assert_eq!(
        sect["fill"],
        json!([{"type":"solid","color":"#1A1A1A"}]),
        "dark structural band is intentional, not stripped"
    );
}

#[test]
fn structural_wrapper_section_substring_and_header_stripped() {
    // role *containing* "section" (e.g. a custom "feature-section") → stripped.
    let mut feat = json!({
        "type":"frame","role":"feature-section","fill":[{"type":"solid","color":"#FAFAFA"}],
        "children":[{"type":"text"}]
    });
    fix_structural_wrapper_transparency(&mut feat);
    assert_eq!(feat["fill"], json!([]), "*-section role → transparent");
    // header is structural too.
    let mut header = json!({
        "type":"frame","role":"header","fill":[{"type":"solid","color":"#FFFFFF"}],
        "children":[{"type":"text"}]
    });
    fix_structural_wrapper_transparency(&mut header);
    assert_eq!(header["fill"], json!([]), "header → transparent");
}

#[test]
fn structural_wrapper_surface_variable_ref_stripped() {
    // glm emits UNRESOLVED $color-* refs (variable binding runs after post-pass),
    // so hex_luminance can't read them — the strip must match neutral surface
    // tokens by name. This is the actual header-background bug: role navbar +
    // fill $--muted was surviving because luminance("$--muted")
    // returned None.
    let mut header = json!({
        "type":"frame","role":"navbar","fill":[{"type":"solid","color":"$--muted"}],
        "children":[{"type":"text"}]
    });
    fix_structural_wrapper_transparency(&mut header);
    assert_eq!(
        header["fill"],
        json!([]),
        "navbar with $--muted → transparent"
    );
    // A colored token (deliberate accent band) is NOT a neutral surface → kept.
    let mut band = json!({
        "type":"frame","role":"section","fill":[{"type":"solid","color":"$--primary"}],
        "children":[{"type":"text"}]
    });
    fix_structural_wrapper_transparency(&mut band);
    assert_eq!(
        band["fill"],
        json!([{"type":"solid","color":"$--primary"}]),
        "deliberate colored band kept"
    );
}

#[test]
fn structural_wrapper_strips_border_with_fill() {
    // The mobile header (role navbar) carried a $--card fill AND a bottom
    // hairline stroke — the user flagged BOTH the background and the border.
    // Stripping the surface fill must drop the accompanying border too; a
    // transparent structural wrapper keeps no card/bar chrome.
    let mut header = json!({
        "type":"frame","role":"navbar","fill":[{"type":"solid","color":"$--card"}],
        "stroke":{"thickness":[0,0,1,0],"fill":[{"type":"solid","color":"$--border"}]},
        "children":[{"type":"text"}]
    });
    fix_structural_wrapper_transparency(&mut header);
    assert_eq!(header["fill"], json!([]), "surface fill stripped");
    assert!(
        header.get("stroke").is_none(),
        "bottom border stripped alongside the fill"
    );
}

#[test]
fn structural_wrapper_non_structural_role_left_alone() {
    // A plain content role that isn't structural and isn't card-like → untouched.
    let mut text = json!({
        "type":"text","role":"body","fill":[{"type":"solid","color":"#FFFFFF"}]
    });
    fix_structural_wrapper_transparency(&mut text);
    assert_eq!(
        text["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}]),
        "non-structural role is out of scope"
    );
}

// ── fixSurfaceColorDiscipline ────────────────────────────────────────────

#[test]
fn state_bg_token_on_input_recolored_to_neutral() {
    // The pink-search bug: glm used $--color-error as the input surface.
    let mut input = json!({
        "type":"text_input","name":"Search Input",
        "fill":[{"type":"solid","color":"$--color-error"}]
    });
    fix_surface_color_discipline(&mut input, false);
    assert_eq!(
        input["fill"],
        json!([{"type":"solid","color":"$--muted"}]),
        "danger-bg misused as input surface → neutral muted"
    );
}

#[test]
fn state_bg_token_kept_on_status_element() {
    // A real status element (name says "Error") legitimately uses danger-bg.
    let mut badge = json!({
        "type":"frame","role":"badge","name":"Error Badge",
        "fill":[{"type":"solid","color":"$--color-error"}],
        "children":[{"type":"text","content":"Failed"}]
    });
    fix_surface_color_discipline(&mut badge, false);
    assert_eq!(
        badge["fill"],
        json!([{"type":"solid","color":"$--color-error"}]),
        "status element keeps its semantic state color"
    );
}

#[test]
fn page_bg_token_stripped_from_inner_node_kept_on_root() {
    // Inner wrapper repainting the page bg (the cool grey panel behind search).
    let mut root = json!({
        "type":"frame","name":"Page","fill":[{"type":"solid","color":"$--background"}],
        "children":[
            {"type":"frame","name":"Search & Categories",
             "fill":[{"type":"solid","color":"$--background"}],
             "children":[{"type":"text_input","name":"Search"}]}
        ]
    });
    fix_surface_color_discipline(&mut root, true);
    assert_eq!(
        root["fill"],
        json!([{"type":"solid","color":"$--background"}]),
        "page root keeps the page-bg token"
    );
    assert_eq!(
        root["children"][0]["fill"],
        json!([]),
        "inner wrapper using the page-bg token → transparent"
    );
}

#[test]
fn page_bg_token_kept_when_root_paints_a_different_ground() {
    // 0808-gm-1.op: the page root paints a literal `#0A0A0A` while two of its
    // sections paint `$--background` (#0F172A) — a deliberate darker band,
    // NOT a repaint of the root's own ground. The strip is a redundancy
    // repair, so with nothing to be redundant WITH it must not fire.
    let mut root = json!({
        "type":"frame","name":"Page","fill":[{"type":"solid","color":"#0A0A0A"}],
        "children":[
            {"type":"frame","name":"Interactive Showcase",
             "fill":[{"type":"solid","color":"$--background"}],
             "children":[{"type":"text","content":"x"}]}
        ]
    });
    fix_surface_color_discipline(&mut root, true);
    assert_eq!(
        root["children"][0]["fill"],
        json!([{"type":"solid","color":"$--background"}]),
        "a band the root does not repeat is a surface, not a redundant repaint"
    );
}

// ── fixSectionAlternation ────────────────────────────────────────────────

#[test]
fn section_alternation_paints_unfilled_runs() {
    let mut col = json!({
        "type":"frame","layout":"vertical","children":[
            {"type":"frame","role":"section","children":[]},
            {"type":"frame","role":"section","children":[]},
            {"type":"frame","role":"section","children":[]}
        ]
    });
    fix_section_alternation(&mut col);
    assert_eq!(
        col["children"][0]["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}])
    );
    assert_eq!(
        col["children"][1]["fill"],
        json!([{"type":"solid","color":"#F8FAFC"}])
    );
    assert_eq!(
        col["children"][2]["fill"],
        json!([{"type":"solid","color":"#FFFFFF"}])
    );
}

#[test]
fn section_alternation_skipped_on_dark_parent() {
    let mut col = json!({
        "type":"frame","layout":"vertical","fill":[{"type":"solid","color":"#0F172A"}],
        "children":[
            {"type":"frame","role":"section","children":[]},
            {"type":"frame","role":"section","children":[]},
            {"type":"frame","role":"section","children":[]}
        ]
    });
    fix_section_alternation(&mut col);
    assert!(
        col["children"][0].get("fill").is_none(),
        "dark page: no white strips"
    );
}
