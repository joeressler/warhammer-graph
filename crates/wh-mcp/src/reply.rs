//! Turns library results and errors into tool replies.
//!
//! A successful result is compact JSON in a text block, which every MCP client
//! reads. A library error is a tool-level error, flagged `isError`, so the agent
//! sees it and can retry. A protocol error would reach the agent as an opaque
//! failure, so nothing here uses one.

use rmcp::model::{CallToolResult, ContentBlock};
use serde::Serialize;
use serde_json::{json, Value};
use wh_ask::AskError;

/// Said with every ambiguity error. Models tend to take the first candidate and
/// answer as if it were the only one, so this says not to.
pub const AMBIGUOUS_INSTRUCTION: &str = "Several different things share this name. Do not pick one silently. \
Tell the user that more than one exists, and either ask which they mean, or call again once with each id and \
cover every one in your answer.";

/// A successful reply carrying `value` as JSON.
pub fn ok<T: Serialize>(value: &T) -> CallToolResult {
    match serde_json::to_string(value) {
        Ok(text) => CallToolResult::success(vec![ContentBlock::text(text)]),
        Err(err) => fail(json!({ "error": format!("could not encode the result: {err}") })),
    }
}

/// A reply from a library call: the result, or the error as the agent should see it.
pub fn from<T: Serialize>(result: Result<T, AskError>) -> CallToolResult {
    match result {
        Ok(value) => ok(&value),
        Err(error) => error_reply(&error),
    }
}

/// A library error, with the ids or names the agent needs to retry.
pub fn error_reply(error: &AskError) -> CallToolResult {
    let body = match error {
        AskError::Ambiguous { candidates, .. } => json!({
            "error": error.to_string(),
            "candidates": candidates,
            "instruction": AMBIGUOUS_INSTRUCTION,
        }),
        AskError::NotFound { suggestions, .. } => {
            json!({ "error": error.to_string(), "suggestions": suggestions })
        }
        _ => json!({ "error": error.to_string() }),
    };
    fail(body)
}

fn fail(body: Value) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(body.to_string())])
}
