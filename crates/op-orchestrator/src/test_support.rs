//! 测试桩 —— crate 内各测试模块共用。仅在 `cfg(test)` 下编译。

use crate::types::{CallRequest, DocSink, LlmChunk, LlmClient, LlmError};
use futures::stream::BoxStream;
use op_editor_core::{EditorCommand, EditorState};
use std::collections::VecDeque;
use std::sync::Mutex;

/// 录命令 + 持内存 `EditorState` 的 `DocSink` 实现。
pub(crate) struct VecDocSink {
    pub state: EditorState,
    pub applied: Vec<EditorCommand>,
    pub batch_depth: i32,
}

impl VecDocSink {
    pub(crate) fn new() -> Self {
        Self {
            state: EditorState::new(),
            applied: Vec::new(),
            batch_depth: 0,
        }
    }
}

impl DocSink for VecDocSink {
    fn state(&self) -> &EditorState {
        &self.state
    }
    fn apply(&mut self, cmd: EditorCommand) -> bool {
        self.applied.push(cmd.clone());
        self.state.apply(cmd)
    }
    fn begin_undo_batch(&mut self) {
        self.batch_depth += 1;
    }
    fn end_undo_batch(&mut self) {
        self.batch_depth -= 1;
    }
}

/// 一次脚本化的 LLM 响应。
pub(crate) enum ScriptResponse {
    /// 一段成功文本(作为单个 `Text` chunk 返回)。
    Text(String),
    /// 一次失败。
    Fail(LlmError),
}

/// 按脚本逐次返回响应的 `LlmClient` 桩 —— 每次 `call` 弹一条。
pub(crate) struct ScriptedLlm {
    responses: Mutex<VecDeque<ScriptResponse>>,
}

impl ScriptedLlm {
    pub(crate) fn new(responses: Vec<ScriptResponse>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
        }
    }
}

impl LlmClient for ScriptedLlm {
    fn call(&self, _req: CallRequest) -> BoxStream<'static, Result<LlmChunk, LlmError>> {
        let next = self.responses.lock().unwrap().pop_front();
        let items: Vec<Result<LlmChunk, LlmError>> = match next {
            Some(ScriptResponse::Text(t)) => vec![Ok(LlmChunk::Text(t))],
            Some(ScriptResponse::Fail(e)) => vec![Err(e)],
            None => vec![Err(LlmError {
                message: "scripted LLM exhausted".into(),
                aborted: false,
            })],
        };
        Box::pin(futures::stream::iter(items))
    }
}
