//! `glossa-content` — generate comprehensible-input content from graph state.
//!
//! The [`ContentGenerator`] trait is the seam. Two implementations ship:
//! - [`AnthropicContentGenerator`] — calls the Anthropic Messages API and
//!   requests **structured JSON output** so new vs. reinforced words are logged
//!   deterministically, never parsed from prose (spec §5, §7).
//! - [`MockContentGenerator`] — deterministic, offline, no API key. Lets the
//!   whole app run end-to-end before you wire up a key, and backs unit tests.
//!
//! Rust has no official Anthropic SDK, so the API client is thin `reqwest` over
//! `POST /v1/messages` — the documented raw-HTTP path.

mod anthropic;
mod mock;

pub use anthropic::{AnthropicContentGenerator, DEFAULT_MODEL};
pub use mock::MockContentGenerator;

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use glossa_core::{ContentRequest, GeneratedContent, LanguageCode, PartOfSpeech};

/// A request to turn the learner's English input into target-language vocabulary.
/// `count == 0` means "translate exactly the word(s) typed"; `count > 0` means
/// "suggest that many words on this topic".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VocabRequest {
    pub language: LanguageCode,
    pub native_language: LanguageCode,
    pub query: String,
    pub count: usize,
}

/// One suggested vocabulary entry from the generator.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SuggestedWord {
    /// The word in the target language (nouns include their article: "der Tisch").
    pub term: String,
    /// A short meaning in the learner's native language.
    pub gloss: String,
    pub pos: Option<PartOfSpeech>,
}

/// One row of a bundled offline dictionary file.
#[derive(Debug, Clone, Deserialize)]
struct DictEntry {
    en: String,
    term: String,
    pos: String,
}

/// An offline English → target-language dictionary, loaded from bundled JSON.
/// Backs the "add words by English" flow with no API key — translation only;
/// topic generation still needs the LLM.
#[derive(Default)]
pub struct OfflineDictionary {
    by_lang: HashMap<String, HashMap<String, Vec<SuggestedWord>>>,
}

impl OfflineDictionary {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load one language's dictionary from its bundled JSON.
    pub fn load(&mut self, lang: &str, json: &str) -> Result<()> {
        let entries: Vec<DictEntry> =
            serde_json::from_str(json).map_err(|e| ContentError::Parse(e.to_string()))?;
        let map = self.by_lang.entry(lang.to_string()).or_default();
        for e in entries {
            let word = SuggestedWord {
                term: e.term,
                gloss: e.en.clone(),
                pos: pos_from_str(&e.pos),
            };
            map.entry(normalize_en(&e.en)).or_default().push(word);
        }
        Ok(())
    }

    /// Translate the English input (a word, or a comma-separated list) into any
    /// matching target-language words. Empty when nothing matches.
    pub fn lookup(&self, lang: &str, query: &str) -> Vec<SuggestedWord> {
        let Some(map) = self.by_lang.get(lang) else {
            return Vec::new();
        };
        let mut out: Vec<SuggestedWord> = Vec::new();
        for part in query.split(',') {
            if let Some(hits) = map.get(&normalize_en(part)) {
                for h in hits {
                    if !out.contains(h) {
                        out.push(h.clone());
                    }
                }
            }
        }
        out
    }
}

/// Normalize an English key for matching: lowercase, trim, drop a leading "to ".
fn normalize_en(s: &str) -> String {
    let s = s.trim().to_lowercase();
    s.strip_prefix("to ").unwrap_or(&s).trim().to_string()
}

/// Parse a part-of-speech string (snake_case) into [`PartOfSpeech`].
pub fn pos_from_str(s: &str) -> Option<PartOfSpeech> {
    Some(match s.trim().to_lowercase().as_str() {
        "noun" => PartOfSpeech::Noun,
        "verb" => PartOfSpeech::Verb,
        "adjective" => PartOfSpeech::Adjective,
        "adverb" => PartOfSpeech::Adverb,
        "pronoun" => PartOfSpeech::Pronoun,
        "preposition" => PartOfSpeech::Preposition,
        "conjunction" => PartOfSpeech::Conjunction,
        "determiner" => PartOfSpeech::Determiner,
        "numeral" => PartOfSpeech::Numeral,
        "interjection" => PartOfSpeech::Interjection,
        "other" => PartOfSpeech::Other,
        _ => return None,
    })
}

/// Errors a generator can raise.
#[derive(Debug, thiserror::Error)]
pub enum ContentError {
    #[error("http transport error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("anthropic api error ({status}): {message}")]
    Api { status: u16, message: String },
    #[error("could not parse model output as the requested schema: {0}")]
    Parse(String),
    #[error("the model declined to generate this content (refusal)")]
    Refusal,
    #[error("the model returned no text content")]
    NoContent,
    #[error("missing configuration: {0}")]
    Config(String),
}

pub type Result<T> = std::result::Result<T, ContentError>;

/// Turns a graph-chosen [`ContentRequest`] into concrete text plus the
/// structured record of which words it used and introduced.
#[async_trait]
pub trait ContentGenerator: Send + Sync {
    async fn generate(&self, request: &ContentRequest) -> Result<GeneratedContent>;

    /// Translate / suggest target-language vocabulary from the learner's English
    /// input, for the "add words by English" flow. Offline generators that can't
    /// translate should return [`ContentError::Config`].
    async fn suggest_vocab(&self, request: &VocabRequest) -> Result<Vec<SuggestedWord>>;
}
