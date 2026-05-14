// Copyright (c) 2026 Elias Bachaalany
// SPDX-License-Identifier: MIT

//! Transport layer for the Copilot SDK.
//!
//! Provides async byte I/O and Content-Length message framing (LSP-style).

use crate::error::{CopilotError, Result};
use std::future::Future;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader};

// =============================================================================
// Transport Trait
// =============================================================================

/// Async transport for raw byte I/O.
///
/// Implementations provide the underlying byte stream (stdio pipes, TCP sockets, etc.)
/// The transport is responsible for reading/writing raw bytes; framing is handled
/// separately by `MessageFramer`.
pub trait Transport: Send + Sync {
    /// Read up to `buf.len()` bytes into buffer.
    /// Returns the number of bytes read (0 indicates EOF).
    fn read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + Send + 'a>>;

    /// Write all bytes to the transport.
    fn write<'a>(
        &'a mut self,
        data: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>>;

    /// Close the transport.
    fn close(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;

    /// Check if transport is open.
    fn is_open(&self) -> bool;
}

// =============================================================================
// Stdio Transport
// =============================================================================

/// Transport using stdin/stdout of a child process.
pub struct StdioTransport {
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
    open: bool,
}

impl StdioTransport {
    /// Create a new stdio transport from child process handles.
    pub fn new(stdin: tokio::process::ChildStdin, stdout: tokio::process::ChildStdout) -> Self {
        Self {
            stdin,
            stdout: BufReader::new(stdout),
            open: true,
        }
    }

    /// Split into separate read and write handles.
    pub fn split(self) -> (tokio::process::ChildStdin, tokio::process::ChildStdout) {
        (self.stdin, self.stdout.into_inner())
    }
}

impl Transport for StdioTransport {
    fn read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + Send + 'a>> {
        Box::pin(async move {
            if !self.open {
                return Err(CopilotError::ConnectionClosed);
            }
            self.stdout.read(buf).await.map_err(CopilotError::Transport)
        })
    }

    fn write<'a>(
        &'a mut self,
        data: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            if !self.open {
                return Err(CopilotError::ConnectionClosed);
            }
            self.stdin
                .write_all(data)
                .await
                .map_err(CopilotError::Transport)?;
            // Flush may fail on Windows with pipes, but data is still written
            let _ = self.stdin.flush().await;
            Ok(())
        })
    }

    fn close(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            self.open = false;
            Ok(())
        })
    }

    fn is_open(&self) -> bool {
        self.open
    }
}

// =============================================================================
// Content-Length Message Framer (LSP-style)
// =============================================================================

/// Handles Content-Length header framing for JSON-RPC messages.
///
/// Message format:
/// ```text
/// Content-Length: <length>\r\n
/// \r\n
/// <json-rpc-message>
/// ```
///
/// This is the standard LSP (Language Server Protocol) framing used by
/// StreamJsonRpc's HeaderDelimitedMessageHandler.
pub struct MessageFramer<T: Transport> {
    transport: T,
    buffer: Vec<u8>,
    buffer_pos: usize,
    buffer_len: usize,
}

/// Message writer for framing outgoing messages.
pub struct MessageWriter<W> {
    writer: W,
}

impl<W> MessageWriter<W>
where
    W: AsyncWrite + Unpin + Send,
{
    /// Create a new message writer.
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    /// Write a message with Content-Length framing.
    pub async fn write_message(&mut self, message: &str) -> Result<()> {
        let frame = format!("Content-Length: {}\r\n\r\n{}", message.len(), message);
        self.writer
            .write_all(frame.as_bytes())
            .await
            .map_err(CopilotError::Transport)?;
        // Flush may fail on Windows with pipes, but data is still written.
        let _ = self.writer.flush().await;
        Ok(())
    }
}

/// Message reader for parsing incoming framed messages.
pub struct MessageReader<R> {
    reader: BufReader<R>,
    buffer: Vec<u8>,
    buffer_pos: usize,
    buffer_len: usize,
}

