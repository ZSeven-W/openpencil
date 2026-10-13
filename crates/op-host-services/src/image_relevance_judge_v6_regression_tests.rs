use super::*;
use serde_json::{json, Value};

const HARD_CODES: [&str; 12] = [
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
];
const BOTTLE_BRIEF: &str =
    "editorial product photography of a luxury skincare serum glass bottle, \
    soft diffused light, ivory background, minimal elegant composition";
const CREAM_BRIEF: &str = "soft-lit macro close-up of luxurious skincare cream texture, ivory and \
    warm neutral tones, gentle diffused light, premium editorial cosmetics photography, \
    shallow depth of field";

fn matching_reply() -> Value {
    let checks: Vec<_> = HARD_CODES
        .into_iter()
        .map(|code| {
            if ["subject", "unrequested_brand", "watermark_ad"].contains(&code) {
                json!([
                    code,
                    "brief requirement",
                    "visible matching evidence",
                    "match"
                ])
            } else {
                json!([code, "", "", "not_required"])
            }
        })
        .collect();
    json!({
        "schema_version": 2,
        "checks": checks,
        "appearance_contract": {"kind":"not_required", "source_quote":""},
        "aesthetic_preferences": [],
        "soft_deviations": [],
        "verdict": "on",
        "reason": "visible requirements match"
    })
}

fn parse(reply: &Value, brief: &str) -> Option<(RelevanceVerdict, String)> {
    parse_evidence_response(Some(&reply.to_string()), None, "fallback query", brief)
}

fn bottle_reply() -> Value {
    // Reduced from quality-v5-chat-probe/evidence.jsonl line 3. Keep the
    // original soft tuple and preference text that caused judge unavailable.
    let mut reply = matching_reply();
    reply["checks"][0] = json!([
        "subject",
        "luxury skincare serum glass bottle",
        "amber dropper serum bottle",
        "match"
    ]);
    reply["aesthetic_preferences"] = json!([
        [
            "luxury",
            "style",
            "plain apothecary-style amber vial",
            "uncertain"
        ],
        [
            "soft diffused light",
            "lighting",
            "soft even lighting",
            "match"
        ],
        ["minimal elegant", "style", "minimal composition", "match"]
    ]);
    reply["soft_deviations"] = json!([[
        "style",
        "plain amber apothecary look rather than luxury finish"
    ]]);
    reply["verdict"] = "weak".into();
    reply["reason"] = "Matches brief; luxury finish uncertain on plain amber vial".into();
    reply
}

fn cream_reply(preferences: Value) -> Value {
    let mut reply = matching_reply();
    reply["checks"][0] = json!([
        "subject",
        "skincare cream texture macro",
        "smeared cream texture close-up",
        "match"
    ]);
    reply["aesthetic_preferences"] = preferences;
    reply["reason"] = "cream texture macro matches brief tones and style".into();
    reply
}

#[test]
fn v5_bottle_explained_soft_tuple_is_weak_after_complete_hard_matches() {
    for headline in ["on", "weak"] {
        let mut reply = bottle_reply();
        reply["verdict"] = headline.into();
        let (verdict, reason) = parse(&reply, BOTTLE_BRIEF).unwrap();
        assert_eq!(verdict, RelevanceVerdict::Weak);
        assert!(reason.contains("soft_deviations=style"));
    }
    let mut reply = matching_reply();
    reply["soft_deviations"] = json!([["lighting", "illumination is slightly harsher"]]);
    assert_eq!(
        parse(&reply, BOTTLE_BRIEF).unwrap().0,
        RelevanceVerdict::Weak
    );
}

#[test]
fn v5_cream_merged_preference_quotes_are_weak_instead_of_unavailable() {
    // Original preference rows from evidence.jsonl lines 13 and 14: these
    // merge separate brief phrases but do not assert physical constraints.
    for preferences in [
        json!([
            [
                "soft-lit gentle diffused light",
                "lighting",
                "soft even lighting",
                "match"
            ],
            [
                "premium editorial mood",
                "style",
                "premium texture look",
                "match"
            ]
        ]),
        json!([
            [
                "soft-lit, gentle diffused light",
                "lighting",
                "soft even light",
                "match"
            ],
            [
                "shallow depth of field",
                "style",
                "uniform focus, minimal blur",
                "uncertain"
            ]
        ]),
    ] {
        for headline in ["on", "weak"] {
            let mut reply = cream_reply(preferences.clone());
            reply["verdict"] = headline.into();
            let (verdict, reason) = parse(&reply, CREAM_BRIEF).unwrap();
            assert_eq!(verdict, RelevanceVerdict::Weak, "{reply}");
            assert!(reason.contains("lighting"));
        }
    }
}

#[test]
fn soft_tuple_rejects_unknown_codes_empty_explanations_and_malformed_shapes() {
    for value in [
        json!([["mood", "calm atmosphere"]]),
        json!([["unknown", "a visible difference"]]),
        json!([["", "a visible difference"]]),
        json!([["style", ""]]),
        json!([["style", " \n\t "]]),
        json!([["style", null]]),
        json!([["style", 3]]),
        json!([[null, "explanation"]]),
        json!([["style"]]),
        json!([["style", "explanation", "extra"]]),
        json!([[]]),
        json!([null]),
        json!([{"style":"explanation"}]),
        json!(["style", "explanation"]),
        Value::Null,
    ] {
        for headline in ["on", "weak"] {
            let mut reply = matching_reply();
            reply["verdict"] = headline.into();
            reply["soft_deviations"] = value.clone();
            assert!(parse(&reply, BOTTLE_BRIEF).is_none(), "{reply}");
        }
    }
}

