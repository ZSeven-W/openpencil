//! Engine-thread tests for the mobile design agent loop: a builtin-provider
//! DESIGN request must run the REAL shared tool loop and land real nodes in
//! the open document (not HTML prose in the transcript), with the tool
//! cards folded into the bound tab's assistant bubble.

use super::*;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use op_editor_core::{BuiltinAgentKind, PenNodeExt, Viewport};

// iPhone 17 Pro safe-area-local editor viewport (full 874 pt height minus
// system status / home-indicator insets). The Swift shell passes this local
// size to the Rust editor after forwarding the insets separately.
const TEST_VIEWPORT: (f32, f32) = (402.0, 782.0);

fn host_with_builtin_provider(base_url: &str) -> WidgetHostNative {
    let mut host = WidgetHostNative::new();
    let settings = &mut host.editor_state_mut().editor_ui.agent_settings;
    settings.builtin_agents.clear();
    settings.add_builtin_agent_config(
        "DeepSeek",
        "sk-mobile-design",
        "deepseek-chat",
        BuiltinAgentKind::OpenAiCompat,
        base_url,
    );
    host.editor_state_mut().rebuild_chat_models();
    let chat = &mut host.editor_state_mut().chat;
    let builtin_index = chat
        .available_models
        .iter()
        .position(|entry| entry.builtin_provider_id.is_some())
        .expect("a ready builtin agent must surface a chat model entry");
    chat.selected_model = builtin_index;
    host
}

fn send_user_message(host: &mut WidgetHostNative, text: &str) {
    let chat = &mut host.editor_state_mut().chat;
    chat.set_input_text(text);
    assert!(chat.begin_send(), "begin_send must queue the turn");
}

/// Serve one canned HTTP response per accepted connection, in order,
/// recording every raw request into the returned log. The serving thread is
/// detached: the shared loop's corrective budgets make the exact request
/// count choreography-dependent, so unused canned responses must never hang
/// the test on a join.
fn spawn_sequential_chat_server(responses: Vec<String>) -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local chat server");
    let address = listener.local_addr().expect("local chat address");
    let requests = Arc::new(Mutex::new(Vec::new()));
    let log = Arc::clone(&requests);
    std::thread::spawn(move || {
        for response in responses {
            let Ok((mut stream, _)) = listener.accept() else {
                return;
            };
            let mut request = Vec::new();
            let mut chunk = [0_u8; 8192];
            loop {
                let Ok(length) = stream.read(&mut chunk) else {
                    return;
                };
                request.extend_from_slice(&chunk[..length]);
                if length == 0 || request_complete(&request) {
                    break;
                }
            }
            log.lock()
                .expect("request log lock")
                .push(String::from_utf8_lossy(&request).into_owned());
            let _ = stream.write_all(response.as_bytes());
        }
    });
    (format!("http://{address}"), requests)
}

/// True once the buffered request holds its whole `Content-Length` body.
fn request_complete(raw: &[u8]) -> bool {
    let text = String::from_utf8_lossy(raw);
    let Some(header_end) = text.find("\r\n\r\n") else {
        return false;
    };
    let content_length = text
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().ok())?
        })
        .unwrap_or(0);
    raw.len() >= header_end + 4 + content_length
}

fn sse_ok_response(events: &[&str]) -> String {
    let body: String = events
        .iter()
        .map(|event| format!("data: {event}\n\n"))
        .collect();
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    )
}

/// A voluntary model stop with no tool calls.
fn stop_response() -> String {
    sse_ok_response(&[
        r#"{"choices":[{"delta":{"content":"Done — the login screen is on the canvas."}}]}"#,
        r#"{"choices":[{"delta":{},"finish_reason":"stop"}]}"#,
        "[DONE]",
    ])
}

fn pump_to_completion(chat_host: &mut MobileChatHost, host: &mut WidgetHostNative) {
    let started = Instant::now();
    let mut now_ms = 10;
    loop {
        let wake = chat_host.pump(host, now_ms, TEST_VIEWPORT);
        if wake.is_none() {
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "design turn did not complete within the test deadline"
        );
        std::thread::sleep(Duration::from_millis(5));
        now_ms += STREAM_POLL_INTERVAL_MS;
    }
}

