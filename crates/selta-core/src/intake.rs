//! Intake normalization (docs/03 §1): fence stripping and minimal JSON repair.
//! Every repair is a notice; intake never invents content.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::verdict::{Delta, Notice};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    #[default]
    Lenient,
    Strict,
}

#[derive(Debug, Clone)]
pub struct Intake {
    pub value: Value,
    pub notices: Vec<Notice>,
}

pub fn intake(text: &str, mode: Mode) -> Result<Intake, Box<Delta>> {
    let mut notices = Vec::new();
    let trimmed = text.trim();
    let mut body = trimmed.to_string();

    if trimmed.starts_with("```") {
        if mode == Mode::Strict {
            return Err(Box::new(Delta::structure(
                "$".to_string(),
                "markdown fence present in strict mode".to_string(),
            )));
        }
        body = strip_fence(trimmed);
        notices.push(notice("stripped markdown code fence"));
    }

    match serde_json::from_str(&body) {
        Ok(value) => Ok(Intake { value, notices }),
        Err(first_err) => {
            if mode == Mode::Lenient {
                let repaired = strip_trailing_commas(&body);
                if repaired != body {
                    if let Ok(value) = serde_json::from_str(&repaired) {
                        notices.push(notice("removed trailing comma(s)"));
                        return Ok(Intake { value, notices });
                    }
                }
            }
            Err(Box::new(Delta::structure(
                "$".to_string(),
                format!("unrecoverable JSON: {first_err}"),
            )))
        }
    }
}

fn notice(message: &str) -> Notice {
    Notice {
        path: "$".to_string(),
        message: message.to_string(),
        source: "intake".to_string(),
    }
}

fn strip_fence(text: &str) -> String {
    let mut lines: Vec<&str> = text.lines().collect();
    if !lines.is_empty() {
        lines.remove(0);
    }
    if lines.last().map(|l| l.trim() == "```").unwrap_or(false) {
        lines.pop();
    }
    lines.join("\n")
}

/// Drop commas that directly precede `}` or `]`, outside of strings.
fn strip_trailing_commas(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut in_string = false;
    let mut escape = false;
    let mut i = 0;

    while i < chars.len() {
        let c = chars[i];
        if in_string {
            out.push(c);
            if escape {
                escape = false;
            } else if c == '\\' {
                escape = true;
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        match c {
            '"' => {
                in_string = true;
                out.push(c);
                i += 1;
            }
            ',' => {
                let mut j = i + 1;
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                if j < chars.len() && (chars[j] == '}' || chars[j] == ']') {
                    i += 1;
                } else {
                    out.push(c);
                    i += 1;
                }
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}
