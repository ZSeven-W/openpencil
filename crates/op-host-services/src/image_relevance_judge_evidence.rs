//! Evidence validation shared by both image-judge transports.

use op_image_enrich::net::RelevanceVerdict;

pub(super) fn parse_verdict_response(
    content: Option<&str>,
    reasoning_content: Option<&str>,
    query: &str,
    intent: &str,
) -> Option<(RelevanceVerdict, String)> {
    let brief = if intent.trim().is_empty() {
        query
    } else {
        intent
    };
    parse_verdict_text(content, brief).or_else(|| parse_verdict_text(reasoning_content, brief))
}

fn parse_verdict_text(text: Option<&str>, brief: &str) -> Option<(RelevanceVerdict, String)> {
    let text = text?.trim();
    let json = serde_json::from_str::<serde_json::Value>(text)
        .ok()
        .or_else(|| {
            let start = text.find('{')?;
            let end = text.rfind('}')?;
            (start < end).then(|| serde_json::from_str(&text[start..=end]).ok())?
        })?;
    const HARD_CODES: &[&str] = &[
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
    let (mut hard_conflicts, hard_valid) =
        contract_codes(&json, "hard_conflicts", HARD_CODES, false);
    // A known hard conflict cannot be made usable by putting it in the soft
    // array, attaching another malformed field, or writing a weaker verdict.
    let (misclassified_hard, _) = contract_codes(&json, "soft_deviations", HARD_CODES, false);
    for code in misclassified_hard {
        if !hard_conflicts.contains(&code) {
            hard_conflicts.push(code);
        }
    }
    let reason = json
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .split_whitespace()
        .take(12)
        .collect::<Vec<_>>()
        .join(" ");
    if !hard_conflicts.is_empty() {
        return Some((
            RelevanceVerdict::Off,
            format!("hard_conflicts={}; {reason}", hard_conflicts.join(",")),
        ));
    }
    // Legacy rejections remain useful, but an acceptance now needs complete
    // evidence. A model's empty hard-conflict list is not proof of a match.
    // Check explicit negative observations before structural validation so a
    // malformed sibling row cannot erase a useful, conservative rejection.
    let checks = json.get("checks").and_then(serde_json::Value::as_array);
    if let Some(checks) = checks {
        for check in checks {
            if let Some(row) = check.as_array() {
                if let (Some(category), Some(status)) = (
                    row.first().and_then(serde_json::Value::as_str),
                    row.get(3).and_then(serde_json::Value::as_str),
                ) {
                    if HARD_CODES.contains(&category) && ["mismatch", "unknown"].contains(&status) {
                        return Some((
                            RelevanceVerdict::Off,
                            format!("check={category}:{status}; {reason}"),
                        ));
                    }
                }
            }
        }
    }
    let mut verdict = match json
        .get("verdict")?
        .as_str()?
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "on" => RelevanceVerdict::On,
        "weak" => RelevanceVerdict::Weak,
        "off" => RelevanceVerdict::Off,
        _ => return None,
    };
    // A compatibility allowance for soft metadata must never turn an explicit
    // rejection into permission. Preserve Off even when its evidence is invalid.
    if verdict == RelevanceVerdict::Off {
        return Some((verdict, reason));
    }
    if json.get("schema_version").and_then(|v| v.as_u64()) != Some(2) {
        return None;
    }
    validate_evidence_checks(checks?, HARD_CODES)?;
    validate_appearance(&json, checks?, brief)?;
    let preference_deviations = preference_deviations(&json, brief)?;
    let (soft_deviations, soft_valid) =
        contract_codes(&json, "soft_deviations", &["lighting", "style"], true);
    let mut soft_deviations: Vec<&str> = soft_deviations;
    for code in preference_deviations {
        if !soft_deviations.contains(&code) {
            soft_deviations.push(code);
        }
    }
    if !hard_valid
        || !soft_valid
        || !json
            .get("soft_deviations")
            .is_some_and(serde_json::Value::is_array)
        || reason.is_empty()
        || (verdict == RelevanceVerdict::Weak && soft_deviations.is_empty())
    {
        return None;
    }
    let reason = if !soft_deviations.is_empty() {
        // Only an otherwise verified On/Weak can tolerate uncertain aesthetic
        // evidence or a paraphrased soft quote, and it can never become On.
        verdict = RelevanceVerdict::Weak;
        format!("soft_deviations={}; {reason}", soft_deviations.join(","))
    } else {
        reason
    };
    Some((verdict, reason))
}

