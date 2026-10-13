use super::*;

fn fitness_screen() -> Value {
    let cards: Vec<_> = [210, 132, 132, 132, 132, 132]
        .into_iter()
        .enumerate()
        .map(|(index, width)| {
            let image_height = if index == 0 { 108 } else { 74 };
            json!({"type":"frame","id":format!("card-{index}"),"width":width,
            "height":"fit_content","layout":"vertical","gap":8,"clipContent":true,
            "fill":[{"type":"solid","color":"#FFFFFF"}],"children":[
                {"type":"image","id":format!("cover-{index}"),"src":"",
                 "width":width,"height":image_height},
                {"type":"text","id":format!("label-{index}"),"content":"课程",
                 "width":"fill_container","height":"fit_content","fontSize":12}
            ]})
        })
        .collect();
    json!({"type":"frame","id":"root","width":375,"height":812,"layout":"vertical",
    "children":[
        {"type":"frame","id":"status","role":"status-bar","height":44},
        {"type":"frame","id":"section","width":"fill_container","height":"fit_content",
         "layout":"vertical","gap":12,"children":[
            {"type":"frame","id":"header","layout":"horizontal",
             "width":"fill_container","height":"fit_content","padding":[0,24],
             "children":[text("title","推荐课程")]},
            {"type":"frame","id":"viewport","layout":"vertical",
             "width":"fill_container","height":"fit_content","clipContent":true,"children":[
                {"type":"frame","id":"lane","layout":"horizontal","width":"fit_content",
                 "height":"fit_content","padding":[0,0,4,24],"gap":12,"children":cards}
             ]}
         ]},
        {"type":"frame","id":"nav","role":"bottom-tab-bar","height":72}
    ]})
}

#[test]
fn vertical_carousel_keeps_one_leading_rail_and_authored_card_sizes() {
    let mut sink = insert(fitness_screen());
    repair_mobile_content_rails_for_all_roots(&mut sink);
    assert!(node_json(&sink, "section").get("padding").is_none());
    assert_eq!(node_json(&sink, "header")["padding"], json!([0.0, 24.0]));
    assert_eq!(
        node_json(&sink, "viewport")["padding"],
        json!([0.0, 0.0, 0.0, 24.0])
    );
    assert_eq!(
        node_json(&sink, "lane")["padding"],
        json!([0.0, 0.0, 4.0, 0.0])
    );

    // Later lint/spacing and real-layout geometry passes must agree with the
    // structural repair instead of adding a second inset or equalizing cards.
    let mut root = find(&sink.state.active_children()[0], "root")
        .unwrap()
        .clone();
    assert!(op_design_lint::detect_edge_section_padding(&root).is_empty());
    crate::spacing_repair::strip_wrapper_double_inset(&mut root);
    assert_eq!(
        serde_json::to_value(&root).unwrap(),
        node_json(&sink, "root")
    );
    crate::geometry_validation::geometry_validate_and_fix(&mut sink, "root");
    for (index, width) in [210, 132, 132, 132, 132, 132].into_iter().enumerate() {
        assert_eq!(
            node_json(&sink, &format!("card-{index}"))["width"].as_f64(),
            Some(f64::from(width))
        );
        assert_eq!(
            node_json(&sink, &format!("cover-{index}"))["width"].as_f64(),
            Some(f64::from(width))
        );
        assert_eq!(
            node_json(&sink, &format!("cover-{index}"))["height"].as_f64(),
            Some(if index == 0 { 108.0 } else { 74.0 })
        );
    }
    sink.applied.clear();
    repair_mobile_content_rails_for_all_roots(&mut sink);
    assert!(sink.applied.is_empty(), "rail ownership must be idempotent");
}

#[test]
fn vertical_container_with_fill_row_still_gets_content_inset() {
    let mut screen = fitness_screen();
    screen["children"][1]["children"][1]["children"][0]["width"] = json!("fill_container");
    let mut sink = insert(screen);
    repair_mobile_content_rails_for_all_roots(&mut sink);
    assert_eq!(
        node_json(&sink, "section")["padding"],
        json!([0.0, 24.0, 0.0, 24.0])
    );
}
