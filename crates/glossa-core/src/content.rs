//! Content generation request/response shapes.
//!
//! Three layers, on purpose:
//! - [`ContentRequest`]  — what the graph decided to teach (graph → generator).
//! - [`GeneratedContent`] — raw structured output from the generator, keyed by
//!   lemma strings, mirroring the LLM JSON in spec §5 (generator → service).
//! - [`ContentResponse`] — enriched, tokenized, id-resolved payload the
//!   frontend renders (service → UI).

use serde::{Deserialize, Serialize};

use crate::ids::{LearnerId, LexemeId};
use crate::lang::{LanguageCode, PartOfSpeech};
use crate::lexeme::{GrammarPattern, Lexeme};

/// Whether we want a single sentence set or a short story.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContentKind {
    Sentence,
    Story,
}

/// The graph's decision about the next most useful piece of content
/// (spec §4.3, §5 `glossa-graph::next_best_content`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentRequest {
    pub learner_id: LearnerId,
    pub language: LanguageCode,
    pub kind: ContentKind,
    /// Known/partial words to build from (the comprehensible base).
    pub known_vocab: Vec<Lexeme>,
    /// The 1–3 new words to introduce through context.
    pub new_targets: Vec<Lexeme>,
    /// Optional grammar pattern to target without naming it (spec §2.2).
    pub grammar_target: Option<GrammarPattern>,
    /// Target ratio of known words, default 0.95 (spec §2.1).
    pub known_ratio: f32,
}

/// Raw generator output. Field names match the JSON contract in spec §5 so the
/// app never regex-parses prose to learn what vocabulary was used (spec §7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedContent {
    pub text: String,
    pub known_words_used: Vec<String>,
    pub new_words_introduced: Vec<String>,
    pub grammar_targeted: Option<String>,
}

/// How a single token relates to the learner's knowledge — drives highlighting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenStatus {
    Known,
    Partial,
    /// A deliberately-introduced new word.
    New,
    /// A word not in the inventory / not classified.
    Unknown,
}

/// One renderable token. Non-words (spaces, punctuation) carry `is_word=false`
/// and no status, so the frontend can render the text verbatim while colouring
/// only the words.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    pub text: String,
    pub is_word: bool,
    pub status: Option<TokenStatus>,
    pub lexeme_id: Option<LexemeId>,
}

/// A new word surfaced to the learner alongside the text (the "5%").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WordInfo {
    pub lemma: String,
    pub lexeme_id: Option<LexemeId>,
    pub pos: Option<PartOfSpeech>,
}

/// The enriched, frontend-facing result of generating a piece of content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContentResponse {
    pub story_id: uuid::Uuid,
    pub language: LanguageCode,
    pub kind: ContentKind,
    pub text: String,
    /// Tokenized text, each word tagged with the learner's status for it.
    pub tokens: Vec<Token>,
    /// The new words introduced in this piece (for an inline glossary).
    pub new_words: Vec<WordInfo>,
    pub grammar_targeted: Option<String>,
    /// Actual measured ratio of known tokens (sanity-check against the target).
    pub known_ratio: f32,
}