fn validate_appearance(
    json: &serde_json::Value,
    checks: &[serde_json::Value],
    brief: &str,
) -> Option<()> {
    let contract = json.get("appearance_contract")?.as_object()?;
    let kind = contract.get("kind")?.as_str()?;
    let quote = contract.get("source_quote")?.as_str()?.trim();
    let appearance = checks.iter().find(|row| row[0] == "appearance")?;
    match kind {
        "not_required" if quote.is_empty() && appearance[3] == "not_required" => Some(()),
        "surface_texture" | "physical_condition"
            if appearance[3] == "match"
                && normalize(appearance[1].as_str()?) == normalize(quote)
                && source_quote_exists(brief, quote) =>
        {
            Some(())
        }
        _ => None,
    }
}

/// Check provenance, not natural-language semantics. The closed physical kinds
/// remove an aesthetic catch-all, while the original quote makes invented
/// constraints detectable. Semantic classification is still a model task.
fn source_quote_exists(brief: &str, quote: &str) -> bool {
    let brief = normalize(brief);
    let quote = normalize(quote);
    if quote.is_empty() {
        return false;
    }
    let word = |ch: char| ch.is_ascii_alphanumeric() || ch == '_';
    brief.match_indices(&quote).any(|(start, matched)| {
        // Preserve ordinary CJK phrase matching without accepting invented
        // English conditions such as "fused" sliced out of "diffused".
        let left = !quote.starts_with(word) || !brief[..start].ends_with(word);
        let right = !quote.ends_with(word) || !brief[start + matched.len()..].starts_with(word);
        left && right
    })
}

fn normalize(text: &str) -> String {
    text.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn preference_deviations(json: &serde_json::Value, brief: &str) -> Option<Vec<&'static str>> {
    let preferences = json.get("aesthetic_preferences")?.as_array()?;
    let mut deviations = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for preference in preferences {
        let row = preference.as_array()?;
        if row.len() != 4 {
            return None;
        }
        let quote = row[0].as_str()?.trim();
        let dimension = row[1].as_str()?;
        let observation = row[2].as_str()?.trim();
        let status = row[3].as_str()?;
        if quote.is_empty()
            || observation.is_empty()
            || !seen.insert((quote.to_lowercase(), dimension))
        {
            return None;
        }
        let code = match dimension {
            "lighting" => "lighting",
            "style" | "mood" => "style",
            _ => return None,
        };
        let differs = match status {
            "match" => false,
            "deviation" | "uncertain" => true,
            _ => return None,
        };
        // Soft provenance is diagnostic: a paraphrase or combined quote does
        // not invalidate complete hard evidence. It is uncertainty, never
        // evidence for On. Physical source quotes remain strict above.
        if (differs || !source_quote_exists(brief, quote)) && !deviations.contains(&code) {
            deviations.push(code);
        }
    }
    Some(deviations)
}

/// All categories must be considered, but optional categories cannot invent
/// requirements. Text observations establish an auditable comparison; this
/// validates the evidence contract, not arbitrary natural-language semantics.
fn validate_evidence_checks(checks: &[serde_json::Value], categories: &[&str]) -> Option<()> {
    if checks.len() != categories.len() {
        return None;
    }
    let mut seen = std::collections::HashSet::new();
    for check in checks {
        let row = check.as_array()?;
        if row.len() != 4 {
            return None;
        }
        let category = row[0].as_str()?;
        let requirement = row[1].as_str()?.trim();
        let observation = row[2].as_str()?.trim();
        let status = row[3].as_str()?;
        if !categories.contains(&category) || !seen.insert(category) {
            return None;
        }
        match status {
            "match" if !requirement.is_empty() && !observation.is_empty() => {}
            "not_required"
                if requirement.is_empty()
                    && observation.is_empty()
                    && !["subject", "unrequested_brand", "watermark_ad"].contains(&category) => {}
            _ => return None,
        }
    }
    Some(())
}

fn contract_codes<'a>(
    json: &'a serde_json::Value,
    field: &str,
    allowed: &[&str],
    allow_explanation: bool,
) -> (Vec<&'a str>, bool) {
    let Some(value) = json.get(field) else {
        return (Vec::new(), true);
    };
    let Some(values) = value.as_array() else {
        return (Vec::new(), false);
    };
    let mut codes = Vec::new();
    let mut valid = true;
    for value in values {
        let tuple = value.as_array();
        let code = value.as_str().or_else(|| tuple?.first()?.as_str());
        // Recognize the code before validating its shape: a known hard conflict
        // inside a malformed tuple must still exclude the candidate. Only soft
        // codes may use the narrow [code, nonempty explanation] compatibility.
        let shape_valid = value.is_string()
            || (allow_explanation
                && tuple.is_some_and(|row| {
                    row.len() == 2 && row[1].as_str().is_some_and(|s| !s.trim().is_empty())
                }));
        valid &= shape_valid;
        match code.filter(|code| allowed.contains(code)) {
            Some(code) if !codes.contains(&code) => codes.push(code),
            Some(_) => {}
            None => valid = false,
        }
    }
    (codes, valid)
}
