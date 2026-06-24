//! Curriculum units — the visible learning roadmap.
//!
//! A unit is an ordered step (like a Duolingo skill): a themed set of target
//! vocabulary + an optional grammar focus, plus a few **authored** example
//! sentences that teach it coherently (so lessons read well even without the
//! LLM). The knowledge graph still tracks mastery per word; the unit just gives
//! that progress a shape and a sense of place.

use serde::{Deserialize, Serialize};

use crate::ids::{LexemeId, PatternId, UnitId};
use crate::lang::LanguageCode;

/// One authored example: target-language text plus its native translation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExampleSentence {
    pub text: String,
    pub translation: String,
}

/// A roadmap step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Unit {
    pub id: UnitId,
    pub language: LanguageCode,
    pub title: String,
    pub description: String,
    /// The words this unit teaches; progress is measured over these.
    pub target_lexemes: Vec<LexemeId>,
    /// Optional grammar pattern this unit focuses on.
    pub target_pattern: Option<PatternId>,
    /// Hand-written example sentences that introduce the unit's vocabulary.
    pub examples: Vec<ExampleSentence>,
}
