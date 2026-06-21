use super::*;

#[test]
fn legacy_clip_true_threads_to_clip_content_payload() {
    let src = r##"{
      "version":"1.0.0",
      "pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"root","width":240,"height":260,
         "children":[
           {"type":"frame","id":"card","width":180,"height":216,
            "clip":true,"cornerRadius":20,
            "children":[
              {"type":"rectangle","id":"hero","width":180,"height":100,
               "fill":[{"type":"solid","color":"#ffffff"}]}
            ]}
         ]}
      ]}],
      "children":[]
    }"##;
    let r = load(src);
    let card = &r.payload.pages[0].children[0].children[0];

    assert!(
        card.clip_content,
        "legacy clip:true must clip rounded cards"
    );
}

#[test]
fn sided_stroke_thickness_survives_payload_conversion() {
    let src = r##"{
      "version":"1.0.0",
      "pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"search","width":200,"height":36,
         "stroke":{"thickness":{"bottom":1},"fill":[{"type":"solid","color":"#000000"}]}}
      ]}],
      "children":[]
    }"##;
    let r = load(src);
    let stroke = r.payload.pages[0].children[0]
        .stroke
        .as_ref()
        .expect("stroke");
    let encoded = serde_json::to_value(stroke).expect("stroke json");

    assert_eq!(encoded["width"], 1.0);
    assert_eq!(encoded["sides"], serde_json::json!([0.0, 0.0, 1.0, 0.0]));
}

#[test]
fn legacy_status_bar_layout_shell_strokes_are_not_painted() {
    let src = r##"{
      "version":"1.0.0",
      "pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"status","name":"Status bar - iPhone",
         "width":402,"height":62,
         "children":[
           {"type":"frame","id":"time","name":"Time","x":24,"y":21,
            "width":100,"height":22,
            "stroke":{"align":"inside","thickness":1},
            "children":[
              {"type":"text","id":"clock","name":"Time","x":30,"y":3,
               "width":41,"height":19,"content":"9:41"}
            ]},
           {"type":"frame","id":"levels","name":"Levels","x":278,"y":21,
            "width":100,"height":22,
            "stroke":{"align":"inside","thickness":1},
            "children":[
              {"type":"frame","id":"battery","name":"Frame","x":61,"y":5,
               "width":27,"height":13,
               "stroke":{"align":"inside","thickness":1},
               "children":[
                 {"type":"rectangle","id":"battery_border","name":"Border",
                  "x":18,"y":0,"width":25,"height":13,
                  "stroke":{"align":"inside","thickness":1}},
                 {"type":"rectangle","id":"battery_capacity","name":"Capacity",
                  "x":20,"y":2,"width":21,"height":9,
                  "fill":[{"type":"solid","color":"#000000"}]}
               ]}
            ]}
         ]}
      ]}],
      "children":[]
    }"##;
    let r = load(src);
    let status = &r.payload.pages[0].children[0];
    let time = &status.children[0];
    let levels = &status.children[1];
    let battery = &levels.children[0];
    let battery_border = &battery.children[0];

    assert!(time.stroke.is_none(), "time shell stroke should be hidden");
    assert!(
        levels.stroke.is_none(),
        "levels shell stroke should be hidden"
    );
    assert!(
        battery.stroke.is_none(),
        "battery wrapper stroke should be hidden"
    );
    assert!(
        battery_border.stroke.is_some(),
        "actual battery outline stroke should still paint"
    );
}

