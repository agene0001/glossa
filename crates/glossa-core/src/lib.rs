//! `glossa-core` — shared domain types with no I/O.
//!
//! Everything the rest of the workspace agrees on lives here: identifiers,
//! the vocabulary/grammar model, mastery state, learning events, and the
//! request/response shapes that flow between the knowledge graph, the content
//! generator, and the frontend.
//!
//! This crate deliberately has no async runtime, no database, and no HTTP — so
//! it stays trivially testable and reusable from a future `glossa-api`.

mod content;
mod event;
mod ids;
mod lang;
mod learner;
mod lexeme;
mod mastery;
mod unit;

pub use content::{
    ContentKind, ContentRequest, ContentResponse, GeneratedContent, Token, TokenStatus, WordInfo,
};
pub use event::LearningEvent;
pub use ids::{LearnerId, LexemeId, PatternId, UnitId};
pub use lang::{LanguageCode, PartOfSpeech};
pub use learner::LearnerProfile;
pub use lexeme::{GrammarPattern, Lexeme};
pub use mastery::{GrammarState, LexemeState, MasteryState};
pub use unit::{ExampleSentence, ReadingPassage, Unit};

/// Crate-wide error type. Kept intentionally small; domain logic rarely fails,
/// and I/O errors belong to the crates that actually do I/O.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("invalid language code: {0:?}")]
    InvalidLanguageCode(String),
    #[error("invalid mastery confidence: {0} (must be within 0.0..=1.0)")]
    InvalidConfidence(f32),
}
