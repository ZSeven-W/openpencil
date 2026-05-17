//! ndJSON framing — newline-delimited JSON over an async byte stream.
//!
//! ACP speaks JSON-RPC where every message is one JSON object on its
//! own line (the SDK's `ndJsonStream`). These helpers read and write
//! that framing; the JSON-RPC engine ([`crate::jsonrpc`]) sits on top.

use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

use crate::types::AcpError;

/// Read the next ndJSON frame from `reader`. Blank lines are skipped;
/// a line that fails to parse as JSON is skipped (ndJSON robustness —
/// a stray non-JSON line must not kill the connection). Returns
/// `Ok(None)` at end of stream.
pub async fn read_frame(
    reader: &mut (impl AsyncBufRead + Unpin),
) -> Result<Option<Value>, AcpError> {
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .await
            .map_err(|e| AcpError::Transport(e.to_string()))?;
        if n == 0 {
            return Ok(None);
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(trimmed) {
            Ok(v) => return Ok(Some(v)),
            // Skip a malformed line rather than tearing down the link.
            Err(_) => continue,
        }
    }
}

/// `AsyncBufRead` is needed for `read_line`; re-exported so callers
/// don't need a separate `tokio::io` import.
pub use tokio::io::AsyncBufRead;

/// Write one ndJSON frame: the compact JSON of `value` followed by a
/// newline, flushed.
pub async fn write_frame(
    writer: &mut (impl AsyncWrite + Unpin),
    value: &Value,
) -> Result<(), AcpError> {
    let mut bytes =
        serde_json::to_vec(value).map_err(|e| AcpError::Protocol(e.to_string()))?;
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
        // EOF.
        assert!(read_frame(&mut reader).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn skips_blank_and_malformed_lines() {
        let data = "\n  \nnot json at all\n{\"ok\":true}\n";
        let mut reader = Cursor::new(data.as_bytes());
        let f = read_frame(&mut reader).await.unwrap().unwrap();
        assert_eq!(f["ok"], true);
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
