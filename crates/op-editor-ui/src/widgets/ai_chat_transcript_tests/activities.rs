//! Structured activities and progress steps — rows, ordering and collapse state.
//!
//! Split out of `ai_chat_transcript_tests.rs` to keep that file under the
//! 800-line cap.

use super::*;

#[test]
fn empty_design_progress_lines_do_not_render_inline_or_as_typing_placeholder() {
    let mut m = ChatMessage::assistant_streaming();
    m.thinking = "\n• Planning…\n• Scaffold ready".into();
    let items = build_transcript(
        std::slice::from_ref(&m),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert!(
        items[0].steps.is_empty(),
        "empty plan/progress rows belong to the fixed checklist, matching TS ActionSteps"
    );
    assert!(
        items[0].thinking.is_none(),
        "design progress should not render as a reasoning block"
    );
    assert!(
        items[0].bubble.is_none(),
        "fixed checklist progress should suppress the empty streaming typing placeholder"
    );
}

#[test]
fn structured_activities_render_as_compact_rows_without_thinking_text() {
    let mut message = ChatMessage::assistant_streaming();
    message.activities = vec![
        op_editor_core::ChatActivity {
            id: "header".into(),
            title: "Greeting header".into(),
            detail: None,
            status: op_editor_core::ChatActivityStatus::Running,
            content_offset: None,
        },
        op_editor_core::ChatActivity {
            id: "rail".into(),
            title: "Recently played".into(),
            detail: Some("12 elements".into()),
            status: op_editor_core::ChatActivityStatus::Done,
            content_offset: None,
        },
    ];
    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 2);
    assert_eq!(items[0].steps[0].label, "Greeting header");
    assert!(items[0].steps[0].active);
    assert_eq!(items[0].steps[0].rect.size.y, ACTION_STEP_H);
    assert_eq!(items[0].steps[1].label, "Recently played");
    assert!(items[0].steps[1].done);
    assert!(items[0].thinking.is_none());
    assert!(items[0].bubble.is_none());
}

#[test]
fn failed_structured_activity_defaults_open_to_show_its_diagnostic() {
    let mut message = ChatMessage::assistant("Finished with issues");
    message.activities.push(op_editor_core::ChatActivity {
        id: "customer-table".into(),
        title: "Customer Table".into(),
        detail: Some("Reason: parent_id=dashboard was not found".into()),
        status: op_editor_core::ChatActivityStatus::Error,
        content_offset: None,
    });

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );
    assert!(items[0].steps[0].failed);
    assert!(items[0].steps[0].expanded);
    assert_eq!(
        items[0].steps[0].details,
        vec!["Reason: parent_id=dashboard was not found"]
    );

    message.action_step_expanded_overrides = vec![Some(false)];
    let collapsed = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );
    assert!(!collapsed[0].steps[0].expanded);
}

#[test]
fn structured_activities_interleave_with_cli_narration_by_offset() {
    let first = "I mapped the screen.";
    let second = "The sections are in place.";
    let final_text = "Done — the layout has been checked.";
    let content = format!("{first}\n\n{second}\n\n{final_text}");
    let second_offset = first.len() + 2 + second.len();
    let mut message = ChatMessage::assistant(&content);
    message.activities = vec![
        op_editor_core::ChatActivity {
            id: "build".into(),
            title: "Building sections".into(),
            detail: None,
            status: op_editor_core::ChatActivityStatus::Done,
            content_offset: Some(first.len() as u32),
        },
        op_editor_core::ChatActivity {
            id: "check".into(),
            title: "Checking the design".into(),
            detail: None,
            status: op_editor_core::ChatActivityStatus::Done,
            content_offset: Some(second_offset as u32),
        },
    ];

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );
    let item = &items[0];

    assert_eq!(item.steps.len(), 2);
    assert_eq!(item.flow_bubbles.len(), 3);
    assert!(item.bubble.is_none());
    assert!(item.flow_bubbles[0].rect.origin.y < item.steps[0].rect.origin.y);
    assert!(item.steps[0].rect.origin.y < item.flow_bubbles[1].rect.origin.y);
    assert!(item.flow_bubbles[1].rect.origin.y < item.steps[1].rect.origin.y);
    assert!(item.steps[1].rect.origin.y < item.flow_bubbles[2].rect.origin.y);
}