#[test]
fn legacy_status_bar_shell_strokes_survive_gap_padding_layout_repair_hidden() {
    let src = r##"{
      "version":"1.0.0",
      "pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"status","name":"Status bar - iPhone",
         "width":402,"height":62,
         "gap":154,"padding":[21,24,19,24],
         "alignItems":"center","justifyContent":"center",
         "children":[
           {"type":"frame","id":"time","name":"Time","x":24,"y":21,
            "width":100,"height":22,
            "gap":10,"padding":[1.5,0,0,0],
            "alignItems":"center","justifyContent":"center",
            "stroke":{"align":"inside","thickness":1},
            "children":[
              {"type":"text","id":"clock","name":"Time","x":30,"y":3,
               "width":41,"height":19,"content":"9:41"}
            ]},
           {"type":"frame","id":"levels","name":"Levels","x":278,"y":21,
            "width":100,"height":22,
            "gap":7,"padding":[1,1,0,0],
            "alignItems":"center","justifyContent":"center",
            "stroke":{"align":"inside","thickness":1},
            "children":[
              {"type":"frame","id":"battery","name":"Frame","x":61,"y":5,
               "width":27.328037,"height":13,
               "stroke":{"align":"inside","thickness":1},
               "children":[
                 {"type":"rectangle","id":"battery_border","name":"Border",
                  "x":18,"y":0,"width":25,"height":13,
                  "stroke":{"align":"inside","thickness":1}},
                 {"type":"path","id":"battery_cap","name":"Cap",
                  "x":44,"y":4.5,"width":0,"height":0,
                  "geometry":"M0 0l0 4c0.8-0.3 1.3-1.1 1.3-2 0-0.9-0.5-1.7-1.3-2",
                  "fill":[{"type":"solid","color":"#000000"}]},
                 {"type":"rectangle","id":"battery_capacity","name":"Capacity",
                  "x":20,"y":2,"width":21,"height":9,
                  "fill":[{"type":"solid","color":"#000000"}]}
               ]}
            ]}
         ]}
      ]}],
      "children":[]
    }"##;
    let r = load(src);
    let status = &r.payload.pages[0].children[0];
    let time = &status.children[0];
    let levels = &status.children[1];
    let battery = &levels.children[0];

    assert!(time.stroke.is_none(), "time shell stroke should be hidden");
    assert!(
        levels.stroke.is_none(),
        "levels shell stroke should be hidden"
    );
    assert!(
        battery.stroke.is_none(),
        "battery wrapper stroke should stay hidden even after layout repair"
    );
    assert_eq!(battery.h, 13.0, "battery wrapper should not be stretched");
}

#[test]
fn legacy_help_expanded_row_side_strokes_do_not_overlap_parent_outline() {
    let src = r##"{
      "version":"1.0.0",
      "pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"help_list","name":"helpList2",
         "width":354,"height":266,
         "layout":"vertical",
         "stroke":{"thickness":2,"fill":[{"type":"solid","color":"#000000"}]},
         "children":[
           {"type":"frame","id":"help_1","name":"help1_2",
            "width":354,"height":56,"padding":16,
            "children":[{"type":"text","id":"help_1_text","content":"How do streaks work?"}]},
           {"type":"rectangle","id":"div_1","name":"helpDivider1_2",
            "width":354,"height":1,"fill":[{"type":"solid","color":"#E5E5E5"}]},
           {"type":"frame","id":"help_2","name":"help2_2",
            "width":354,"height":152,
            "layout":"vertical","gap":12,"padding":16,
            "fill":[{"type":"solid","color":"#F5F5F5"}],
            "stroke":{"align":"inside","thickness":{"right":2,"left":2},
                      "fill":[{"type":"solid","color":"#000000"}]},
            "children":[
              {"type":"frame","id":"help_2_header","name":"help2Header_2",
               "width":322,"height":24,
               "children":[{"type":"text","id":"help_2_title","content":"Setting up reminders"}]},
              {"type":"text","id":"help_2_body","name":"help2Content_2",
               "width":322,"height":63,
               "content":"Tap on any habit and select Remind me to set up custom notifications."}
            ]},
           {"type":"rectangle","id":"div_2","name":"helpDivider2_2",
            "width":354,"height":1,"fill":[{"type":"solid","color":"#E5E5E5"}]},
           {"type":"frame","id":"help_3","name":"help3_2",
            "width":354,"height":56,"padding":16,
            "children":[{"type":"text","id":"help_3_text","content":"Understanding your stats"}]}
         ]}
      ]}],
      "children":[]
    }"##;
    let r = load(src);
    let help_list = &r.payload.pages[0].children[0];
    let help_2 = &help_list.children[2];

    assert!(
        help_list.stroke.is_some(),
        "parent help list outline should stay visible"
    );
    assert!(
        help_2.stroke.is_none(),
        "expanded help row side strokes duplicate the parent outline"
    );
}

#[test]
fn legacy_dark_help_card_missing_stroke_fill_uses_divider_color() {
    let src = r##"{
      "version":"1.0.0",
      "pages":[{"id":"p","name":"P","children":[
        {"type":"frame","id":"help_card","name":"helpCard7",
         "width":354,"height":227,
         "fill":[{"type":"solid","color":"#1E2026"}],
         "stroke":{"thickness":1},
         "children":[
           {"type":"frame","id":"help_1","name":"help1_7",
            "width":354,"height":53,
            "children":[{"type":"text","id":"help_1_text","content":"How do streaks work?"}]}
         ]}
      ]}],
      "children":[]
    }"##;
    let r = load(src);
    let help_card = &r.payload.pages[0].children[0];
    let stroke = help_card.stroke.as_ref().expect("help card stroke");

    assert_eq!(stroke.color, [0.16470589, 0.16862746, 0.1882353, 1.0]);
}