impl<R> MessageReader<R>
where
    R: AsyncRead + Unpin + Send,
{
    /// Create a new message reader.
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
            buffer: vec![0u8; 4096],
            buffer_pos: 0,
            buffer_len: 0,
        }
    }

    /// Read a complete framed message.
    pub async fn read_message(&mut self) -> Result<String> {
        // Read headers until empty line
        let mut content_length: Option<usize> = None;

        loop {
            let line = self.read_line().await?;

            // Empty line signals end of headers
            if line.is_empty() {
                break;
            }

            // Parse Content-Length header (case-insensitive)
            let lower_line = line.to_lowercase();
            if let Some(value) = lower_line.strip_prefix("content-length:") {
                let value_str = value.trim();
                content_length = Some(value_str.parse().map_err(|_| {
                    CopilotError::Protocol(format!("Invalid Content-Length value: {}", value_str))
                })?);
            }
        }

        let content_length = content_length
            .ok_or_else(|| CopilotError::Protocol("Missing Content-Length header".into()))?;

        // Read the message body
        let mut message = vec![0u8; content_length];
        self.read_exact(&mut message).await?;

        String::from_utf8(message)
            .map_err(|e| CopilotError::Protocol(format!("Invalid UTF-8 in message: {}", e)))
    }

    /// Read exactly n bytes.
    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let mut total_read = 0;

        // First, use any buffered data
        while total_read < buf.len() && self.buffer_pos < self.buffer_len {
            buf[total_read] = self.buffer[self.buffer_pos];
            total_read += 1;
            self.buffer_pos += 1;
        }

        // Read remaining directly from reader
        while total_read < buf.len() {
            let bytes_read = self
                .reader
                .read(&mut buf[total_read..])
                .await
                .map_err(CopilotError::Transport)?;
            if bytes_read == 0 {
                return Err(CopilotError::ConnectionClosed);
            }
            total_read += bytes_read;
        }

        Ok(())
    }

    /// Read a single line (up to \r\n or \n).
    async fn read_line(&mut self) -> Result<String> {
        let mut line = String::new();

        loop {
            // Refill buffer if empty
            if self.buffer_pos >= self.buffer_len {
                self.fill_buffer(1).await?;
                if self.buffer_len == 0 {
                    return Err(CopilotError::ConnectionClosed);
                }
            }

            let c = self.buffer[self.buffer_pos] as char;
            self.buffer_pos += 1;

            if c == '\n' {
                // Remove trailing \r if present
                if line.ends_with('\r') {
                    line.pop();
                }
                return Ok(line);
            }

            line.push(c);
        }
    }

    /// Fill buffer with at least min_bytes.
    async fn fill_buffer(&mut self, min_bytes: usize) -> Result<()> {
        // Compact buffer if needed
        if self.buffer_pos > 0 {
            if self.buffer_pos < self.buffer_len {
                self.buffer.copy_within(self.buffer_pos..self.buffer_len, 0);
                self.buffer_len -= self.buffer_pos;
            } else {
                self.buffer_len = 0;
            }
            self.buffer_pos = 0;
        }

        // Read more data
        while self.buffer_len < min_bytes {
            let bytes_read = self
                .reader
                .read(&mut self.buffer[self.buffer_len..])
                .await?;

            if bytes_read == 0 {
                // EOF - return what we have
                return Ok(());
            }

            self.buffer_len += bytes_read;
        }

        Ok(())
    }
}

