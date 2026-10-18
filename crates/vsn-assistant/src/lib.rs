//! Devify AI assistant (DVF-11): Claude through the user's own Anthropic API key.
//!
//! Raw HTTP to the Messages API (there is no official Rust SDK). The key is passed in by
//! the caller (read from the VSN vault) and only ever sent to api.anthropic.com.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;
use thiserror::Error;

pub const DEFAULT_MODEL: &str = "claude-opus-5-5";
const API_URL: &str = "https://api.anthropic.com/v1/messages";
/// Models the assistant may use; anything else is rejected before a request is sent.
pub const MODELS: &[&str] = &["claude-opus-5-5", "claude-sonnet-5-5", "claude-haiku-5-5"];

#[derive(Debug, Error)]
pub enum AiError {
    #[error("invalid input: {0}")]
    Invalid(String),
    #[error("Claude API error ({status}): {message}")]
    Api { status: u16, message: String },
    #[error("network error: {0}")]
    Transport(String),
    #[error("Claude declined this request ({0}). Try rephrasing it.")]
    Refused(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatMessage {
    /// "user" or "assistant"
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChatReply {
    pub text: String,
    pub model: String,
    pub stop_reason: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

pub fn validate_key(key: &str) -> Result<(), AiError> {
    if !key.starts_with("sk-ant-") || key.len() > 512 || key.chars().any(|c| c.is_whitespace()) {
        return Err(AiError::Invalid(
            "that does not look like an Anthropic API key (sk-ant-…)".into(),
        ));
    }
    Ok(())
}

/// Request body for one assistant turn. Pure, so it can be tested without the network.
pub fn build_request(
    model: &str,
    system: &str,
    messages: &[ChatMessage],
    effort: &str,
) -> Result<Value, AiError> {
    if !MODELS.contains(&model) {
        return Err(AiError::Invalid(format!("unsupported model {model}")));
    }
    if !matches!(effort, "low" | "medium" | "high" | "xhigh" | "max") {
        return Err(AiError::Invalid(
            "effort must be low, medium, high, xhigh or max".into(),
        ));
    }
    if messages.is_empty() || messages.len() > 200 {
        return Err(AiError::Invalid("send between 1 and 200 messages".into()));
    }
    let mut total = system.len();
    for (i, m) in messages.iter().enumerate() {
        let expected = if i % 2 == 0 { "user" } else { "assistant" };
        if m.role != expected {
            return Err(AiError::Invalid(
                "messages must alternate user/assistant, starting with user".into(),
            ));
        }
        total += m.content.len();
    }
    if messages.last().map(|m| m.role.as_str()) != Some("user") {
        return Err(AiError::Invalid(
            "the last message must be from the user".into(),
        ));
    }
    if total > 2_000_000 {
        return Err(AiError::Invalid(
            "conversation is too long; start a new chat".into(),
        ));
    }
    let mut body = json!({
        "model": model,
        "max_tokens": 16000,
        "thinking": {"type": "adaptive"},
        "output_config": {"effort": effort},
        "messages": messages,
    });
    if !system.is_empty() {
        body["system"] = json!(system);
    }
    // If a safety classifier declines, retry server-side on a fallback chosen by category.
    if model != "claude-haiku-5-5" {
        body["fallbacks"] = json!("default");
    }
    Ok(body)
}

/// Pull the reply text out of a Messages API response, honouring refusals.
pub fn parse_reply(v: &Value) -> Result<ChatReply, AiError> {
    let stop = v["stop_reason"].as_str().unwrap_or_default().to_string();
    if stop == "refusal" {
        let category = v["stop_details"]["category"]
            .as_str()
            .unwrap_or("policy")
            .to_string();
        return Err(AiError::Refused(category));
    }
    let text: String = v["content"]
        .as_array()
        .map(|blocks| {
            blocks
                .iter()
                .filter(|b| b["type"] == "text")
                .filter_map(|b| b["text"].as_str())
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    Ok(ChatReply {
        text,
        model: v["model"].as_str().unwrap_or_default().to_string(),
        stop_reason: stop,
        input_tokens: v["usage"]["input_tokens"].as_u64().unwrap_or(0),
        output_tokens: v["usage"]["output_tokens"].as_u64().unwrap_or(0),
    })
}

pub fn chat(
    api_key: &str,
    model: &str,
    system: &str,
    messages: &[ChatMessage],
    effort: &str,
) -> Result<ChatReply, AiError> {
    validate_key(api_key)?;
    let body = build_request(model, system, messages, effort)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(600))
        .build()
        .map_err(|e| AiError::Transport(e.to_string()))?;
    let mut req = client
        .post(API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body);
    if body.get("fallbacks").is_some() {
        req = req.header("anthropic-beta", "server-side-fallback-2026-07-01");
    }
    let resp = req.send().map_err(|e| AiError::Transport(e.to_string()))?;
    let status = resp.status().as_u16();
    let v: Value = resp.json().map_err(|e| AiError::Transport(e.to_string()))?;
    if status >= 400 {
        let message = v["error"]["message"]
            .as_str()
            .unwrap_or("request failed")
            .to_string();
        return Err(AiError::Api { status, message });
    }
    parse_reply(&v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn msg(role: &str, content: &str) -> ChatMessage {
        ChatMessage {
            role: role.into(),
            content: content.into(),
        }
    }

    #[test]
    fn builds_adaptive_request_with_fallbacks() {
        let b = build_request(
            DEFAULT_MODEL,
            "You help with a Laravel project.",
            &[msg("user", "hi")],
            "medium",
        )
        .unwrap();
        assert_eq!(b["model"], "claude-opus-5-5");
        assert_eq!(b["thinking"]["type"], "adaptive");
        assert_eq!(b["output_config"]["effort"], "medium");
        assert_eq!(b["fallbacks"], "default");
        assert!(b.get("temperature").is_none());
    }

    #[test]
    fn rejects_bad_input() {
        assert!(build_request("gpt-4", "", &[msg("user", "x")], "low").is_err());
        assert!(build_request(DEFAULT_MODEL, "", &[msg("assistant", "x")], "low").is_err());
        assert!(build_request(
            DEFAULT_MODEL,
            "",
            &[msg("user", "a"), msg("assistant", "b")],
            "low"
        )
        .is_err());
        assert!(validate_key("hello").is_err());
        assert!(validate_key("sk-ant-api03-abc").is_ok());
    }

    #[test]
    fn parses_text_and_refusal() {
        let ok = json!({"model":"claude-opus-5-5","stop_reason":"end_turn","content":[{"type":"thinking","thinking":""},{"type":"text","text":"Hello"}],"usage":{"input_tokens":5,"output_tokens":2}});
        let r = parse_reply(&ok).unwrap();
        assert_eq!((r.text.as_str(), r.output_tokens), ("Hello", 2));
        let refused = json!({"stop_reason":"refusal","stop_details":{"type":"refusal","category":"cyber"},"content":[]});
        assert!(matches!(parse_reply(&refused), Err(AiError::Refused(c)) if c == "cyber"));
    }
}
