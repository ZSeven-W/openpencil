//! `op` — the OpenPencil command-line tool.
//!
//! A thin client over the HTTP MCP transport (`op-host-desktop
//! --mcp-http <port> <file>`): every `op` invocation maps onto one
//! MCP `tools/call` and prints the JSON-RPC reply. Because the editor
//! already exposes its full editing surface as MCP tools, the generic
//! `op <tool> key=value …` form drives all of them — insert / update /
//! delete / move / design_* / variables / pages / export, etc.
//!
//! Usage:
//!   op [--port N] tools                  list every tool + schema
//!   op [--port N] <tool> [key=value …]   call one tool
//!   op help                              this message
//!
//! `--port` defaults to 8765; start the server with
//! `op-host-desktop --mcp-http 8765 <file>.op`.
//!
//! Dependency-free on purpose: the HTTP/1.1 request and the JSON-RPC
//! body are hand-rolled, mirroring `op-mcp`'s hand-rolled wire layer.

use std::io::{Read, Write};

/// Default HTTP MCP port — pair with `op-host-desktop --mcp-http 8765`.
const DEFAULT_PORT: u16 = 8765;

const USAGE: &str = "\
op — OpenPencil CLI (drives the editor over the HTTP MCP transport)

USAGE:
  op [--port N] tools                 list every MCP tool + input schema
  op [--port N] <tool> [key=value …]  call one MCP tool with string args
  op help                             show this message

EXAMPLES:
  op tools
  op insert_node kind=rect name=Box x=10 y=20 width=100 height=60
  op set_node_fill_hex node_id=n3 hex=#ff0000
  op --port 9001 get_document_info

The server must be running:
  op-host-desktop --mcp-http 8765 path/to/file.op";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(out) => {
            println!("{out}");
        }
        Err(e) => {
            eprintln!("op: {e}");
            std::process::exit(1);
        }
    }
}

/// Parse `args`, perform the request, return the text to print.
/// Pure except for [`post`] — the parse/build steps are unit-tested.
fn run(args: &[String]) -> Result<String, String> {
    let Parsed { port, command } = parse_args(args)?;
    match command {
        Command::Help => Ok(USAGE.to_string()),
        Command::ToolsList => post(port, &tools_list_body()),
        Command::ToolCall { tool, args } => {
            post(port, &tool_call_body(&tool, &args_to_json(&args)))
        }
    }
}

/// Outcome of argument parsing.
struct Parsed {
    port: u16,
    command: Command,
}

enum Command {
    Help,
    ToolsList,
    ToolCall {
        tool: String,
        args: Vec<(String, String)>,
    },
}

/// Parse `[--port N] <command> …` into a [`Parsed`]. `--port` may
/// appear anywhere before the command.
fn parse_args(args: &[String]) -> Result<Parsed, String> {
    let mut port = DEFAULT_PORT;
    let mut rest: Vec<&String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--port" => {
                let raw = args
                    .get(i + 1)
                    .ok_or("--port needs a value (e.g. --port 8765)")?;
                port = raw
                    .parse::<u16>()
                    .map_err(|_| format!("--port must be a u16, got {raw:?}"))?;
                i += 2;
            }
            _ => {
                rest.push(&args[i]);
                i += 1;
            }
        }
    }
    let Some(cmd) = rest.first() else {
        return Err(format!("missing command\n\n{USAGE}"));
    };
    let command = match cmd.as_str() {
        "help" | "--help" | "-h" => Command::Help,
        "tools" => Command::ToolsList,
        tool => {
            let mut pairs = Vec::new();
            for kv in &rest[1..] {
                let (k, v) = kv
                    .split_once('=')
                    .ok_or_else(|| format!("argument must be key=value, got {kv:?}"))?;
                if k.is_empty() {
                    return Err(format!("argument has an empty key: {kv:?}"));
                }
                pairs.push((k.to_string(), v.to_string()));
            }
            Command::ToolCall {
                tool: tool.to_string(),
                args: pairs,
            }
        }
    };
    Ok(Parsed { port, command })
}

/// JSON-RPC body for `tools/list`.
fn tools_list_body() -> String {
    r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#.to_string()
}

/// JSON-RPC body for a `tools/call` of `tool` with the already-built
/// `arguments` object JSON.
fn tool_call_body(tool: &str, args_json: &str) -> String {
    format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{{"name":"{}","arguments":{}}}}}"#,
        json_escape(tool),
        args_json
    )
}

