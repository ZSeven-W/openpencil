//! Regression tests for layout repair passes layered on top of jian/taffy.

use super::super::editor_state_to_layout_scene;
use super::state_from;

#[test]
fn vertical_centered_text_child_infers_center_text_align_like_ts_layout() {
    // TS layout parity: a text child in a vertical flex container with
    // alignItems:center is centered inside its own text box even when
    // textAlign is omitted. Bottom-tab labels in pencil-demo.op rely on this.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"tab","width":88,"height":57,
         "layout":"vertical","gap":4,"padding":[8,16],"alignItems":"center",
         "children":[
           {"type":"text","id":"label","width":56,"height":13,
            "content":"Home","fontFamily":"DM Sans","fontSize":11,"fontWeight":600}
         ]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let label = scene.pages[0].find("label").expect("label text");

    assert_eq!(
        label.text_align,
        jian_scene::layout_scene::SceneTextAlign::Center
    );
}

#[test]
fn implicit_horizontal_layout_applies_padding_to_children_like_ts_layout() {
    // TS `inferLayout` treats a frame with padding as horizontal auto-layout.
    // Badge labels in pencil-demo.op rely on that padding even though the
    // frame has no explicit `layout` field and the child carries stale x/y=0.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"badge","width":100,"height":29,
         "padding":[6,10],
         "children":[
           {"type":"text","id":"label","x":0,"y":0,"width":47,"height":13,
            "content":"21 days","fontSize":11}
         ]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let label = scene.pages[0].find("label").expect("label text");

    assert_eq!(label.bounds.origin.x, 10.0);
    assert_eq!(label.bounds.origin.y, 6.0);
}

#[test]
fn implicit_horizontal_row_places_nested_text_after_status_icon() {
    // Legacy mobile rows omit `layout:"horizontal"` but still carry
    // padding/gap/alignItems. The nested text column must start after the
    // status icon plus gap; otherwise the icon lands on top of the meta text.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"row","width":354,"height":84,
         "gap":14,"padding":18,"alignItems":"center",
         "children":[
           {"type":"ellipse","id":"status","x":18,"y":28,
            "width":28,"height":28},
           {"type":"frame","id":"content","x":60,"y":18,
            "width":276,"height":48,"layout":"vertical","gap":4,
            "children":[
              {"type":"text","id":"title","x":0,"y":0,
               "width":138,"height":18,"content":"Morning meditation",
               "fontSize":16},
              {"type":"text","id":"meta","x":0,"y":22,
               "width":136,"height":15,"content":"10 min · Completed 06:45",
               "fontSize":13}
            ]}
         ]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let status = scene.pages[0].find("status").expect("status icon");
    let content = scene.pages[0].find("content").expect("content column");
    let title = scene.pages[0].find("title").expect("title text");
    let meta = scene.pages[0].find("meta").expect("meta text");

    assert_eq!(status.bounds.origin.x, 18.0);
    assert_eq!(content.bounds.origin.x, 60.0);
    assert_eq!(title.bounds.origin.x, 60.0);
    assert_eq!(meta.bounds.origin.x, 60.0);
}

#[test]
fn overflowing_fixed_width_horizontal_wrapper_expands_before_parent_space_between() {
    // Desktop transaction rows generated a fixed-width right-side wrapper
    // from stale content estimates. Its visible children are wider than the
    // wrapper, so the parent space-between row must use the expanded child
    // width or the amount column renders outside the table border.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"item","width":360,"height":60,
         "padding":[10,20],"justifyContent":"space_between","alignItems":"center",
         "children":[
           {"type":"frame","id":"left","x":20,"y":20,
            "width":60,"height":20},
           {"type":"frame","id":"right","x":220,"y":19,
            "width":120,"height":22,"gap":20,"alignItems":"center",
            "children":[
              {"type":"frame","id":"badge","x":0,"y":0,
               "width":100,"height":22,
               "children":[{"type":"text","id":"badge_text",
                "x":0,"y":0,"width":14,"height":10,"content":"OK"}]},
              {"type":"text","id":"amount","x":0,"y":4,
               "width":100,"height":14,"content":"+$2,400.00",
               "textAlign":"right"}
            ]}
         ]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let right = scene.pages[0].find("right").expect("right wrapper");
    let amount = scene.pages[0].find("amount").expect("amount text");

    assert_eq!(right.bounds.origin.x, 120.0);
    assert_eq!(right.bounds.size.x, 220.0);
    assert_eq!(amount.bounds.origin.x, 240.0);
    assert_eq!(amount.bounds.origin.x + amount.bounds.size.x, 340.0);
}

