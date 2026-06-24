//! Anthropic Messages API client for content generation.
//!
//! Uses structured outputs (`output_config.format` + a JSON schema) so the
//! model returns exactly `{text, known_words_used, new_words_introduced,
//! grammar_targeted}` — the contract in spec §5/§7 — rather than prose we'd
//! have to regex. Default model is `claude-opus-4-8`; override with the
//! `GLOSSA_MODEL` env var (e.g. a cheaper model for the high-volume loop).

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::json;

use glossa_core::{ContentKind, ContentRequest, GeneratedContent};

use crate::{ContentError, ContentGenerator, Result};

/// Default generation model. Override per-deployment with `GLOSSA_MODEL`.
pub const DEFAULT_MODEL: &str = "claude-opus-4-8";

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
/// Plenty for a short story / sentence set; keeps the request non-streaming.
const MAX_TOKENS: u32 = 1024;
/// Bound the known-vocab list we send, to control prompt token cost (spec §5).
const MAX_KNOWN_IN_PROMPT: usize = 60;

pub struct AnthropicContentGenerator {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl AnthropicContentGenerator {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }

    /// Build from the environment: `ANTHROPIC_API_KEY` (required) and
    /// `GLOSSA_MODEL` (optional, defaults to [`DEFAULT_MODEL`]).
    pub fn from_env() -> Result<Self> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| ContentError::Config("ANTHROPIC_API_KEY is not set".into()))?;
        let model = std::env::var("GLOSSA_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_string());
        Ok(Self::new(api_key, model))
    }

    /// Convenience: `Some(generator)` if a key is configured, else `None`.
    pub fn try_from_env() -> Option<Self> {
        Self::from_env().ok()
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

#[async_trait]
impl ContentGenerator for AnthropicContentGenerator {
    async fn generate(&self, request: &ContentRequest) -> Result<GeneratedContent> {
        let (system, user) = build_prompts(request);

        let body = json!({
            "model": self.model,
            "max_tokens": MAX_TOKENS,
            "system": system,
            "messages": [{ "role": "user", "content": user }],
            "output_config": { "format": output_schema() },
        });

        let resp = self
            .client
            .post(API_URL)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let raw = resp.text().await.unwrap_or_default();
            return Err(ContentError::Api {
                status: status.as_u16(),
                message: extract_api_error(&raw),
            });
        }

        let parsed: ApiResponse = resp.json().await?;
        if parsed.stop_reason.as_deref() == Some("refusal") {
            return Err(ContentError::Refusal);
        }

        let text = parsed
            .content
            .into_iter()
            .find_map(|b| if b.kind == "text" { b.text } else { None })
            .ok_or(ContentError::NoContent)?;

        let wire: Wire =
            serde_json::from_str(&text).map_err(|e| ContentError::Parse(e.to_string()))?;
        Ok(wire.into())
    }
}

/// JSON schema for the structured response (spec §5).
fn output_schema() -> serde_json::Value {
    json!({
        "type": "json_schema",
        "schema": {
            "type": "object",
            "properties": {
                "text": { "type": "string" },
                "known_words_used": { "type": "array", "items": { "type": "string" } },
                "new_words_introduced": { "type": "array", "items": { "type": "string" } },
                "grammar_targeted": { "type": "string" },
                "translation": { "type": "string" }
            },
            "required": [
                "text",
                "known_words_used",
                "new_words_introduced",
                "grammar_targeted",
                "translation"
            ],
            "additionalProperties": false
        }
    })
}

