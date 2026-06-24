//! Mastery state — the heart of the knowledge graph (spec §2.6, §6).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::ids::{LexemeId, PatternId};

/// How well the learner knows one item (a word or a grammar pattern).
///
/// Serialized internally-tagged so the JSON the frontend sees is flat and
/// obvious: `{"status":"partial","confidence":0.6}` / `{"status":"known"}`.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum MasteryState {
    /// Never meaningfully encountered.
    Unknown,
    /// Seen in context but not yet reliable. `confidence` is within `0.0..=1.0`.
    Partial { confidence: f32 },
    /// Reliably understood.
    Known,
}

impl MasteryState {
    /// A scalar in `0.0..=1.0` summarizing this state, for ranking/decay math.
    pub fn confidence(&self) -> f32 {
        match self {
            MasteryState::Unknown => 0.0,
            MasteryState::Partial { confidence } => *confidence,
            MasteryState::Known => 1.0,
        }
    }

    pub fn is_known(&self) -> bool {
        matches!(self, MasteryState::Known)
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, MasteryState::Unknown)
    }

    /// Short human label for the Review view.
    pub fn label(&self) -> &'static str {
        match self {
            MasteryState::Unknown => "unknown",
            MasteryState::Partial { .. } => "partial",
            MasteryState::Known => "known",
        }
    }
}

impl Default for MasteryState {
    fn default() -> Self {
        MasteryState::Unknown
    }
}

/// Per-learner state for a single lexeme. Mirrors `learner_lexeme_state` (§6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LexemeState {
    pub lexeme_id: LexemeId,
    pub mastery: MasteryState,
    /// Count of meaningful contextual exposures (drives transitions — §11.3).
    pub exposure_count: u32,
    pub last_seen_at: Option<DateTime<Utc>>,
}

impl LexemeState {
    /// A fresh, never-seen state for a lexeme.
    pub fn unseen(lexeme_id: LexemeId) -> Self {
        Self {
            lexeme_id,
            mastery: MasteryState::Unknown,
            exposure_count: 0,
            last_seen_at: None,
        }
    }
}

/// Per-learner state for a grammar pattern. Mirrors `learner_grammar_state` (§6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrammarState {
    pub pattern_id: PatternId,
    pub mastery: MasteryState,
    pub exposure_count: u32,
    pub last_seen_at: Option<DateTime<Utc>>,
}

impl GrammarState {
    pub fn unseen(pattern_id: PatternId) -> Self {
        Self {
            pattern_id,
            mastery: MasteryState::Unknown,
            exposure_count: 0,
            last_seen_at: None,
        }
    }
}
