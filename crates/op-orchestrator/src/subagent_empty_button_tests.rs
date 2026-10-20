//! Empty text-role buttons must not reach the canvas (img-a1-travel rep-2:
//! the booking CTA rendered as a bare painted block with no label).

use super::*;

#[test]
fn travel_booking_bar_empty_cta_button_is_rejected() {
    // Real artifact from the img-a1-travel rep-2 booking screen: the fixed
    // bottom bar's "Confirm booking" CTA came out as a painted, childless
    // frame next to the total text.
    let node: PenNode = serde_json::from_value(serde_json::json!({
        "type":"frame","id":"bar","name":"Booking Bar","layout":"horizontal","width":375,
        "children":[
            {"type":"text","id":"total","name":"Total","content":"Total $877","fontSize":16},
            {"type":"frame","id":"cta","name":"Confirm Booking Button","role":"button",
             "width":172,"height":48,"layout":"horizontal",
             "justifyContent":"center","alignItems":"center",
             "fill":[{"type":"solid","color":"#EA580C"}],"children":[]}
        ]
    }))
    .unwrap();
    let reason = empty_controls::rejection(&[node]).unwrap();
    assert!(reason.starts_with("self-check failed: "), "{reason}");
    assert!(reason.contains("empty-button"), "{reason}");
    assert!(reason.contains("Confirm Booking Button"), "{reason}");
    assert!(crate::retry::is_self_check_rejection(&reason));
}

#[test]
fn buttons_with_labels_icons_image_fills_or_no_pixels_pass_the_gate() {
    for extras in [
        r#""children":[{"type":"text","id":"label","content":"Confirm booking"}]"#,
        r#""children":[{"type":"icon_font","id":"glyph","iconFontName":"check"}]"#,
        r#""fill":[{"type":"image","url":"asset:confirm"}]"#,
        r#""visible":false"#,
    ] {
        let node: PenNode = serde_json::from_str(&format!(
            r#"{{"type":"frame","id":"cta","role":"button",{extras}}}"#
        ))
        .unwrap();
        assert!(empty_controls::rejection(&[node]).is_none(), "{extras}");
    }
}

#[test]
fn empty_role_button_scripts_are_rejected_before_insert() {
    let script = r##"const bar=I(null,{type:"frame",name:"Booking Bar",layout:"horizontal",width:375});
I(bar,{type:"text",content:"Total $877",fontSize:16});
I(bar,{type:"frame",name:"Confirm Booking Button",role:"button",width:172,height:48,layout:"horizontal",justifyContent:"center",alignItems:"center",fill:[{type:"solid",color:"#EA580C"}]});"##;
    let llm = ScriptedLlm::new(vec![ScriptResponse::Text(script.into())]);
    let mut sink = VecDocSink::new();
    let outcome = block_on(run_subtask(
        &subtask(),
        &plan(),
        &req(),
        &llm,
        &mut sink,
        &AbortFlag::new(),
        false,
        false,
    ));
    let error = outcome.error.unwrap();
    assert!(
        error.contains("empty-button") && error.contains("Confirm Booking Button"),
        "{error}"
    );
    assert!(crate::retry::is_self_check_rejection(&error));
    assert_eq!(outcome.node_count, 0);
    assert!(!sink
        .applied
        .iter()
        .any(|c| matches!(c, EditorCommand::InsertSubtree { .. })));
}
