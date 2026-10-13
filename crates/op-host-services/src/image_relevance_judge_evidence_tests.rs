use super::*;

fn evidence_reply() -> serde_json::Value {
    let checks: Vec<_> = [
        "subject",
        "count",
        "required_object",
        "color_material",
        "appearance",
        "action",
        "background",
        "composition",
        "medium",
        "unrequested_brand",
        "watermark_ad",
        "forbidden_text",
    ]
    .into_iter()
    .map(|category| {
        if ["subject", "unrequested_brand", "watermark_ad"].contains(&category) {
            serde_json::json!([
                category,
                "brief requirement",
                "visible matching evidence",
                "match"
            ])
        } else {
            serde_json::json!([category, "", "", "not_required"])
        }
    })
    .collect();
    serde_json::json!({
        "schema_version": 2,
        "checks": checks,
        "appearance_contract": {"kind":"not_required", "source_quote":""},
        "aesthetic_preferences": [],
        "soft_deviations": [],
        "verdict": "on",
        "reason": "visible requirements match"
    })
}

fn result(reply: &serde_json::Value) -> Option<(RelevanceVerdict, String)> {
    parse_verdict_response(Some(&reply.to_string()), None)
}

#[test]
fn observed_conflict_excludes_candidate_even_when_verdict_calls_it_style() {
    // Real regression: the model described a conflicting physical condition
    // while calling the overall mismatch a permissible style deviation.
    for (category, requirement, observation) in [
        ("appearance", "smooth", "visibly rough"),
        ("action", "running", "standing still"),
        ("count", "single bottle", "three bottles"),
    ] {
        let mut reply = evidence_reply();
        let rows = reply["checks"].as_array_mut().unwrap();
        let row = rows.iter_mut().find(|row| row[0] == category).unwrap();
        *row = serde_json::json!([category, requirement, observation, "mismatch"]);
        reply["verdict"] = "weak".into();
        reply["soft_deviations"] = serde_json::json!(["style"]);
        let (verdict, reason) = result(&reply).unwrap();
        assert_eq!(verdict, RelevanceVerdict::Off);
        assert!(reason.contains(&format!("{category}:mismatch")));
        // Negative evidence remains exclusionary despite malformed siblings.
        reply["checks"][0] = serde_json::Value::Null;
        assert_eq!(result(&reply).unwrap().0, RelevanceVerdict::Off);
    }
}

#[test]
fn required_detail_unknown_is_not_a_weak_visual_match() {
    let mut reply = evidence_reply();
    reply["checks"][3] = serde_json::json!([
        "color_material",
        "glass bottle",
        "material hidden by opaque wrapper",
        "unknown"
    ]);
    reply["verdict"] = "on".into();
    let (verdict, reason) = result(&reply).unwrap();
    assert_eq!(verdict, RelevanceVerdict::Off);
    assert!(reason.contains("color_material:unknown"));
}

#[test]
fn negative_evidence_survives_invalid_verdict_and_cannot_fall_back_to_reasoning_on() {
    let reasoning = evidence_reply().to_string();
    for status in ["mismatch", "unknown"] {
        for verdict in [
            serde_json::Value::Null,
            serde_json::json!("maybe"),
            serde_json::json!(7),
        ] {
            let mut content = serde_json::json!({
                "checks": [["count", "one bottle", "three bottles", status]],
                "verdict": verdict
            });
            for missing in [false, true] {
                if missing {
                    content.as_object_mut().unwrap().remove("verdict");
                }
                let (verdict, reason) =
                    parse_verdict_response(Some(&content.to_string()), Some(&reasoning)).unwrap();
                assert_eq!(verdict, RelevanceVerdict::Off);
                assert!(reason.contains(&format!("count:{status}")));
            }
        }
    }
}

#[test]
fn complete_matching_evidence_accepts_on_and_only_minor_weak() {
    let mut reply = evidence_reply();
    // Optional physical constraints stay absent: no default count, action,
    // face, palette or premium appearance is demanded from every brief.
    assert_eq!(result(&reply).unwrap().0, RelevanceVerdict::On);
    reply["soft_deviations"] = serde_json::json!(["lighting", "style"]);
    assert_eq!(result(&reply).unwrap().0, RelevanceVerdict::Weak);
    reply["verdict"] = "weak".into();
    assert_eq!(result(&reply).unwrap().0, RelevanceVerdict::Weak);
    reply["soft_deviations"] = serde_json::json!([]);
    assert!(
        result(&reply).is_none(),
        "unexplained uncertainty cannot be a match"
    );
}

