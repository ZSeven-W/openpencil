use super::*;
use op_ai::chat_provider::{ChatDelta, ChatRequest, ChatToolExecutor, StopReason};
use op_editor_core::{ChatMessage, PenNodeExt};

struct BatchThenFinalizeProvider {
    executor: std::sync::Arc<dyn ChatToolExecutor>,
}

struct BatchThenFinalizeIter {
    executor: std::sync::Arc<dyn ChatToolExecutor>,
    step: u8,
}

impl Iterator for BatchThenFinalizeIter {
    type Item = ChatDelta;

    fn next(&mut self) -> Option<ChatDelta> {
        self.step += 1;
        match self.step {
            1 => {
                let args = r#"{"operations":"root=I(null,{type:'frame',name:'Header',width:1200,height:64})"}"#;
                let _ = self.executor.execute("batch_design", args);
                Some(ChatDelta::TextDelta("inserted".into()))
            }
            2 => {
                self.executor.finalize();
                Some(ChatDelta::Done {
                    stop_reason: StopReason::EndTurn,
                })
            }
            _ => None,
        }
    }
}

impl op_ai::chat_provider::ChatProvider for BatchThenFinalizeProvider {
    fn provider_label(&self) -> &str {
        "batch-finalize"
    }

    fn send(&self, _request: ChatRequest) -> Box<dyn Iterator<Item = ChatDelta> + Send> {
        Box::new(BatchThenFinalizeIter {
            executor: self.executor.clone(),
            step: 0,
        })
    }
}

#[test]
fn pump_defers_loop_finalize_until_registered_reveals_drain() {
    let _guard = crate::agent_indicator_test_lock::LOCK
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    op_editor_core::agent_indicators::clear();
    let epoch = op_editor_core::agent_indicators::begin();

    let mut host = WidgetHostNative::new();
    host.editor_state_mut()
        .chat
        .messages
        .push(ChatMessage::assistant_streaming());
    let (executor, tool_rx) = op_host_services::chat_canvas_tools::chat_tool_channel();
    let provider = Box::new(BatchThenFinalizeProvider {
        executor: std::sync::Arc::new(executor),
    });
    let mut current = Some(ChatSession::start_with_tools(
        provider,
        ChatRequest {
            user_message: "build a header".into(),
            max_output_tokens: 64,
            ..Default::default()
        },
        Some(tool_rx),
    ));

    for _ in 0..200 {
        pump(&mut host, &mut current, None, None, (1200.0, 800.0));
        if op_editor_core::agent_indicators::latest_reveal_end_ms(epoch).is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let reveal_end = op_editor_core::agent_indicators::latest_reveal_end_ms(epoch)
        .expect("batch_design should register reveals before finalize");

    for _ in 0..20 {
        pump(&mut host, &mut current, None, None, (1200.0, 800.0));
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let header = header_node(&host).expect("Header frame inserted");
    assert_eq!(
        header.base().role.as_deref(),
        None,
        "finalize must not run while the reveal queue is still draining"
    );

    let now = reveal_now_millis();
    if reveal_end > now {
        std::thread::sleep(std::time::Duration::from_millis(reveal_end - now + 20));
    }
    for _ in 0..200 {
        pump(&mut host, &mut current, None, None, (1200.0, 800.0));
        if current.is_none() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(current.is_none(), "turn must finish after reveal drain");
    let header = header_node(&host).expect("Header frame still present");
    assert_eq!(
        header.base().role.as_deref(),
        Some("navbar"),
        "loop-end finalize should run after the reveal queue drains"
    );

    op_editor_core::agent_indicators::end_if_epoch(epoch);
    op_editor_core::agent_indicators::clear();
}

fn header_node(host: &WidgetHostNative) -> Option<&jian_ops_schema::node::PenNode> {
    host.editor_state()
        .active_children()
        .iter()
        .find(|node| node.base().name.as_deref() == Some("Header"))
}

fn reveal_now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}