#[test]
fn soft_tuple_tolerance_cannot_bypass_the_schema_or_physical_evidence() {
    let base = bottle_reply();
    for missing in ["schema_version", "checks", "appearance_contract", "verdict"] {
        let mut reply = base.clone();
        reply.as_object_mut().unwrap().remove(missing);
        assert!(parse(&reply, BOTTLE_BRIEF).is_none(), "{missing}");
    }
    let mut invalid = Vec::new();
    let mut reply = base.clone();
    reply["checks"].as_array_mut().unwrap().pop();
    invalid.push(reply);
    let mut reply = base.clone();
    reply["checks"][1] = reply["checks"][0].clone();
    invalid.push(reply);
    let mut reply = base.clone();
    reply["checks"][0][2] = "".into();
    invalid.push(reply);
    let mut reply = base.clone();
    reply["appearance_contract"]["kind"] = "style".into();
    invalid.push(reply);
    for headline in [json!(null), json!("maybe"), json!(1)] {
        let mut reply = base.clone();
        reply["verdict"] = headline;
        invalid.push(reply);
    }
    for reply in invalid {
        assert!(parse(&reply, BOTTLE_BRIEF).is_none(), "{reply}");
    }
}

#[test]
fn nonverbatim_preference_tolerance_keeps_row_shape_and_dimensions_strict() {
    for preference in [
        json!(["", "lighting", "soft even lighting", "match"]),
        json!(["soft-lit gentle light", "lighting", "", "match"]),
        json!(["soft-lit gentle light", "lighting", "soft light", "unknown"]),
        json!(["two bottles", "count", "one bottle", "deviation"]),
        json!(["soft-lit gentle light", "lighting", "soft light"]),
        json!([
            "soft-lit gentle light",
            "lighting",
            "soft light",
            "match",
            "extra"
        ]),
        json!([null, "lighting", "soft light", "match"]),
    ] {
        let reply = cream_reply(json!([preference]));
        assert!(parse(&reply, CREAM_BRIEF).is_none(), "{reply}");
    }
}

#[test]
fn explicit_off_is_never_upgraded_by_complete_soft_evidence_or_reasoning_on() {
    let reasoning = matching_reply().to_string();
    for mut reply in [
        bottle_reply(),
        cream_reply(json!([[
            "premium editorial mood",
            "style",
            "premium texture look",
            "match"
        ]])),
        cream_reply(json!([[
            "shallow depth of field",
            "style",
            "uniform focus",
            "uncertain"
        ]])),
    ] {
        reply["verdict"] = "off".into();
        let result = parse_evidence_response(
            Some(&reply.to_string()),
            Some(&reasoning),
            "fallback query",
            CREAM_BRIEF,
        )
        .unwrap();
        assert_eq!(result.0, RelevanceVerdict::Off, "{reply}");
    }
}

#[test]
fn hard_codes_inside_tuples_survive_missing_verdict_and_reasoning_on() {
    let reasoning = matching_reply().to_string();
    for field in ["soft_deviations", "hard_conflicts"] {
        for code in HARD_CODES {
            // Corrupt siblings must not hide an explicit known hard conflict.
            let mut reply = json!({
                "schema_version": null,
                "checks": null,
                "appearance_contract": null,
                "verdict": "maybe",
                "reason": "explicit physical contradiction"
            });
            reply[field] = json!([[code, "explicit physical contradiction"]]);
            for missing_verdict in [false, true] {
                if missing_verdict {
                    reply.as_object_mut().unwrap().remove("verdict");
                }
                let (verdict, reason) = parse_evidence_response(
                    Some(&reply.to_string()),
                    Some(&reasoning),
                    "fallback query",
                    BOTTLE_BRIEF,
                )
                .unwrap();
                assert_eq!(verdict, RelevanceVerdict::Off, "{reply}");
                assert!(reason.contains(&format!("hard_conflicts={code}")));
            }
        }
    }
}

#[test]
fn hard_negative_checks_precede_soft_tuple_and_paraphrase_tolerance() {
    for code in HARD_CODES {
        for status in ["unknown", "mismatch"] {
            let mut reply = bottle_reply();
            reply["aesthetic_preferences"] = json!([[
                "premium editorial mood",
                "style",
                "premium texture look",
                "match"
            ]]);
            let row = reply["checks"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|row| row[0] == code)
                .unwrap();
            *row = json!([code, "explicit requirement", "not satisfied", status]);
            let (verdict, reason) = parse(&reply, BOTTLE_BRIEF).unwrap();
            assert_eq!(verdict, RelevanceVerdict::Off, "{reply}");
            assert!(reason.contains(&format!("{code}:{status}")));
        }
    }
}

#[test]
fn soft_paraphrases_never_relax_physical_appearance_source_provenance() {
    let mut reply = bottle_reply();
    reply["checks"][4] = json!(["appearance", "intact", "unbroken glass", "match"]);
    reply["appearance_contract"] = json!({"kind":"physical_condition","source_quote":"intact"});
    let brief = "intact smooth glass bottle in diffused light";
    assert_eq!(parse(&reply, brief).unwrap().0, RelevanceVerdict::Weak);

    // Both words occur in the brief, but the check and its source must agree.
    reply["appearance_contract"]["source_quote"] = "smooth".into();
    assert!(parse(&reply, brief).is_none());
    for invented_quote in ["fused", "intact smooth bottle", "unbroken"] {
        reply["checks"][4][1] = invented_quote.into();
        reply["appearance_contract"]["source_quote"] = invented_quote.into();
        assert!(parse(&reply, brief).is_none(), "{invented_quote}");
    }
}
