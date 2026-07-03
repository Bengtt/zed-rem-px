//! cssrem-lsp — a minimal Language Server that turns a typed CSS length into a
//! converted-unit completion.
//!
//! When the token immediately before the cursor looks like `<number>px` or
//! `<number>rem`, the server offers a single completion whose insert text is
//! the same length expressed in the other unit. `16px` -> `1rem`,
//! `1rem` -> `16px`. The conversion uses a configurable root font size
//! (default 16px), passed via the LSP `initializationOptions.rootFontSize`.

use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};

use regex::Regex;
use serde_json::{json, Value};

fn main() {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let mut docs: HashMap<String, String> = HashMap::new();
    let mut root_font_size: f64 = 16.0;

    // Match a number with a px/rem unit anchored to the end of the prefix.
    // Accepts 16, 16.5, .5 and a leading minus sign.
    let num_re = Regex::new(r"(?i)(-?(?:\d+\.?\d*|\.\d+))(px|rem)$").unwrap();

    while let Some(msg) = read_message(&mut reader) {
        let method = msg
            .get("method")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let id = msg.get("id").cloned();

        match method.as_str() {
            "initialize" => {
                if let Some(size) = msg
                    .pointer("/params/initializationOptions/rootFontSize")
                    .and_then(Value::as_f64)
                {
                    if size > 0.0 {
                        root_font_size = size;
                    }
                }
                respond(
                    &mut out,
                    id,
                    json!({
                        "capabilities": {
                            "textDocumentSync": 1, // full document sync
                            "completionProvider": {
                                "resolveProvider": false,
                                "triggerCharacters": ["x", "m"]
                            }
                        },
                        "serverInfo": { "name": "cssrem-lsp", "version": "0.1.0" }
                    }),
                );
            }
            "shutdown" => respond(&mut out, id, Value::Null),
            "exit" => break,

            "textDocument/didOpen" => {
                if let (Some(uri), Some(text)) = (
                    msg.pointer("/params/textDocument/uri").and_then(Value::as_str),
                    msg.pointer("/params/textDocument/text").and_then(Value::as_str),
                ) {
                    docs.insert(uri.to_string(), text.to_string());
                }
            }
            "textDocument/didChange" => {
                if let Some(uri) =
                    msg.pointer("/params/textDocument/uri").and_then(Value::as_str)
                {
                    // Full sync: the last content change carries the whole document.
                    if let Some(text) = msg
                        .pointer("/params/contentChanges/0/text")
                        .and_then(Value::as_str)
                    {
                        docs.insert(uri.to_string(), text.to_string());
                    }
                }
            }
            "textDocument/didClose" => {
                if let Some(uri) =
                    msg.pointer("/params/textDocument/uri").and_then(Value::as_str)
                {
                    docs.remove(uri);
                }
            }

            "textDocument/completion" => {
                let uri = msg
                    .pointer("/params/textDocument/uri")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let line = msg
                    .pointer("/params/position/line")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize;
                let character = msg
                    .pointer("/params/position/character")
                    .and_then(Value::as_u64)
                    .unwrap_or(0) as usize;

                let items =
                    completion_items(&docs, uri, line, character, root_font_size, &num_re);
                respond(
                    &mut out,
                    id,
                    json!({ "isIncomplete": false, "items": items }),
                );
            }

            // Any other request (has an id) gets an empty reply so the client
            // does not block. Notifications (no id) are ignored.
            _ => {
                if id.is_some() {
                    respond(&mut out, id, Value::Null);
                }
            }
        }
    }
}

fn completion_items(
    docs: &HashMap<String, String>,
    uri: &str,
    line: usize,
    character: usize,
    root: f64,
    re: &Regex,
) -> Vec<Value> {
    let text = match docs.get(uri) {
        Some(t) => t,
        None => return vec![],
    };
    let line_str = text.lines().nth(line).unwrap_or("");

    // LSP `character` is a UTF-16 offset; for the ASCII numbers/units we care
    // about, char indexing is equivalent and keeps this simple.
    let chars: Vec<char> = line_str.chars().collect();
    let cut = character.min(chars.len());
    let prefix: String = chars[..cut].iter().collect();

    let caps = match re.captures(&prefix) {
        Some(c) => c,
        None => return vec![],
    };
    let num_str = caps.get(1).unwrap().as_str();
    let unit = caps.get(2).unwrap().as_str().to_lowercase();
    let value: f64 = match num_str.parse() {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let matched = caps.get(0).unwrap().as_str();
    let match_len = matched.chars().count();
    let start_char = cut - match_len;

    let (converted, target_unit) = if unit == "px" {
        (value / root, "rem")
    } else {
        (value * root, "px")
    };
    let new_text = format!("{}{}", trim_num(converted), target_unit);

    let item = json!({
        "label": new_text,
        "kind": 12, // CompletionItemKind.Value
        "detail": format!(
            "cssrem: {}{} → {} (root {}px)",
            trim_num(value), unit, target_unit, trim_num(root)
        ),
        "sortText": "00000",
        "preselect": true,
        // Keep the item visible while the user's typed text is still the
        // original value (e.g. "16px").
        "filterText": matched,
        "textEdit": {
            "range": {
                "start": { "line": line, "character": start_char },
                "end":   { "line": line, "character": cut }
            },
            "newText": new_text
        }
    });

    vec![item]
}

/// Format a float without trailing zeros: 1.0 -> "1", 0.875 -> "0.875".
fn trim_num(n: f64) -> String {
    if n == 0.0 {
        return "0".to_string();
    }
    let s = format!("{:.6}", n);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    s.to_string()
}

/// Read one LSP message (Content-Length framed) from the reader.
fn read_message<R: BufRead>(reader: &mut R) -> Option<Value> {
    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None; // EOF
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break; // end of headers
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            content_length = rest.trim().parse().ok()?;
        }
    }
    let mut buf = vec![0u8; content_length];
    reader.read_exact(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

fn respond<W: Write>(out: &mut W, id: Option<Value>, result: Value) {
    let msg = json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "result": result
    });
    let body = serde_json::to_string(&msg).unwrap();
    let _ = write!(out, "Content-Length: {}\r\n\r\n{}", body.len(), body);
    let _ = out.flush();
}