fn pump_to_completion_asserting_first_root_fit(
    chat_host: &mut MobileChatHost,
    host: &mut WidgetHostNative,
    root_name: &str,
) {
    let started = Instant::now();
    let mut now_ms = 10;
    let mut observed_first_root = false;
    loop {
        let wake = chat_host.pump(host, now_ms, TEST_VIEWPORT);
        if !observed_first_root
            && host
                .editor_state()
                .active_children()
                .iter()
                .any(|node| node.base().name.as_deref() == Some(root_name))
        {
            assert!(
                wake.is_some(),
                "the first generated root must be fitted before terminal completion"
            );
            assert_mobile_content_centered(host);
            observed_first_root = true;
        }
        if wake.is_none() {
            assert!(
                observed_first_root,
                "design turn never produced {root_name}"
            );
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(60),
            "design turn did not complete within the test deadline"
        );
        std::thread::sleep(Duration::from_millis(5));
        now_ms += STREAM_POLL_INTERVAL_MS;
    }
}

fn assert_mobile_content_centered(host: &WidgetHostNative) {
    let scene = op_pen_loader::editor_state_to_active_page_layout_scene(host.editor_state());
    let content = scene
        .content_bounds()
        .expect("generated content has bounds");
    let (_canvas_x, _canvas_y, canvas_w, canvas_h) =
        op_editor_ui::widgets::host_canvas_geometry::canvas_region(
            host.editor_state(),
            TEST_VIEWPORT.0,
            TEST_VIEWPORT.1,
        );
    let viewport = host.editor_state().viewport;
    let content_center_x = content.origin.x + content.size.x / 2.0;
    let content_center_y = content.origin.y + content.size.y / 2.0;
    assert!(
        (viewport.pan_x + content_center_x * viewport.zoom - canvas_w / 2.0).abs() < 0.5,
        "generated output must be horizontally centered: {viewport:?}"
    );
    assert!(
        (viewport.pan_y + content_center_y * viewport.zoom - canvas_h / 2.0).abs() < 0.5,
        "generated output must be vertically centered: {viewport:?}"
    );
    let left = viewport.pan_x + content.origin.x * viewport.zoom;
    let top = viewport.pan_y + content.origin.y * viewport.zoom;
    let right = viewport.pan_x + (content.origin.x + content.size.x) * viewport.zoom;
    let bottom = viewport.pan_y + (content.origin.y + content.size.y) * viewport.zoom;
    assert!(
        left >= 0.0 && top >= 0.0 && right <= canvas_w && bottom <= canvas_h,
        "generated output must be fully visible in the mobile canvas"
    );
}

fn last_assistant(host: &WidgetHostNative) -> op_editor_core::ChatMessage {
    host.editor_state()
        .chat
        .messages
        .iter()
        .rev()
        .find(|message| message.role == ChatRole::Assistant)
        .expect("transcript holds an assistant bubble")
        .clone()
}

/// SSE events for one model turn that calls `batch_design` with an
/// `operations` DSL program building a small but FILLED login screen (an
/// empty screen-shaped frame would trip the loop's promise-delivery fill
/// rounds and stretch the choreography).
fn tool_call_turn_events() -> Vec<String> {
    let operations = concat!(
        r##"root=I(null,{"type":"frame","name":"Login","width":390,"height":844,"fill":"#111318","layout":"vertical","padding":24,"gap":16})"##,
        "\n",
        r##"title=I(root,{"type":"text","content":"Welcome back","fontSize":28,"fontWeight":"700","textColor":"#FFFFFF"})"##,
        "\n",
        r##"sub=I(root,{"type":"text","content":"Sign in to continue","fontSize":15,"textColor":"#9CA3AF"})"##,
        "\n",
        r##"btn=I(root,{"type":"frame","name":"SignIn","width":"fill_container","height":48,"fill":"#3B82F6","layout":"vertical","alignItems":"center","justifyContent":"center"})"##,
        "\n",
        r##"btnLabel=I(btn,{"type":"text","content":"Sign in","fontSize":16,"fontWeight":"600","textColor":"#FFFFFF"})"##,
    );
    let args = serde_json::json!({ "operations": operations }).to_string();
    let call = serde_json::json!({
        "choices": [{
            "delta": {
                "tool_calls": [{
                    "index": 0,
                    "id": "call_design_1",
                    "function": { "name": "batch_design", "arguments": args },
                }],
            },
        }],
    });
    vec![
        r#"{"choices":[{"delta":{"content":"Building the login screen."}}]}"#.to_string(),
        call.to_string(),
        r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#.to_string(),
        "[DONE]".to_string(),
    ]
}