impl<T: Transport> MessageFramer<T> {
    /// Create a new message framer wrapping a transport.
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            buffer: vec![0u8; 4096],
            buffer_pos: 0,
            buffer_len: 0,
        }
    }

    /// Read a complete framed message.
    ///
    /// Returns the message content (without headers).
    pub async fn read_message(&mut self) -> Result<String> {
        // Read headers until empty line
        let mut content_length: Option<usize> = None;

        loop {
            let line = self.read_line().await?;

            // Empty line signals end of headers
            if line.is_empty() {
                break;
            }

            // Parse Content-Length header (case-insensitive)
            let lower_line = line.to_lowercase();
            if let Some(value) = lower_line.strip_prefix("content-length:") {
                let value_str = value.trim();
                content_length = Some(value_str.parse().map_err(|_| {
                    CopilotError::Protocol(format!("Invalid Content-Length value: {}", value_str))
                })?);
            }
            // Ignore other headers (e.g., Content-Type)
        }

        let content_length = content_length
            .ok_or_else(|| CopilotError::Protocol("Missing Content-Length header".into()))?;

        // Read the message body
        let mut message = vec![0u8; content_length];
        self.read_exact(&mut message).await?;

        String::from_utf8(message)
            .map_err(|e| CopilotError::Protocol(format!("Invalid UTF-8 in message: {}", e)))
    }

    /// Write a message with Content-Length framing.
    pub async fn write_message(&mut self, message: &str) -> Result<()> {
        let frame = format!("Content-Length: {}\r\n\r\n{}", message.len(), message);
        self.transport.write(frame.as_bytes()).await
    }

    /// Get a reference to the underlying transport.
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Get a mutable reference to the underlying transport.
    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }

    /// Consume the framer and return the transport.
    pub fn into_transport(self) -> T {
        self.transport
    }

    // =========================================================================
    // Private helpers
    // =========================================================================

    /// Read exactly n bytes.
    async fn read_exact(&mut self, buf: &mut [u8]) -> Result<()> {
        let mut total_read = 0;

        // First, use any buffered data
        while total_read < buf.len() && self.buffer_pos < self.buffer_len {
            buf[total_read] = self.buffer[self.buffer_pos];
            total_read += 1;
            self.buffer_pos += 1;
        }

        // Read remaining directly from transport
        while total_read < buf.len() {
            let bytes_read = self.transport.read(&mut buf[total_read..]).await?;
            if bytes_read == 0 {
                return Err(CopilotError::ConnectionClosed);
            }
            total_read += bytes_read;
        }

        Ok(())
    }

    /// Read a single line (up to \r\n or \n).
    async fn read_line(&mut self) -> Result<String> {
        let mut line = String::new();

        loop {
            // Refill buffer if empty
            if self.buffer_pos >= self.buffer_len {
                self.fill_buffer(1).await?;
                if self.buffer_len == 0 {
                    return Err(CopilotError::ConnectionClosed);
                }
            }

            let c = self.buffer[self.buffer_pos] as char;
            self.buffer_pos += 1;

            if c == '\n' {
                // Remove trailing \r if present
                if line.ends_with('\r') {
                    line.pop();
                }
                return Ok(line);
            }

            line.push(c);
        }
    }

    /// Fill buffer with at least min_bytes.
    async fn fill_buffer(&mut self, min_bytes: usize) -> Result<()> {
        // Compact buffer if needed
        if self.buffer_pos > 0 {
            if self.buffer_pos < self.buffer_len {
                self.buffer.copy_within(self.buffer_pos..self.buffer_len, 0);
                self.buffer_len -= self.buffer_pos;
            } else {
                self.buffer_len = 0;
            }
            self.buffer_pos = 0;
        }

        // Read more data
        while self.buffer_len < min_bytes {
            let bytes_read = self
                .transport
                .read(&mut self.buffer[self.buffer_len..])
                .await?;

            if bytes_read == 0 {
                // EOF - return what we have
                return Ok(());
            }

            self.buffer_len += bytes_read;
        }

        Ok(())
    }
}

// =============================================================================
// In-Memory Transport (for testing)
// =============================================================================

/// In-memory transport for testing.
#[cfg(test)]
pub struct MemoryTransport {
    read_data: Vec<u8>,
    read_pos: usize,
    write_data: Vec<u8>,
    open: bool,
}

#[cfg(test)]
impl MemoryTransport {
    /// Create a new memory transport with initial read data.
    pub fn new(read_data: Vec<u8>) -> Self {
        Self {
            read_data,
            read_pos: 0,
            write_data: Vec::new(),
            open: true,
        }
    }

    /// Get the data that was written.
    pub fn written_data(&self) -> &[u8] {
        &self.write_data
    }
}

