//! Vocabulary and grammar reference entities (the seeded inventory).

use serde::{Deserialize, Serialize};

use crate::ids::{LexemeId, PatternId};
use crate::lang::{LanguageCode, PartOfSpeech};
use crate::unit::ExampleSentence;

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
    /// Latin-script romanization, for languages whose script the learner is
    /// still acquiring (e.g. Russian `кошка` → `koshka`). `None` for
    /// Latin-script languages, which don't need it.
    #[serde(default)]
    pub transliteration: Option<String>,
}

/// One grammar drill: a sentence with a blank to fill, its answer, and a
/// translation. The dedicated Grammar track tests a pattern with these.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GrammarDrill {
    /// The cue, with `___` marking the blank, e.g. `"Yo ___ español. (hablar)"`.
    pub prompt: String,
    /// The expected fill, e.g. `"hablo"`.
    pub answer: String,
    /// A native-language translation of the completed sentence.
    pub translation: String,
    /// An optional teaching note shown after answering — e.g. why an irregular
    /// answer doesn't follow the rule. Most drills leave this empty.
    #[serde(default)]
    pub note: Option<String>,
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
    /// Learner-facing title for the Grammar track, e.g. "The past tense (-ar)".
    #[serde(default)]
    pub title: String,
    /// CEFR level tag (e.g. "A1", "A2", "B1"), to group the Grammar track.
    #[serde(default)]
    pub level: String,
    pub example_template: String,
    /// Optional learner-facing explanation, shown as a tip inside a unit lesson
    /// (spec §2.2: surfaced only as opt-in support, never a gate).
    #[serde(default)]
    pub explanation: Option<String>,
    /// Patterns that must be learned first — gates this lesson in the track.
    #[serde(default)]
    pub prerequisites: Vec<PatternId>,
    /// Worked examples shown in the lesson (the rule in action).
    #[serde(default)]
    pub examples: Vec<ExampleSentence>,
    /// Teaching notes / nuances / common-mistake & irregular callouts, shown as
    /// bullets in the lesson before the drills.
    #[serde(default)]
    pub notes: Vec<String>,
    /// Drills that test this pattern in the Grammar track.
    #[serde(default)]
    pub drills: Vec<GrammarDrill>,
}