#[test]
fn incomplete_duplicate_and_unsubstantiated_evidence_cannot_grant_acceptance() {
    let base = evidence_reply();
    let mut bad = Vec::new();
    let mut reply = base.clone();
    reply.as_object_mut().unwrap().remove("checks");
    bad.push(reply);
    let mut reply = base.clone();
    reply["checks"].as_array_mut().unwrap().pop();
    bad.push(reply);
    let mut reply = base.clone();
    reply["checks"][1] = reply["checks"][0].clone();
    bad.push(reply);
    for row in [
        serde_json::json!(["appearance", "smooth", "", "match"]),
        serde_json::json!(["appearance", "", "smooth", "match"]),
        serde_json::json!(["appearance", "smooth", "rough", "not_required"]),
        serde_json::json!(["appearance", "smooth", "smooth", "probably"]),
        serde_json::json!(["unknown", "", "", "not_required"]),
        serde_json::json!(["appearance", "smooth", "smooth", "match", "extra"]),
        serde_json::json!(["appearance", null, "smooth", "match"]),
    ] {
        let mut reply = base.clone();
        reply["checks"][4] = row;
        bad.push(reply);
    }
    for index in [0, 9, 10] {
        let mut reply = base.clone();
        let category = reply["checks"][index][0].clone();
        reply["checks"][index] = serde_json::json!([category, "", "", "not_required"]);
        bad.push(reply);
    }
    let mut reply = base.clone();
    reply.as_object_mut().unwrap().remove("soft_deviations");
    bad.push(reply);
    for reply in bad {
        assert!(result(&reply).is_none(), "{reply}");
        assert_eq!(
            parse_candidate_response(0, Some(&reply.to_string()), None).verdict,
            RelevanceVerdict::Off
        );
    }
}

#[test]
fn legacy_rejection_is_retained_but_legacy_acceptance_is_unverified() {
    for verdict in ["on", "weak"] {
        let reply = serde_json::json!({
            "verdict": verdict, "reason": "broad topic matches",
            "hard_conflicts": [], "soft_deviations": ["style"]
        });
        assert!(result(&reply).is_none());
    }
    for reply in [
        serde_json::json!({"verdict":"off","reason":"wrong subject"}),
        serde_json::json!({"verdict":"off","checks":null,"soft_deviations":null}),
        serde_json::json!({"verdict":"weak","hard_conflicts":["count"],"checks":null}),
    ] {
        assert_eq!(result(&reply).unwrap().0, RelevanceVerdict::Off);
    }
}

fn result_for_brief(reply: &serde_json::Value, brief: &str) -> Option<(RelevanceVerdict, String)> {
    parse_evidence_response(Some(&reply.to_string()), None, "fallback query", brief)
}

#[test]
fn aesthetic_uncertainty_is_weak_after_complete_physical_matches() {
    let brief = "editorial photography of a luxury glass serum bottle in soft diffused light";
    for preference_status in ["deviation", "uncertain"] {
        for model_verdict in ["on", "weak"] {
            let mut reply = evidence_reply();
            reply["aesthetic_preferences"] = serde_json::json!([
                [
                    "luxury",
                    "style",
                    "plain apothecary finish",
                    preference_status
                ],
                [
                    "soft diffused light",
                    "lighting",
                    "soft even illumination",
                    "match"
                ]
            ]);
            // Regression: a quality impression cannot impose an unobservable
            // physical condition on a visibly correct, unbranded dropper.
            reply["verdict"] = model_verdict.into();
            let (verdict, reason) = result_for_brief(&reply, brief).unwrap();
            assert_eq!(verdict, RelevanceVerdict::Weak, "{reply}");
            assert!(reason.contains("soft_deviations=style"));
        }
    }
}

#[test]
fn aesthetic_uncertainty_cannot_override_any_hard_negative() {
    for category in ["subject", "count", "action", "appearance", "color_material"] {
        for status in ["unknown", "mismatch"] {
            let mut reply = evidence_reply();
            reply["aesthetic_preferences"] = serde_json::json!([[
                "cinematic",
                "style",
                "ordinary photographic treatment",
                "uncertain"
            ]]);
            let row = reply["checks"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|row| row[0] == category)
                .unwrap();
            *row = serde_json::json!([category, "explicit requirement", "not satisfied", status]);
            let (verdict, reason) = result_for_brief(&reply, "cinematic scene").unwrap();
            assert_eq!(verdict, RelevanceVerdict::Off);
            assert!(reason.contains(&format!("{category}:{status}")));
        }
    }
}

