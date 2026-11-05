use super::*;

fn state() -> EditorState {
    let doc = jian_ops_schema::load_str(
        r#"{"version":"1.0","children":[{"type":"rectangle","id":"n1","name":"Card","x":0,"y":0,"width":10,"height":10}]}"#,
    )
    .expect("fixture")
    .value;
    let mut state = EditorState::new();
    state.doc = doc;
    state
}

fn args(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

/// Run a write tool and apply its command like the MCP host does.
fn write(
    state: &mut EditorState,
    make: fn(&EditorState) -> CommentTool,
    pairs: &[(&str, &str)],
) -> Value {
    match make(state).call(&args(pairs)) {
        ToolOutcome::OkJsonWithCommand(json, cmd) => {
            assert!(
                state.apply(cmd),
                "host apply must accept a validated command"
            );
            serde_json::from_str(&json).unwrap()
        }
        other => panic!("expected a write, got {other:?}"),
    }
}

fn read(tool: CommentTool, pairs: &[(&str, &str)]) -> Value {
    match tool.call(&args(pairs)) {
        ToolOutcome::OkJson(json) => serde_json::from_str(&json).unwrap(),
        other => panic!("expected json, got {other:?}"),
    }
}

fn add(state: &mut EditorState, text: &str) -> String {
    let out = write(
        state,
        add_comment_snapshot,
        &[("node_id", "n1"), ("text", text), ("author", "Claude")],
    );
    out["comment"]["id"].as_str().unwrap().to_owned()
}

#[test]
fn agent_adds_replies_and_addresses_a_thread() {
    let mut s = state();
    let id = add(&mut s, "Tighten the padding");
    let out = write(
        &mut s,
        reply_comment_snapshot,
        &[("comment_id", &id), ("text", "Padding is now 8")],
    );
    assert_eq!(out["comment"]["messages"].as_array().unwrap().len(), 2);
    write(
        &mut s,
        set_comment_status_snapshot,
        &[("comment_id", &id), ("status", "addressed")],
    );
    let thread = s.comment(&id).unwrap();
    assert_eq!(thread.status, CommentStatus::Addressed);
    assert_eq!(thread.messages[0].author.name, "Claude");
    assert_eq!(thread.messages[1].author.kind, CommentAuthorKind::Agent);

    let got = read(get_comment_snapshot(&s), &[("comment_id", &id)]);
    assert_eq!(got["comment"]["status"], "addressed");
    assert_eq!(got["comment"]["orphaned"], false);
}

#[test]
fn list_excludes_archived_unless_history_is_requested() {
    let mut s = state();
    let open = add(&mut s, "Open item");
    let old = add(&mut s, "Old item");
    for status in ["addressed", "archived"] {
        write(
            &mut s,
            set_comment_status_snapshot,
            &[("comment_id", &old), ("status", status)],
        );
    }

    let default = read(list_comments_snapshot(&s), &[]);
    assert_eq!(default["count"], 1);
    assert_eq!(default["hiddenArchivedCount"], 1);
    assert_eq!(default["comments"][0]["id"], open.as_str());

    let history = read(list_comments_snapshot(&s), &[("include_archived", "true")]);
    assert_eq!(history["count"], 2);

    let archived = read(list_comments_snapshot(&s), &[("status", "archived")]);
    assert_eq!(archived["count"], 1);
    assert_eq!(archived["comments"][0]["id"], old.as_str());
}

#[test]
fn invalid_writes_are_refused_before_reaching_the_host() {
    let s = state();
    let missing_node = add_comment_snapshot(&s).call(&args(&[("node_id", "nope"), ("text", "x")]));
    assert!(matches!(
        missing_node,
        ToolOutcome::Err(ToolErrorCode::ToolFailed, _)
    ));
    let no_anchor = add_comment_snapshot(&s).call(&args(&[("text", "x")]));
    assert!(matches!(
        no_anchor,
        ToolOutcome::Err(ToolErrorCode::ToolFailed, _)
    ));
    let no_text = add_comment_snapshot(&s).call(&args(&[("x", "1"), ("y", "2")]));
    assert!(matches!(
        no_text,
        ToolOutcome::Err(ToolErrorCode::MissingArgument, _)
    ));
    let bad_status = list_comments_snapshot(&s).call(&args(&[("status", "done")]));
    assert!(matches!(
        bad_status,
        ToolOutcome::Err(ToolErrorCode::InvalidArgument, _)
    ));
}

#[test]
fn archived_thread_can_only_be_reopened() {
    let mut s = state();
    let id = add(&mut s, "Item");
    write(
        &mut s,
        set_comment_status_snapshot,
        &[("comment_id", &id), ("status", "archived")],
    );
    let refused = set_comment_status_snapshot(&s)
        .call(&args(&[("comment_id", &id), ("status", "addressed")]));
    assert!(matches!(
        refused,
        ToolOutcome::Err(ToolErrorCode::ToolFailed, _)
    ));
    write(
        &mut s,
        set_comment_status_snapshot,
        &[("comment_id", &id), ("status", "pending")],
    );
    assert_eq!(s.comment(&id).unwrap().status, CommentStatus::Pending);
}

#[test]
fn point_comments_resolve_to_their_page() {
    let mut s = state();
    let out = write(
        &mut s,
        add_comment_snapshot,
        &[("x", "40"), ("y", "-12.5"), ("text", "Something here")],
    );
    assert_eq!(out["comment"]["anchor"]["x"], 40.0);
    assert_eq!(
        out["comment"]["messages"][0]["author"]["name"],
        DEFAULT_AGENT_AUTHOR
    );
}