#[cfg(test)]
impl Transport for MemoryTransport {
    fn read<'a>(
        &'a mut self,
        buf: &'a mut [u8],
    ) -> Pin<Box<dyn Future<Output = Result<usize>> + Send + 'a>> {
        Box::pin(async move {
            if !self.open {
                return Err(CopilotError::ConnectionClosed);
            }
            let remaining = self.read_data.len() - self.read_pos;
            let to_read = remaining.min(buf.len());
            buf[..to_read].copy_from_slice(&self.read_data[self.read_pos..self.read_pos + to_read]);
            self.read_pos += to_read;
            Ok(to_read)
        })
    }

    fn write<'a>(
        &'a mut self,
        data: &'a [u8],
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + 'a>> {
        Box::pin(async move {
            if !self.open {
                return Err(CopilotError::ConnectionClosed);
            }
            self.write_data.extend_from_slice(data);
            Ok(())
        })
    }

    fn close(&mut self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>> {
        Box::pin(async move {
            self.open = false;
            Ok(())
        })
    }

    fn is_open(&self) -> bool {
        self.open
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_read_message() {
        let data = b"Content-Length: 13\r\n\r\n{\"test\":true}";
        let transport = MemoryTransport::new(data.to_vec());
        let mut framer = MessageFramer::new(transport);

        let message = framer.read_message().await.unwrap();
        assert_eq!(message, "{\"test\":true}");
    }

    #[tokio::test]
    async fn test_read_message_lf_only() {
        // Some implementations use LF only
        let data = b"Content-Length: 13\n\n{\"test\":true}";
        let transport = MemoryTransport::new(data.to_vec());
        let mut framer = MessageFramer::new(transport);

        let message = framer.read_message().await.unwrap();
        assert_eq!(message, "{\"test\":true}");
    }

    #[tokio::test]
    async fn test_read_message_with_extra_headers() {
        let data = b"Content-Type: application/json\r\nContent-Length: 13\r\n\r\n{\"test\":true}";
        let transport = MemoryTransport::new(data.to_vec());
        let mut framer = MessageFramer::new(transport);

        let message = framer.read_message().await.unwrap();
        assert_eq!(message, "{\"test\":true}");
    }

    #[tokio::test]
    async fn test_write_message() {
        let transport = MemoryTransport::new(Vec::new());
        let mut framer = MessageFramer::new(transport);

        framer.write_message("{\"test\":true}").await.unwrap();

        let written = framer.transport().written_data();
        assert_eq!(written, b"Content-Length: 13\r\n\r\n{\"test\":true}");
    }

    #[tokio::test]
    async fn test_read_multiple_messages() {
        let data =
            b"Content-Length: 13\r\n\r\n{\"test\":true}Content-Length: 14\r\n\r\n{\"test\":false}";
        let transport = MemoryTransport::new(data.to_vec());
        let mut framer = MessageFramer::new(transport);

        let msg1 = framer.read_message().await.unwrap();
        assert_eq!(msg1, "{\"test\":true}");

        let msg2 = framer.read_message().await.unwrap();
        assert_eq!(msg2, "{\"test\":false}");
    }

    #[tokio::test]
    async fn test_missing_content_length() {
        let data = b"Content-Type: application/json\r\n\r\n{\"test\":true}";
        let transport = MemoryTransport::new(data.to_vec());
        let mut framer = MessageFramer::new(transport);

        let result = framer.read_message().await;
        assert!(result.is_err());
        if let Err(CopilotError::Protocol(msg)) = result {
            assert!(msg.contains("Missing Content-Length"));
        } else {
            panic!("Expected Protocol error");
        }
    }

    #[tokio::test]
    async fn test_case_insensitive_header() {
        let data = b"content-length: 13\r\n\r\n{\"test\":true}";
        let transport = MemoryTransport::new(data.to_vec());
        let mut framer = MessageFramer::new(transport);

        let message = framer.read_message().await.unwrap();
        assert_eq!(message, "{\"test\":true}");
    }

    #[tokio::test]
    async fn test_transport_closed() {
        let mut transport = MemoryTransport::new(Vec::new());
        transport.close().await.unwrap();

        let mut buf = [0u8; 10];
        let result = transport.read(&mut buf).await;
        assert!(matches!(result, Err(CopilotError::ConnectionClosed)));
    }
}