#[test]
fn legacy_fixed_width_text_badge_centers_label_like_badge_builder() {
    // Older generated `.op` files used fixed-width Badge frames with
    // padding but omitted the badge builder's explicit layout/align
    // fields. They should still render as a centered pill label, not
    // as stale child x/y at the top-left.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"badge","name":"Badge","width":100,"height":29,
         "padding":[6,10],
         "children":[
           {"type":"text","id":"label","x":0,"y":0,"width":47,"height":13,
            "content":"21 days","fontSize":11}
         ]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let label = scene.pages[0].find("label").expect("label text");

    assert!(
        (label.bounds.origin.x - 26.5).abs() < 0.01,
        "label x should be centered in the fixed-width badge, got {}",
        label.bounds.origin.x
    );
    assert_eq!(label.bounds.origin.y, 8.0);
}

#[test]
fn non_clipped_fixed_height_layout_containers_expand_to_overflowing_children() {
    // Legacy generated files sometimes persisted numeric heights from an
    // optimistic text estimate. When the resolved child bounds are taller,
    // open containers should grow and parent stacks should reflow instead
    // of letting the final line cross the card border.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"root","width":200,"height":200,
         "layout":"vertical","gap":10,
         "children":[
          {"type":"frame","id":"row","width":100,"height":20,
           "children":[
            {"type":"frame","id":"stack","x":0,"y":0,"width":100,"height":20,
             "layout":"vertical","gap":5,
             "children":[
              {"type":"rectangle","id":"a","x":0,"y":0,"width":10,"height":18},
              {"type":"rectangle","id":"b","x":0,"y":0,"width":10,"height":18}
             ]}
           ]},
          {"type":"rectangle","id":"after","width":100,"height":10}
         ]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let root = &scene.pages[0].children[0];
    let row = root.find("row").expect("row frame");
    let stack = root.find("stack").expect("stack frame");
    let b = root.find("b").expect("second stack child");
    let after = root.find("after").expect("following sibling");

    assert_eq!(b.bounds.origin.y, 23.0);
    assert_eq!(stack.bounds.size.y, 41.0);
    assert_eq!(row.bounds.size.y, 41.0);
    assert_eq!(after.bounds.origin.y, 51.0);
}

#[test]
fn clipped_vertical_root_expands_when_repaired_content_overflows() {
    // Top-level artboards still clip at paint time, but generated legacy roots
    // can receive taller repaired children after exact text/layout measurement.
    // The root bounds must grow with that repaired stack or the bottom nav is
    // clipped out of the exported artboard.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"root","width":200,"height":50,"clip":true,
         "layout":"vertical","gap":10,
         "children":[
          {"type":"frame","id":"row","width":100,"height":20,
           "children":[
            {"type":"frame","id":"stack","x":0,"y":0,"width":100,"height":20,
             "layout":"vertical","gap":5,
             "children":[
              {"type":"rectangle","id":"a","x":0,"y":0,"width":10,"height":18},
              {"type":"rectangle","id":"b","x":0,"y":0,"width":10,"height":18}
             ]}
           ]},
          {"type":"rectangle","id":"after","width":100,"height":10}
         ]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let root = &scene.pages[0].children[0];
    let after = root.find("after").expect("following child");

    assert_eq!(after.bounds.origin.y, 51.0);
    assert_eq!(root.bounds.size.y, 61.0);
    assert!(root.clip_content, "root must still clip after expanding");
}