#[test]
fn legacy_and_interleaved_activity_steps_use_distinct_override_slots() {
    let mut message = ChatMessage::assistant("Narration");
    message.thinking = "• Legacy detail\n  ▸ diagnostic".into();
    message.activities.push(op_editor_core::ChatActivity {
        id: "build".into(),
        title: "Building section".into(),
        detail: Some("2 elements".into()),
        status: op_editor_core::ChatActivityStatus::Done,
        content_offset: Some(0),
    });
    message.action_step_expanded_overrides = vec![Some(false), Some(true)];

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 2);
    assert_eq!(items[0].steps[0].source_index, 0);
    assert_eq!(items[0].steps[1].source_index, 1);
    assert!(!items[0].steps[0].expanded);
    assert!(items[0].steps[1].expanded);
}

#[test]
fn detail_less_structured_activity_has_no_invisible_toggle_hit() {
    let mut message = ChatMessage::assistant_streaming();
    message.activities.push(op_editor_core::ChatActivity {
        id: "header".into(),
        title: "Greeting header".into(),
        detail: None,
        status: op_editor_core::ChatActivityStatus::Running,
        content_offset: None,
    });
    let body = body();
    let canonical = crate::widgets::ai_chat_transcript_cache::unowned_for_tests(
        std::slice::from_ref(&message),
        body,
        op_editor_core::Locale::EnUs,
    );
    let step = &canonical.items[0].steps[0];

    assert_eq!(
        transcript_hit(
            &canonical,
            body,
            step.rect.origin.x + 8.0,
            step.rect.origin.y + 8.0,
            0.0,
        ),
        None
    );
}

#[test]
fn current_step_with_content_is_active_until_terminal() {
    let mut m = ChatMessage::assistant_streaming();
    m.content = r#"<step title="Planning…">Drafting layout constraints</step>"#.into();
    let items = build_transcript(
        std::slice::from_ref(&m),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 1);
    assert_eq!(items[0].steps[0].label, "Planning…");
    assert!(items[0].steps[0].active);
    assert!(!items[0].steps[0].done);
}

#[test]
fn step_tag_content_renders_as_progress_not_raw_bubble() {
    let mut message = ChatMessage::assistant_streaming();
    message.content =
        r#"<step title="Checking guidelines" status="streaming">Analyzing request...</step>"#
            .into();

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 1);
    assert_eq!(items[0].steps[0].label, "Checking guidelines");
    assert!(items[0].steps[0].active);
    assert!(
        items[0].bubble.is_none(),
        "raw <step> markup should not render as answer text"
    );
}

#[test]
fn step_tag_content_surfaces_as_progress_details() {
    let mut message = ChatMessage::assistant_streaming();
    message.content = r#"<step title="Validate design" status="streaming">
lint: fixed spacing
render: captured frame
</step>"#
        .into();

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 1);
    assert_eq!(
        items[0].steps[0].details,
        vec![
            "lint: fixed spacing".to_string(),
            "render: captured frame".to_string()
        ]
    );
    assert!(
        items[0].steps[0].rect.size.y > 28.0,
        "step details should reserve space instead of being dropped"
    );
}

#[test]
fn completed_step_with_content_defaults_collapsed_like_ts_accordion() {
    let mut message = ChatMessage::assistant(
        r#"<step title="Validate design" status="done">
lint: fixed spacing
render: captured frame
</step>"#,
    );
    message.streaming = false;

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    assert_eq!(items[0].steps.len(), 1);
    assert!(items[0].steps[0].done);
    assert!(!items[0].steps[0].active);
    assert!(
        (items[0].steps[0].rect.size.y - ACTION_STEP_H).abs() < 1e-4,
        "TS ActionStepItem defaults completed accordions closed"
    );
}

#[test]
fn paint_completed_step_hides_details_like_collapsed_ts_accordion() {
    let message = ChatMessage::assistant(
        r#"<step title="Validate design" status="done">
lint: fixed spacing
render: captured frame
</step>"#,
    );
    let mut backend = TranscriptPaintBackend::default();
    let mut cx = PaintCx {
        backend: &mut backend,
    };

    paint_transcript(
        &mut cx,
        &crate::Theme::dark(),
        body(),
        &[message],
        0,
        op_editor_core::Locale::EnUs,
    );

    assert!(
        backend.texts.iter().any(|text| text == "Validate design"),
        "collapsed accordion still paints its title"
    );
    assert!(
        !backend
            .texts
            .iter()
            .any(|text| text.contains("lint: fixed spacing")
                || text.contains("render: captured frame")),
        "collapsed TS accordions hide details until opened"
    );
}

