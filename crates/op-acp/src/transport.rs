//! ndJSON framing — newline-delimited JSON over an async byte stream.
//!
//! ACP speaks JSON-RPC where every message is one JSON object on its
//! own line (the SDK's `ndJsonStream`). These helpers read and write
//! that framing; the JSON-RPC engine ([`crate::jsonrpc`]) sits on top.

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

use crate::types::AcpError;

/// Maximum accepted inbound ACP message size for stdio and WebSocket peers.
/// The limit includes the JSON payload but not the ndJSON newline delimiter.
pub const MAX_INBOUND_FRAME_BYTES: usize = 16 * 1024 * 1024;

/// Read the next ndJSON frame from `reader`. Blank lines are skipped, while a
/// malformed or oversized non-empty frame fails the connection. The bounded
/// `fill_buf` loop is intentional: `read_line` would keep allocating forever
/// when an untrusted child writes bytes without a newline.
pub async fn read_frame(
    reader: &mut (impl AsyncBufRead + Unpin),
) -> Result<Option<Value>, AcpError> {
    read_frame_with_limit(reader, MAX_INBOUND_FRAME_BYTES).await
}

async fn read_frame_with_limit(
    reader: &mut (impl AsyncBufRead + Unpin),
    max_bytes: usize,
) -> Result<Option<Value>, AcpError> {
    let mut frame = Vec::new();
    loop {
        let (consumed, reached_delimiter, reached_eof) = {
            let available = reader
                .fill_buf()
                .await
                .map_err(|e| AcpError::Transport(e.to_string()))?;
            if available.is_empty() {
                (0, false, true)
            } else if let Some(index) = available.iter().position(|byte| *byte == b'\n') {
                if frame.len().saturating_add(index) > max_bytes {
                    return Err(AcpError::Protocol(format!(
                        "ACP frame exceeds {max_bytes} bytes"
                    )));
                }
                frame.extend_from_slice(&available[..index]);
                (index + 1, true, false)
            } else {
                if frame.len().saturating_add(available.len()) > max_bytes {
                    return Err(AcpError::Protocol(format!(
                        "ACP frame exceeds {max_bytes} bytes"
                    )));
                }
                frame.extend_from_slice(available);
                (available.len(), false, false)
            }
        };
        reader.consume(consumed);

        if !reached_delimiter && !reached_eof {
            continue;
        }
        if frame.iter().all(u8::is_ascii_whitespace) {
            if reached_eof {
                return Ok(None);
            }
            frame.clear();
            continue;
        }
        return serde_json::from_slice::<Value>(&frame)
            .map(Some)
            .map_err(|error| AcpError::Protocol(format!("invalid ACP JSON frame: {error}")));
    }
}

/// `AsyncBufRead` is needed for `fill_buf`; re-exported so callers
/// don't need a separate `tokio::io` import.
pub use tokio::io::AsyncBufRead;

/// Write one ndJSON frame: the compact JSON of `value` followed by a
/// newline, flushed.
pub async fn write_frame(
    writer: &mut (impl AsyncWrite + Unpin),
    value: &Value,
) -> Result<(), AcpError> {
    let mut bytes = serde_json::to_vec(value).map_err(|e| AcpError::Protocol(e.to_string()))?;
    bytes.push(b'\n');
    writer
        .write_all(&bytes)
        .await
        .map_err(|e| AcpError::Transport(e.to_string()))?;
    writer
        .flush()
        .await
        .map_err(|e| AcpError::Transport(e.to_string()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn reads_frames_split_on_newline() {
        let data = "{\"a\":1}\n{\"b\":2}\n";
        let mut reader = Cursor::new(data.as_bytes());
        let f1 = read_frame(&mut reader).await.unwrap().unwrap();
        assert_eq!(f1["a"], 1);
        let f2 = read_frame(&mut reader).await.unwrap().unwrap();
        assert_eq!(f2["b"], 2);
        assert!(read_frame(&mut reader).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn skips_blank_lines_but_rejects_malformed_frames() {
        let data = "\n  \nnot json at all\n{\"ok\":true}\n";
        let mut reader = Cursor::new(data.as_bytes());
        let error = read_frame(&mut reader).await.unwrap_err();
        assert!(error.to_string().contains("invalid ACP JSON frame"));
    }

    #[tokio::test]
    async fn rejects_an_oversized_frame_without_waiting_for_a_newline() {
        let mut reader = Cursor::new(vec![b'x'; 33]);
        let error = read_frame_with_limit(&mut reader, 32).await.unwrap_err();
        assert!(error.to_string().contains("exceeds 32 bytes"));
    }

    #[tokio::test]
    async fn accepts_a_final_frame_without_a_newline() {
        let mut reader = Cursor::new(br#"{"ok":true}"#);
        let frame = read_frame_with_limit(&mut reader, 32)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(frame["ok"], true);
    }

    #[tokio::test]
    async fn write_frame_appends_newline() {
        let mut buf: Vec<u8> = Vec::new();
        write_frame(&mut buf, &serde_json::json!({"x":1}))
            .await
            .unwrap();
        assert_eq!(buf, b"{\"x\":1}\n");
    }
}