#[test]
fn physical_appearance_needs_a_concrete_kind_and_original_source_quote() {
    for (kind, quote, observation, brief) in [
        (
            "surface_texture",
            "smooth",
            "smooth visible surface",
            "smooth glowing skin",
        ),
        (
            "physical_condition",
            "intact",
            "no visible damage",
            "an intact ceramic cup",
        ),
    ] {
        let mut reply = evidence_reply();
        reply["checks"][4] = serde_json::json!(["appearance", quote, observation, "match"]);
        reply["appearance_contract"] = serde_json::json!({"kind":kind,"source_quote":quote});
        assert_eq!(
            result_for_brief(&reply, brief).unwrap().0,
            RelevanceVerdict::On
        );
        assert!(result_for_brief(&reply, "unrelated subject").is_none());
        for bad_kind in ["style", "aesthetic", "luxury", "not_required"] {
            reply["appearance_contract"]["kind"] = bad_kind.into();
            assert!(result_for_brief(&reply, brief).is_none());
        }
        reply["appearance_contract"]["kind"] = kind.into();
        reply["appearance_contract"]["source_quote"] = "".into();
        assert!(result_for_brief(&reply, brief).is_none());
    }
    // Quotes are checked against image_prompt; search-query wording must not
    // introduce a physical constraint absent from the actual slot brief.
    let mut reply = evidence_reply();
    reply["checks"][4] = serde_json::json!(["appearance", "smooth", "smooth skin", "match"]);
    reply["appearance_contract"] =
        serde_json::json!({"kind":"surface_texture","source_quote":"smooth"});
    let text = reply.to_string();
    assert!(parse_evidence_response(Some(&text), None, "smooth skin", "skin portrait").is_none());
    assert!(parse_evidence_response(Some(&text), None, "smooth skin", " ").is_some());
}

#[test]
fn v2_missing_or_malformed_fields_do_not_restore_unverified_acceptances() {
    let base = evidence_reply();
    for key in [
        "schema_version",
        "appearance_contract",
        "aesthetic_preferences",
    ] {
        let mut reply = base.clone();
        reply.as_object_mut().unwrap().remove(key);
        assert!(result_for_brief(&reply, "bottle").is_none());
        reply["verdict"] = "off".into();
        assert_eq!(
            result_for_brief(&reply, "bottle").unwrap().0,
            RelevanceVerdict::Off
        );
    }
    for preference in [
        serde_json::json!(["luxury", "style", "plain finish", "unknown"]),
        serde_json::json!(["two bottles", "count", "one bottle", "deviation"]),
        serde_json::json!(["", "style", "plain finish", "uncertain"]),
        serde_json::json!(["luxury", "style", "", "uncertain"]),
        serde_json::json!(["luxury", "style", "plain finish", "uncertain", "extra"]),
    ] {
        let mut reply = base.clone();
        reply["aesthetic_preferences"] = serde_json::json!([preference]);
        assert!(
            result_for_brief(&reply, "luxury two bottles").is_none(),
            "{reply}"
        );
    }
    let mut duplicate = base.clone();
    duplicate["aesthetic_preferences"] = serde_json::json!([
        ["luxury", "style", "plain finish", "uncertain"],
        ["luxury", "style", "premium finish", "match"]
    ]);
    assert!(result_for_brief(&duplicate, "luxury bottle").is_none());
}

#[test]
fn matching_aesthetic_preferences_do_not_invent_a_weak_or_override_unexplained_off() {
    let mut reply = evidence_reply();
    reply["aesthetic_preferences"] =
        serde_json::json!([["calm", "mood", "quiet restrained atmosphere", "match"]]);
    assert_eq!(
        result_for_brief(&reply, "calm still life").unwrap().0,
        RelevanceVerdict::On
    );
    reply["verdict"] = "off".into();
    assert_eq!(
        result_for_brief(&reply, "calm still life").unwrap().0,
        RelevanceVerdict::Off
    );
}

#[test]
fn source_quote_cannot_be_an_invented_subword_or_unrelated_check_requirement() {
    let mut reply = evidence_reply();
    reply["checks"][4] = serde_json::json!(["appearance", "fused", "fused glass", "match"]);
    reply["appearance_contract"] =
        serde_json::json!({"kind":"physical_condition","source_quote":"fused"});
    assert!(result_for_brief(&reply, "a glass bottle in diffused light").is_none());
    assert_eq!(
        result_for_brief(&reply, "a fused glass bottle").unwrap().0,
        RelevanceVerdict::On
    );
    reply["checks"][4][1] = "intact".into();
    assert!(result_for_brief(&reply, "a fused glass bottle").is_none());
    let mut reply = evidence_reply();
    reply["aesthetic_preferences"] =
        serde_json::json!([["fused", "style", "ordinary finish", "uncertain"]]);
    // A non-anchored soft quote cannot prove a match, but is not a hard
    // contradiction when all physical evidence is complete and matching.
    assert_eq!(
        result_for_brief(&reply, "bottle in diffused light")
            .unwrap()
            .0,
        RelevanceVerdict::Weak
    );
    reply["aesthetic_preferences"] =
        serde_json::json!([["Soft  Light", "lighting", "hard direct light", "deviation"]]);
    assert_eq!(
        result_for_brief(&reply, "bottle in soft\nlight").unwrap().0,
        RelevanceVerdict::Weak
    );
    reply["aesthetic_preferences"] =
        serde_json::json!([["柔和光线", "lighting", "直接硬光", "deviation"]]);
    assert_eq!(
        result_for_brief(&reply, "有柔和光线的产品图").unwrap().0,
        RelevanceVerdict::Weak
    );
}
