//! Pronunciation / orthography guides — the "how this language sounds" primer.
//!
//! Static reference content (no learner state): the letters, sounds, and
//! spelling rules that differ from English, each with an example word the
//! learner can hear. For Latin-script languages this is a short primer; the same
//! shape will host the alphabet/script course for non-Latin languages later.

use serde::{Deserialize, Serialize};

use crate::lang::LanguageCode;

/// One sound/letter and how to pronounce it, with an example.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoundEntry {
    /// A grouping heading, e.g. "Special letters", "Vowels".
    pub category: String,
    /// The letter or combination, e.g. "ä", "ll", "ch", a digit, or a letter.
    pub symbol: String,
    /// How it sounds, described for an English speaker (may be empty).
    pub sound: String,
    /// What to pronounce when the learner taps the symbol — the letter/word
    /// itself for speakable entries, `None` for spelling rules that aren't a
    /// single pronounceable sound (e.g. "silent final consonants").
    #[serde(default)]
    pub say: Option<String>,
    /// An example word in the target language (may be empty).
    pub example: String,
    /// The example's meaning (may be empty).
    pub example_gloss: String,
}

/// A language's pronunciation primer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PronunciationGuide {
    pub language: LanguageCode,
    pub intro: String,
    pub entries: Vec<SoundEntry>,
}