/// Build a JSON object from `key=value` pairs. MCP tool arguments are
/// all string-typed, so every value is emitted as a JSON string.
fn args_to_json(pairs: &[(String, String)]) -> String {
    let mut out = String::from("{");
    for (i, (k, v)) in pairs.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(&json_escape(k));
        out.push_str("\":\"");
        out.push_str(&json_escape(v));
        out.push('"');
    }
    out.push('}');
    out
}

/// Escape a string for inclusion in a JSON string literal.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// POST `body` to the HTTP MCP server on `127.0.0.1:port` and return
/// the response body (the JSON-RPC reply).
fn post(port: u16, body: &str) -> Result<String, String> {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).map_err(|e| {
        format!(
            "cannot reach the editor on 127.0.0.1:{port}: {e}\n\
             start it with: op-host-desktop --mcp-http {port} <file>.op"
        )
    })?;
    let request = format!(
        "POST / HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("http write: {e}"))?;
    stream.flush().ok();
    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|e| format!("http read: {e}"))?;
    // Strip the HTTP head — everything past the blank line is the
    // JSON-RPC reply.
    Ok(match response.split_once("\r\n\r\n") {
        Some((_, body)) => body.trim().to_string(),
        None => response.trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_escape_handles_quotes_backslash_control() {
        assert_eq!(json_escape(r#"a"b\c"#), r#"a\"b\\c"#);
        assert_eq!(json_escape("line\nbreak"), "line\\nbreak");
        assert_eq!(json_escape("tab\there"), "tab\\there");
        assert_eq!(json_escape("\u{0001}"), "\\u0001");
    }

    #[test]
    fn args_to_json_builds_string_valued_object() {
        assert_eq!(args_to_json(&[]), "{}");
        let pairs = vec![
            ("kind".to_string(), "rect".to_string()),
            ("x".to_string(), "10".to_string()),
        ];
        assert_eq!(args_to_json(&pairs), r#"{"kind":"rect","x":"10"}"#);
    }

    #[test]
    fn args_to_json_escapes_values() {
        let pairs = vec![("name".to_string(), r#"a"b"#.to_string())];
        assert_eq!(args_to_json(&pairs), r#"{"name":"a\"b"}"#);
    }

    #[test]
    fn tool_call_body_wraps_name_and_arguments() {
        let body = tool_call_body("insert_node", r#"{"kind":"rect"}"#);
        assert!(body.contains(r#""method":"tools/call""#));
        assert!(body.contains(r#""name":"insert_node""#));
        assert!(body.contains(r#""arguments":{"kind":"rect"}"#));
    }

    #[test]
    fn tools_list_body_is_a_tools_list_request() {
        assert!(tools_list_body().contains(r#""method":"tools/list""#));
    }

    #[test]
    fn parse_args_defaults_port_and_reads_tool_call() {
        let args = vec![
            "insert_node".to_string(),
            "kind=rect".to_string(),
            "x=10".to_string(),
        ];
        let p = parse_args(&args).expect("parse");
        assert_eq!(p.port, DEFAULT_PORT);
        match p.command {
            Command::ToolCall { tool, args } => {
                assert_eq!(tool, "insert_node");
                assert_eq!(args.len(), 2);
                assert_eq!(args[0], ("kind".to_string(), "rect".to_string()));
            }
            _ => panic!("expected ToolCall"),
        }
    }

    #[test]
    fn parse_args_reads_explicit_port_anywhere() {
        let args = vec![
            "--port".to_string(),
            "9001".to_string(),
            "tools".to_string(),
        ];
        let p = parse_args(&args).expect("parse");
        assert_eq!(p.port, 9001);
        assert!(matches!(p.command, Command::ToolsList));
    }

    #[test]
    fn parse_args_rejects_non_kv_argument() {
        let args = vec!["insert_node".to_string(), "bogus".to_string()];
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn parse_args_rejects_bad_port() {
        let args = vec![
            "--port".to_string(),
            "notnum".to_string(),
            "tools".to_string(),
        ];
        assert!(parse_args(&args).is_err());
        let missing = vec!["--port".to_string()];
        assert!(parse_args(&missing).is_err());
    }

    #[test]
    fn parse_args_help_and_empty() {
        assert!(matches!(
            parse_args(&["help".to_string()]).unwrap().command,
            Command::Help
        ));
        assert!(parse_args(&[]).is_err());
    }
}
