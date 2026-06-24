//! The append-only learning event log (spec §5 `glossa-core`, §6 `events`).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ids::LexemeId;

/// Something the learner did that carries an acquisition signal.
///
/// Events are the *only* way mastery changes: the graph folds them over current
/// state. Persisted append-only so the mastery model can be recomputed or
/// re-tuned later without losing history (spec §9).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LearningEvent {
    /// The learner read a generated story/sentence set.
    StoryRead {
        story_id: Uuid,
        words_seen: Vec<LexemeId>,
        /// Whether the learner reported understanding it (the "I got it" signal).
        understood: bool,
    },
    /// One turn of AI/human conversation (Phase 2+).
    ChatTurn {
        conversation_id: Uuid,
        new_lexemes: Vec<LexemeId>,
        corrected: bool,
    },
    /// An explicit exercise answer (correct/incorrect).
    ExerciseAnswered { lexeme_id: LexemeId, correct: bool },
}
