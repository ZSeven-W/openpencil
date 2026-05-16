//! Message parsing and handling
//!
//! This module provides functionality for parsing JSON messages from the Claude Code CLI
//! into typed Rust structures.

pub mod parser;

pub use parser::parse_message;