#[test]
fn nested_clipped_fixed_height_container_does_not_expand_to_overflowing_children() {
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"root","width":200,"height":100,
         "layout":"vertical",
         "children":[
          {"type":"frame","id":"card","width":100,"height":20,"clip":true,
           "children":[
            {"type":"frame","id":"stack","x":0,"y":0,"width":100,"height":20,
             "layout":"vertical","gap":5,
             "children":[
              {"type":"rectangle","id":"a","x":0,"y":0,"width":10,"height":18},
              {"type":"rectangle","id":"b","x":0,"y":0,"width":10,"height":18}
             ]}
           ]}
         ]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let root = &scene.pages[0].children[0];
    let card = root.find("card").expect("clipped card");

    assert_eq!(card.bounds.size.y, 20.0);
    assert!(card.clip_content);
}

#[test]
fn inferred_horizontal_metric_row_stretches_filled_cards_to_parent_height() {
    // Bauhaus metrics rows were generated with `gap` but without
    // `layout:"horizontal"`. The visual row height belongs to the card
    // children; otherwise one metric card stops above the next section.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"metrics","width":354,"height":185,
         "gap":16,
         "children":[
          {"type":"frame","id":"metric1","name":"metric1_15",
           "width":169,"height":185,
           "fill":[{"type":"solid","color":"#E53935"}]},
          {"type":"frame","id":"metric2","name":"metric2_15",
           "x":185,"y":0,"width":169,"height":161,
           "fill":[{"type":"solid","color":"#000000"}]}
         ]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let metrics = &scene.pages[0].children[0];
    let metric1 = metrics.find("metric1").expect("first metric");
    let metric2 = metrics.find("metric2").expect("second metric");

    assert_eq!(metric1.bounds.size.y, 185.0);
    assert_eq!(metric2.bounds.size.y, 185.0);
}

#[test]
fn vertical_space_between_reflows_children_like_ts_layout() {
    // Some legacy metric footer frames persist all text children at y=0
    // and rely on vertical auto-layout with justifyContent:space_between
    // to separate the label from the badge/change text.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"footer","width":117,"height":33,
         "layout":"vertical","justifyContent":"space_between",
         "children":[
          {"type":"text","id":"label","x":0,"y":0,"width":52,"height":13.56,
           "content":"Day streak","fontSize":12},
          {"type":"text","id":"badge","x":0,"y":0,"width":53,"height":11.3,
           "content":"Personal best","fontSize":10}
         ]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let badge = scene.pages[0].find("badge").expect("badge text");

    assert_eq!(badge.bounds.origin.y, 22.0);
}

#[test]
fn split_metric_column_insets_content_after_divider() {
    // Swiss Expressive metric rows use a right-side divider on the first
    // column. Legacy files omit padding on the second column, but the visual
    // design needs the second metric content inset from that divider.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"metrics","width":354,"height":97,
         "children":[
          {"type":"frame","id":"metric1","name":"metric1_2",
           "x":0,"y":0,"width":177,"height":97,
           "layout":"vertical","gap":6,"padding":[0,24,0,0],
           "stroke":{"thickness":{"right":1}},
           "children":[
            {"type":"text","id":"left_value","x":0,"y":0,
             "width":52,"height":59,"content":"47","fontSize":52}
           ]},
          {"type":"frame","id":"metric2","name":"metric2_2",
           "x":177,"y":0,"width":177,"height":97,
           "layout":"vertical","gap":6,
           "children":[
            {"type":"text","id":"right_value","x":0,"y":0,
             "width":117,"height":59,"content":"2,847","fontSize":52}
           ]}
         ]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let metric1 = scene.pages[0].find("metric1").expect("first metric");
    let value = scene.pages[0]
        .find("right_value")
        .expect("right metric value");
    let divider_x = metric1.bounds.origin.x + metric1.bounds.size.x;

    assert_eq!(divider_x, 177.0);
    assert_eq!(value.bounds.origin.x, 201.0);
}