#[test]
fn design_request_runs_tool_loop_inserts_nodes_and_fits_output() {
    let turn_one = tool_call_turn_events();
    // Turn 1 calls batch_design; turn 2 stops. Extra stop responses absorb
    // any corrective rounds (fill / blocker nudges) the shared loop decides
    // to spend — unused ones are never served.
    let mut responses = vec![sse_ok_response(
        &turn_one.iter().map(String::as_str).collect::<Vec<_>>(),
    )];
    responses.extend(std::iter::repeat_with(stop_response).take(7));
    let (base_url, requests) = spawn_sequential_chat_server(responses);
    let mut host = host_with_builtin_provider(&base_url);
    let size_class = op_editor_core::size_class::size_class(TEST_VIEWPORT.0, TEST_VIEWPORT.1);
    {
        let ui = &mut host.editor_state_mut().editor_ui;
        ui.touch = true;
        ui.size_class = size_class;
        ui.sidebar_open = size_class.is_rail_layout();
    }
    // Recreate the screenshot's exact failure contract without hardcoded
    // camera numbers: fit the canonical 1200x800 starter, then let design
    // launch clear it while retaining that now-stale viewport.
    host.mark_editor_state_dirty();
    host.fit_content_to_viewport(TEST_VIEWPORT.0, TEST_VIEWPORT.1);
    let stale_starter_camera = host.editor_state().viewport;
    send_user_message(&mut host, "设计一个暗色的登录页面");

    let mut chat_host = MobileChatHost::default();
    pump_to_completion_asserting_first_root_fit(&mut chat_host, &mut host, "Login");

    // The design landed as REAL document nodes, not HTML text.
    let login = host
        .editor_state()
        .active_children()
        .iter()
        .find(|node| node.base().name.as_deref() == Some("Login"))
        .expect("batch_design must insert the Login frame into the document");
    assert!(matches!(login, jian_ops_schema::node::PenNode::Frame(_)));
    assert!(
        login
            .children()
            .is_some_and(|children| !children.is_empty()),
        "the login screen keeps its children"
    );

    // Transcript: narration + a finished (not running) batch_design card.
    let reply = last_assistant(&host);
    assert!(!reply.streaming, "a finished design turn stops streaming");
    assert!(reply.content.contains("Building the login screen."));
    assert!(reply
        .content
        .contains("Done — the login screen is on the canvas."));
    let card = reply
        .tool_calls
        .iter()
        .find(|call| call.name == "batch_design")
        .expect("the batch_design call rides the transcript as a tool card");
    let envelope: serde_json::Value = serde_json::from_str(&card.args).expect("card envelope");
    assert_eq!(envelope["status"], "done");
    assert_eq!(envelope["result"]["success"], true);
    // Design-loop turns interleave cards into the narration timeline.
    assert!(card.content_offset.is_some());

    // The designing header cleared once the loop retired.
    assert_eq!(host.editor_state().chat.agents_running, (0, 0));

    // Mobile generation must not leave the new portrait artboard in the
    // retired starter frame's tiny camera. The shared Fit action uses the
    // touch-aware canvas region (app bar + dock), so assert geometry instead
    // of one device's pixels.
    let viewport = host.editor_state().viewport;
    assert_ne!(viewport, stale_starter_camera, "design output must refit");
    assert_mobile_content_centered(&host);

    let requests = requests.lock().expect("request log lock");
    assert!(requests.len() >= 2, "tool round trip takes two turns");
    // Turn 1 advertises the shared toolset + the design-agent system prompt.
    assert!(requests[0].contains("\"tools\""));
    assert!(requests[0].contains("batch_design"));
    assert!(requests[0].contains("product designer"));
    // DeepSeek's model profile marks thinking_disabled for design turns and
    // its family is on the wire-control whitelist.
    assert!(requests[0].contains("\"thinking\":{\"type\":\"disabled\"}"));
    // Turn 2 replays the tool result correlated by id.
    assert!(requests[1].contains("call_design_1"));
    assert!(requests[1].contains("\"role\":\"tool\""));
}