fn build_prompts(request: &ContentRequest) -> (String, String) {
    let lang = request.language.as_str();
    let kind = match request.kind {
        ContentKind::Sentence => "set of 2–4 simple, related sentences",
        ContentKind::Story => "very short story (3–6 sentences)",
    };
    let ratio_pct = (request.known_ratio * 100.0).round() as i32;

    let new_words: Vec<&str> = request
        .new_targets
        .iter()
        .map(|l| l.lemma.as_str())
        .collect();
    let known_words: Vec<&str> = request
        .known_vocab
        .iter()
        .take(MAX_KNOWN_IN_PROMPT)
        .map(|l| l.lemma.as_str())
        .collect();

    let mut system = format!(
        "You are an expert language tutor creating comprehensible-input content in language code '{lang}'.\n\
         Produce a {kind} the learner can mostly understand.\n\
         Rules:\n\
         - Build almost entirely from the KNOWN words provided; aim for about {ratio_pct}% known words.\n\
         - Introduce ONLY the listed NEW words, each at least once, in clear, guessable context.\n\
         - Keep it short, natural, and concrete.\n"
    );
    if let Some(g) = &request.grammar_target {
        system.push_str(&format!(
            "- Weave in the grammatical pattern \"{}\" naturally and more than once. Shape it like: \"{}\". \
             Do NOT name, label, or explain the grammar.\n",
            g.label, g.example_template
        ));
    }
    system.push_str(
        "Return the structured fields: the generated text; the known words you actually used \
         (drawn from the provided list); the new words you introduced; the grammar pattern \
         label you targeted (empty string if none); and a natural English translation of the \
         whole text so the learner can check their understanding.",
    );

    let new_list = if new_words.is_empty() {
        "(none — reinforce known words only)".to_string()
    } else {
        new_words.join(", ")
    };
    let known_list = if known_words.is_empty() {
        "(none yet — this is a brand-new learner)".to_string()
    } else {
        known_words.join(", ")
    };
    let user = format!(
        "KNOWN words ({}): {known_list}\nNEW words to introduce ({}): {new_list}\nGenerate the {kind} now.",
        known_words.len(),
        new_words.len(),
    );

    (system, user)
}

/// Pull `error.message` out of an Anthropic error body, else return a trimmed raw.
fn extract_api_error(raw: &str) -> String {
    serde_json::from_str::<serde_json::Value>(raw)
        .ok()
        .and_then(|v| {
            v.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| raw.chars().take(500).collect())
}

#[derive(Deserialize)]
struct ApiResponse {
    #[serde(default)]
    content: Vec<ApiBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct ApiBlock {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
}

/// The exact JSON shape the schema forces the model to return.
#[derive(Deserialize)]
struct Wire {
    text: String,
    known_words_used: Vec<String>,
    new_words_introduced: Vec<String>,
    grammar_targeted: String,
    translation: String,
}

impl From<Wire> for GeneratedContent {
    fn from(w: Wire) -> Self {
        let blank_to_none = |s: String| if s.trim().is_empty() { None } else { Some(s) };
        GeneratedContent {
            text: w.text,
            known_words_used: w.known_words_used,
            new_words_introduced: w.new_words_introduced,
            grammar_targeted: blank_to_none(w.grammar_targeted),
            translation: blank_to_none(w.translation),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glossa_core::{GrammarPattern, LanguageCode, LearnerId, Lexeme, LexemeId, PatternId, PartOfSpeech};

    fn lex(id: i64, lemma: &str) -> Lexeme {
        Lexeme {
            id: LexemeId(id),
            language: LanguageCode::spanish(),
            lemma: lemma.into(),
            pos: PartOfSpeech::Verb,
            frequency_rank: id as u32,
            gloss: None,
        }
    }

    #[test]
    fn prompt_embeds_new_words_and_grammar() {
        let req = ContentRequest {
            learner_id: LearnerId::new(),
            language: LanguageCode::spanish(),
            kind: ContentKind::Story,
            known_vocab: vec![lex(1, "yo"), lex(2, "comer")],
            new_targets: vec![lex(3, "pizza")],
            grammar_target: Some(GrammarPattern {
                id: PatternId(1),
                language: LanguageCode::spanish(),
                label: "preterite-regular-ar".into(),
                example_template: "Ayer ___é ...".into(),
            }),
            known_ratio: 0.95,
        };
        let (system, user) = build_prompts(&req);
        assert!(system.contains("preterite-regular-ar"));
        assert!(system.contains("95% known"));
        assert!(user.contains("pizza"));
        assert!(user.contains("yo"));
    }

    #[test]
    fn empty_grammar_string_maps_to_none() {
        let wire = Wire {
            text: "Hola.".into(),
            known_words_used: vec!["hola".into()],
            new_words_introduced: vec![],
            grammar_targeted: "  ".into(),
            translation: "Hello.".into(),
        };
        let gc: GeneratedContent = wire.into();
        assert!(gc.grammar_targeted.is_none());
        assert_eq!(gc.translation.as_deref(), Some("Hello."));
    }

    #[test]
    fn output_schema_is_well_formed() {
        let s = output_schema();
        assert_eq!(s["type"], "json_schema");
        assert_eq!(s["schema"]["additionalProperties"], false);
    }
}