#[test]
fn legacy_habit_rows_keep_checkbox_in_gutter_and_text_after_gap() {
    // Swiss Expressive habit rows encode vertical padding as `[16,0]` and
    // omit explicit horizontal layout. The checkbox belongs in the left
    // gutter at x=0; only the text column should follow checkbox + gap.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"list","width":354,"height":75,
         "layout":"vertical",
         "children":[
          {"type":"frame","id":"row","name":"habit1_2",
           "x":0,"y":0,"width":354,"height":75,
           "gap":16,"padding":[16,0],"alignItems":"center",
           "stroke":{"thickness":{"top":1}},
           "children":[
            {"type":"rectangle","id":"status","x":0,"y":28,
             "width":20,"height":20},
            {"type":"frame","id":"content","x":36,"y":16,
             "width":318,"height":43,"layout":"vertical","gap":2,
             "children":[
              {"type":"text","id":"title","x":0,"y":0,
               "width":130,"height":17,"content":"Morning meditation"}
             ]}
           ]}
         ]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let status = scene.pages[0].find("status").expect("status square");
    let content = scene.pages[0].find("content").expect("content column");
    let title = scene.pages[0].find("title").expect("title");

    assert_eq!(status.bounds.origin.x, 0.0);
    assert_eq!(content.bounds.origin.x, 36.0);
    assert_eq!(content.bounds.size.x, 318.0);
    assert_eq!(title.bounds.origin.x, 36.0);
}

#[test]
fn legacy_percent_wrapper_places_symbol_at_wrapper_origin() {
    // Terminal Swiss uses a no-paint wrapper around the superscript `%`.
    // The authored child x was a stale absolute offset, so the percent sign
    // visually detaches from the large number unless the wrapper owns origin.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"row","width":206,"height":112,
         "gap":12,
         "children":[
          {"type":"text","id":"number","name":"heroNumber19",
           "x":0,"y":0,"width":124,"height":149.16,
           "content":"86","fontSize":132,"lineHeight":0.85},
          {"type":"frame","id":"percent","name":"heroPercent19",
           "x":170,"y":0,"width":36,"height":48,
           "padding":[0,4,0,0],"justifyContent":"space_between",
           "children":[
            {"type":"text","id":"symbol","name":"heroPercentSymbol19",
             "x":34,"y":0,"width":48,"height":54.24,
             "content":"%","fontSize":48}
           ]}
         ]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let percent = scene.pages[0].find("percent").expect("percent wrapper");
    let symbol = scene.pages[0].find("symbol").expect("percent symbol");

    assert_eq!(percent.bounds.origin.x, 136.0);
    assert_eq!(symbol.bounds.origin.x, percent.bounds.origin.x);
}

#[test]
fn bottom_stroked_chart_keeps_day_labels_above_bottom_border() {
    // Dashboard chart bars bottom-align their local Bar frames; the day label
    // then sits inside the Chart bottom stroke unless the chart reserves a
    // small gutter below the label row.
    let src = r##"{
      "version":"1.0.0","pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"chart","name":"Chart",
         "width":1048,"height":187,"gap":16,
         "stroke":{"thickness":{"bottom":1}},
         "children":[
          {"type":"frame","id":"bar","name":"Bar",
           "x":0,"y":70,"width":136,"height":117,
           "layout":"vertical","gap":10,
           "children":[
            {"type":"rectangle","id":"fill","name":"bar1Fill",
             "x":0,"y":5,"width":136,"height":90},
            {"type":"text","id":"label","name":"bar1Label",
             "x":0,"y":105,"width":136,"height":12.43,
             "content":"Mon","fontSize":11}
           ]}
         ]}
      ]}],"children":[]
    }"##;
    let scene = editor_state_to_layout_scene(&state_from(src));
    let chart = scene.pages[0].find("chart").expect("chart");
    let label = scene.pages[0].find("label").expect("day label");
    let label_bottom = label.bounds.origin.y + label.bounds.size.y;
    let chart_bottom = chart.bounds.origin.y + chart.bounds.size.y;

    assert!(
        chart_bottom - label_bottom >= 8.0,
        "chart bottom stroke should leave an 8px gutter below labels, got {}",
        chart_bottom - label_bottom
    );
}