/// Script-mode `batch_design` — the desktop generation protocol — must
/// execute on the mobile host too (rquickjs via bindgen on the mobile
/// targets; the same code path runs host-side in this test).
#[test]
fn design_request_executes_script_mode_batch_design() {
    let script = r##"
        const cards = [["Recently played", 4], ["Made for you", 6]];
        const root = I(null, {type:"frame", name:"ScriptHome", width:390, height:844, fill:"#0B0B10", layout:"vertical", padding:20, gap:12});
        for (const [label, count] of cards) {
            const section = I(root, {type:"frame", layout:"vertical", width:"fill_container", gap:8});
            I(section, {type:"text", content:label, fontSize:18, fontWeight:"700", textColor:"#FFFFFF"});
            for (let i = 0; i < count; i++) {
                I(section, {type:"text", content:"Track " + (i + 1), fontSize:14, textColor:"#A1A1AA"});
            }
        }
    "##;
    let args = serde_json::json!({ "script": script }).to_string();
    let call = serde_json::json!({
        "choices": [{
            "delta": {
                "tool_calls": [{
                    "index": 0,
                    "id": "call_script_1",
                    "function": { "name": "batch_design", "arguments": args },
                }],
            },
        }],
    });
    let turn_one = [
        call.to_string(),
        r#"{"choices":[{"delta":{},"finish_reason":"tool_calls"}]}"#.to_string(),
        "[DONE]".to_string(),
    ];
    let mut responses = vec![sse_ok_response(
        &turn_one.iter().map(String::as_str).collect::<Vec<_>>(),
    )];
    responses.extend(std::iter::repeat_with(stop_response).take(7));
    let (base_url, _requests) = spawn_sequential_chat_server(responses);
    let mut host = host_with_builtin_provider(&base_url);
    send_user_message(&mut host, "设计一个音乐首页");

    let mut chat_host = MobileChatHost::default();
    pump_to_completion(&mut chat_host, &mut host);

    let home = host
        .editor_state()
        .active_children()
        .iter()
        .find(|node| node.base().name.as_deref() == Some("ScriptHome"))
        .expect("script-mode batch_design must insert the ScriptHome frame");
    // Finalization prepends the canonical mobile status bar; the JS loop's
    // two content sections remain after it and each carries a title + tracks.
    let children = home.children().expect("script home has children");
    let (status_bar, sections) = children.split_first().expect("script home is not empty");
    assert_eq!(status_bar.base().role.as_deref(), Some("status-bar"));
    assert_eq!(sections.len(), 2);
    assert_eq!(sections[0].children().map(|c| c.len()), Some(1 + 4));
    assert_eq!(sections[1].children().map(|c| c.len()), Some(1 + 6));
    let card = last_assistant(&host)
        .tool_calls
        .iter()
        .find(|call| call.name == "batch_design")
        .cloned()
        .expect("script call rides the transcript");
    let envelope: serde_json::Value = serde_json::from_str(&card.args).expect("card envelope");
    assert_eq!(envelope["status"], "done");
    assert_eq!(envelope["result"]["success"], true);
}

