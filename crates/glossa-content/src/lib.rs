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
