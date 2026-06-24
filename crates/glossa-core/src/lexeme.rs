//! Vocabulary and grammar reference entities (the seeded inventory).

use serde::{Deserialize, Serialize};

use crate::ids::{LexemeId, PatternId};
use crate::lang::{LanguageCode, PartOfSpeech};

/// A dictionary headword in the target language.
///
/// `frequency_rank` is 1-based: rank 1 is the single most common word in the
/// language. The graph uses it to decide which unknown word is most worth
/// teaching next (spec §5, `glossa-graph`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lexeme {
    pub id: LexemeId,
    pub language: LanguageCode,
    pub lemma: String,
    pub pos: PartOfSpeech,
    pub frequency_rank: u32,
    /// Short meaning in the learner's native language (e.g. "to eat"). Without
    /// this, content isn't comprehensible — you can't learn a word you can't
    /// understand. `None` if the seed list didn't provide one.
    pub gloss: Option<String>,
}

/// A grammar pattern tracked as a first-class node, exactly like vocabulary
/// (spec §2.2). `example_template` seeds the LLM prompt when this pattern is
/// the deliberate target for a piece of content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrammarPattern {
    pub id: PatternId,
    pub language: LanguageCode,
    /// Machine-ish label, e.g. `"preterite-regular-ar"`. Never shown as a
    /// mandatory rule gate — only as opt-in support once it has recurred.
    pub label: String,
    pub example_template: String,
}