#[test]
fn an_itemized_activity_detail_expands_into_one_row_per_line() {
    // The quality passes report their repairs as one line each on the
    // "Polishing the layout" row. A single-string detail that rendered as one
    // unwrappable blob is exactly the "I cannot see what the check changed"
    // complaint this exists to answer.
    let detail = [
        "3 auto-repair(s) applied",
        "layout · table-gap · Pricing Row [n42] · gap 0 → 16",
        "palette · light-mobile-nav-surface · Tab Bar [n7] · fill #F8FAFC → #FFFFFF",
        "hierarchy · text-hierarchy · Title [n3] · fontWeight 800 → 400",
    ]
    .join("\n");
    let mut message = ChatMessage::assistant_streaming();
    message.activities = vec![op_editor_core::ChatActivity {
        id: "__polish".into(),
        title: "Polishing the layout".into(),
        detail: Some(detail),
        status: op_editor_core::ChatActivityStatus::Done,
        content_offset: None,
    }];

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    let step = &items[0].steps[0];
    assert_eq!(step.label, "Polishing the layout");
    // Rows are word-wrapped to the bubble width, so a long record may occupy
    // more than one row — what must not happen is lines being merged or
    // dropped.
    assert!(
        step.details.len() >= 4,
        "each reported repair must reach the row list: {:?}",
        step.details
    );
    // Each reported repair must OWN a row: a row that merely CONTAINS the
    // text could be one blob the layout wrapped at an arbitrary column, which
    // is the unreadable rendering this test exists to reject.
    for record in [
        "3 auto-repair(s) applied",
        "layout · table-gap · Pricing Row",
        "palette · light-mobile-nav-surface",
        "hierarchy · text-hierarchy · Title",
    ] {
        assert!(
            step.details.iter().any(|row| row.starts_with(record)),
            "`{record}` must begin its own row: {:?}",
            step.details
        );
    }
    assert!(
        step.details.iter().all(|row| !row.contains('\n')),
        "no row may carry a raw newline: {:?}",
        step.details
    );
}

#[test]
fn a_long_repair_list_renders_every_line_it_was_given_plus_the_overflow_notice() {
    // The host caps the list at 30 and appends a localized "and N more"
    // notice; the transcript's job is to render exactly what it was handed —
    // silently dropping rows here would hide repairs the host chose to show.
    let mut lines = vec!["45 auto-repair(s) applied".to_string()];
    lines.extend(
        (0..30).map(|i| format!("layout · container-geometry · Card {i} [n{i}] · gap 24 → 16")),
    );
    lines.push("… and 15 more (see log)".to_string());
    let mut message = ChatMessage::assistant_streaming();
    message.activities = vec![op_editor_core::ChatActivity {
        id: "__polish".into(),
        title: "Polishing the layout".into(),
        detail: Some(lines.join("\n")),
        status: op_editor_core::ChatActivityStatus::Done,
        content_offset: None,
    }];
    message.action_step_expanded_overrides = vec![Some(true)];

    let items = build_transcript(
        std::slice::from_ref(&message),
        body(),
        op_editor_core::Locale::EnUs,
    );

    let step = &items[0].steps[0];
    assert!(
        step.details.len() >= 32,
        "head line + 30 records + overflow notice must all reach the rows: {}",
        step.details.len()
    );
    assert!(
        step.details
            .last()
            .is_some_and(|row| row.starts_with("… and 15 more")),
        "the truncation notice must own the closing row: {:?}",
        step.details.last()
    );
    assert_eq!(
        step.details
            .iter()
            .filter(|row| row.starts_with("layout · container-geometry"))
            .count(),
        30,
        "every one of the 30 shown records must begin its own row"
    );
    assert!(
        step.expanded,
        "an explicit expand override must survive the itemized list"
    );
    assert!(
        step.rect.size.y > 32.0 * LINE_H,
        "an expanded 32-row list must reserve height for its rows"
    );
}
