//! `glossa-storage` — persistence behind a single [`Store`] trait.
//!
//! V1 ships [`FileStore`]: an in-memory model persisted to a JSON file in the
//! app data directory. It needs zero setup and *keeps your learning progress*
//! across restarts, which is what matters for single-user validation.
//!
//! The production target is PostgreSQL (writes, source of truth) + DuckDB
//! (analytics reads) per spec §6 — see `schema.sql` in this crate. A `PgStore`
//! implementing the exact same [`Store`] trait drops in without touching any
//! other crate (spec §4.3, §9). That's the whole point of the trait.

mod memory;

pub use memory::FileStore;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use glossa_core::{
    GrammarPattern, GrammarState, LanguageCode, LearnerId, LearnerProfile, LearningEvent, Lexeme,
    LexemeId, LexemeState, PatternId, Unit,
};

/// A persisted story/sentence set. Mirrors the `stories` table (§6) plus the
/// lexeme ids it used, so that when the learner later marks it read we know
/// exactly which words to credit with an exposure.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StoredStory {
    pub id: Uuid,
    pub learner_id: LearnerId,
    pub language: LanguageCode,
    pub text: String,
    /// Every lexeme used in the text (known + new), resolved to ids.
    pub lexeme_ids: Vec<LexemeId>,
    /// The subset deliberately introduced as new.
    pub new_lexeme_ids: Vec<LexemeId>,
    pub grammar_pattern_id: Option<PatternId>,
    pub known_word_ratio: f32,
    pub generated_at: DateTime<Utc>,
}

/// Errors any [`Store`] implementation may raise.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("backend error: {0}")]
    Backend(String),
}

pub type Result<T> = std::result::Result<T, StorageError>;

/// The full persistence surface the service layer depends on.
///
/// Object-safe (used as `Arc<dyn Store>`), async so a networked backend fits
/// the same shape. Everything is learner-scoped — the hard part of
/// multi-tenancy, free to get right now (spec §9).
#[async_trait]
pub trait Store: Send + Sync {
    // --- learner ---------------------------------------------------------

    /// Fetch the single default learner, creating it on first run. V1 is
    /// single-user; this is the seam where auth/session lookup lands later.
    async fn get_or_create_default_learner(
        &self,
        target_language: LanguageCode,
        native_language: LanguageCode,
    ) -> Result<LearnerProfile>;

    async fn get_learner(&self, id: LearnerId) -> Result<Option<LearnerProfile>>;

    /// Persist changes to an existing learner (e.g. switching target language).
    async fn update_learner(&self, learner: &LearnerProfile) -> Result<()>;

    // --- reference inventory (seeded) ------------------------------------

    async fn lexemes(&self, language: &LanguageCode) -> Result<Vec<Lexeme>>;
    async fn grammar_patterns(&self, language: &LanguageCode) -> Result<Vec<GrammarPattern>>;
    async fn lexeme_count(&self, language: &LanguageCode) -> Result<usize>;
    async fn upsert_lexemes(&self, lexemes: &[Lexeme]) -> Result<()>;
    async fn upsert_grammar_patterns(&self, patterns: &[GrammarPattern]) -> Result<()>;

    // --- curriculum units (seeded reference content) ---------------------

    async fn units(&self, language: &LanguageCode) -> Result<Vec<Unit>>;
    async fn upsert_units(&self, units: &[Unit]) -> Result<()>;

    // --- per-learner mastery state ---------------------------------------

    async fn lexeme_states(&self, learner: LearnerId) -> Result<Vec<LexemeState>>;
    async fn grammar_states(&self, learner: LearnerId) -> Result<Vec<GrammarState>>;
    async fn upsert_lexeme_states(&self, learner: LearnerId, states: &[LexemeState]) -> Result<()>;
    async fn upsert_grammar_states(
        &self,
        learner: LearnerId,
        states: &[GrammarState],
    ) -> Result<()>;

    // --- append-only log + content ---------------------------------------

    async fn append_event(&self, learner: LearnerId, event: &LearningEvent) -> Result<()>;
    async fn save_story(&self, story: &StoredStory) -> Result<()>;
    async fn get_story(&self, id: Uuid) -> Result<Option<StoredStory>>;

    /// Timestamps of the learner's events, for streak/activity computations.
    async fn activity_dates(&self, learner: LearnerId) -> Result<Vec<DateTime<Utc>>>;
}