#[test]
fn plain_chat_request_keeps_the_plain_streaming_path() {
    let (base_url, requests) = spawn_sequential_chat_server(vec![sse_ok_response(&[
        r#"{"choices":[{"delta":{"content":"A frame is a container."}}]}"#,
        "[DONE]",
    ])]);
    let mut host = host_with_builtin_provider(&base_url);
    let camera = Viewport {
        pan_x: -37.0,
        pan_y: 91.0,
        zoom: 1.25,
    };
    host.editor_state_mut().viewport = camera;
    send_user_message(&mut host, "what is a frame?");

    let mut chat_host = MobileChatHost::default();
    pump_to_completion(&mut chat_host, &mut host);

    assert_eq!(last_assistant(&host).content, "A frame is a container.");
    assert_eq!(
        host.editor_state().viewport,
        camera,
        "ordinary chat must not move the canvas"
    );
    let requests = requests.lock().expect("request log lock");
    // Plain turns advertise no tools (and carry no design prompt).
    assert!(!requests[0].contains("\"tools\""));
    assert!(!requests[0].contains("product designer"));
}

#[test]
fn design_loop_provider_error_lands_in_bubble_and_clears_designing_header() {
    let (base_url, _requests) = spawn_sequential_chat_server(vec![
        "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            .to_string(),
    ]);
    let mut host = host_with_builtin_provider(&base_url);
    let camera = Viewport {
        pan_x: 23.0,
        pan_y: -19.0,
        zoom: 0.75,
    };
    host.editor_state_mut().viewport = camera;
    send_user_message(&mut host, "design a landing page");

    let mut chat_host = MobileChatHost::default();
    pump_to_completion(&mut chat_host, &mut host);

    let reply = last_assistant(&host);
    assert!(
        reply.content.starts_with("error: "),
        "provider failure must surface, got {:?}",
        reply.content
    );
    assert!(reply.content.contains("http 500"));
    assert!(!reply.streaming);
    assert_eq!(host.editor_state().chat.agents_running, (0, 0));
    assert_eq!(
        host.editor_state().viewport,
        camera,
        "a zero-write provider failure must preserve the user's camera"
    );
}

/// Real end-to-end run against DeepSeek. Ignored by default; run with
/// `OPENPENCIL_TEST_DEEPSEEK_KEY=sk-… cargo test -p op-engine-ffi \
/// --features editor real_deepseek -- --ignored --nocapture`.
#[test]
#[ignore = "needs a real DeepSeek API key + network"]
fn real_deepseek_design_turn_inserts_nodes() {
    let Ok(key) = std::env::var("OPENPENCIL_TEST_DEEPSEEK_KEY") else {
        panic!("set OPENPENCIL_TEST_DEEPSEEK_KEY to run this test");
    };
    // Model override so the same run covers the weaker variants the real
    // app selects (e.g. deepseek-v4-flash).
    let model = std::env::var("OPENPENCIL_TEST_DEEPSEEK_MODEL")
        .unwrap_or_else(|_| "deepseek-chat".to_string());
    // OPENPENCIL_TEST_DEEPSEEK_WIRE=anthropic drives DeepSeek's
    // Anthropic-compatible endpoint — the preset's alternate API format the
    // real device config selected in the empty-canvas incident.
    let anthropic_wire = std::env::var("OPENPENCIL_TEST_DEEPSEEK_WIRE")
        .is_ok_and(|wire| wire.eq_ignore_ascii_case("anthropic"));
    let (kind, base_url) = if anthropic_wire {
        (
            BuiltinAgentKind::Anthropic,
            "https://api.deepseek.com/anthropic",
        )
    } else {
        (BuiltinAgentKind::OpenAiCompat, "https://api.deepseek.com")
    };
    eprintln!("--- e2e model: {model}, wire: {kind:?} ({base_url})");
    let mut host = WidgetHostNative::new();
    let settings = &mut host.editor_state_mut().editor_ui.agent_settings;
    settings.builtin_agents.clear();
    settings.add_builtin_agent_config("DeepSeek", &key, &model, kind, base_url);
    host.editor_state_mut().rebuild_chat_models();
    let chat = &mut host.editor_state_mut().chat;
    let builtin_index = chat
        .available_models
        .iter()
        .position(|entry| entry.builtin_provider_id.is_some())
        .expect("builtin model entry");
    chat.selected_model = builtin_index;
    send_user_message(
        &mut host,
        "设计一个带图片的旅行 App 首页(390x844)，包含目的地卡片配图和顶部 hero 大图",
    );

    let mut chat_host = MobileChatHost::default();
    let started = Instant::now();
    let mut now_ms = 10;
    loop {
        if chat_host.pump(&mut host, now_ms, TEST_VIEWPORT).is_none() {
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(600),
            "real design turn did not complete within 10 minutes"
        );
        std::thread::sleep(Duration::from_millis(20));
        now_ms += STREAM_POLL_INTERVAL_MS;
    }

    let reply = last_assistant(&host);
    let mut script_calls = 0usize;
    let mut operations_calls = 0usize;
    for call in &reply.tool_calls {
        if call.name != "batch_design" {
            continue;
        }
        let args: serde_json::Value = serde_json::from_str(&call.args).unwrap_or_default();
        let inner = &args["args"];
        if inner.get("script").is_some() {
            script_calls += 1;
        }
        if inner.get("operations").is_some() {
            operations_calls += 1;
        }
    }
    let tool_names: Vec<&str> = reply
        .tool_calls
        .iter()
        .map(|call| call.name.as_str())
        .collect();
    eprintln!("--- transcript content ---\n{}", reply.content);
    eprintln!("--- tool calls: {tool_names:?}");
    eprintln!(
        "--- batch_design script calls: {script_calls}, operations calls: {operations_calls}"
    );
    eprintln!(
        "--- top-level nodes: {:?}",
        host.editor_state()
            .active_children()
            .iter()
            .map(|node| node.base().name.clone())
            .collect::<Vec<_>>()
    );
    assert!(
        !host.editor_state().active_children().is_empty(),
        "a real design turn must land nodes in the document"
    );
    assert!(
        tool_names.contains(&"batch_design"),
        "the model must build through batch_design, got {tool_names:?}"
    );
    assert!(
        !reply.content.contains("<html") && !reply.content.contains("<!DOCTYPE"),
        "the transcript must not be an HTML page"
    );

    // Image search: pump the mobile enrichment session (real Openverse /
    // Wikimedia ladder, anonymous tier) until every slot settles, then
    // require at least one slot to have resolved to a self-contained image.
    let mut image_search = crate::editor_image_search::MobileImageSearch::default();
    let started = Instant::now();
    let mut now_ms = 10;
    while image_search.pump(&mut host, now_ms).is_some() {
        assert!(
            started.elapsed() < Duration::from_secs(180),
            "image search did not settle within 3 minutes"
        );
        std::thread::sleep(Duration::from_millis(50));
        now_ms += 120;
    }
    let mut resolved = 0usize;
    let mut failed = 0usize;
    let mut pending = 0usize;
    fn walk(nodes: &[jian_ops_schema::node::PenNode], r: &mut usize, f: &mut usize, p: &mut usize) {
        for node in nodes {
            if let jian_ops_schema::node::PenNode::Image(image) = node {
                let src = image.src.as_str();
                if src.starts_with("data:") || src.starts_with("http") {
                    *r += 1;
                } else if src.contains("image-search-failed") {
                    *f += 1;
                } else if src.trim().is_empty() {
                    *p += 1;
                }
            }
            if let Some(children) = node.children() {
                walk(children, r, f, p);
            }
        }
    }
    walk(
        host.editor_state().active_children(),
        &mut resolved,
        &mut failed,
        &mut pending,
    );
    eprintln!("--- image slots: resolved={resolved} failed={failed} pending={pending}");
    assert_eq!(pending, 0, "no slot may stay eternally unresolved");
    // `resolved` depends on the anonymous Openverse/Wikimedia quota at run
    // time — back-to-back e2e runs can exhaust it and legitimately land
    // every slot on the failed-search placeholder. That is the designed
    // degradation, not a pipeline fault, so an all-failed run only warns.
    if resolved == 0 {
        eprintln!(
            "--- WARNING: every image search failed (likely provider rate              limiting); pipeline behavior (placeholder fills, no pending              slots) is still verified"
        );
    }
}
