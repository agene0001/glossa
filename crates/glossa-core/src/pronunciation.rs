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
    /// The letter or combination, e.g. "ä", "ll", "ch".
    pub symbol: String,
    /// How it sounds, described for an English speaker.
    pub sound: String,
    /// An example word in the target language.
    pub example: String,
    /// The example's meaning.
    pub example_gloss: String,
}

/// A language's pronunciation primer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PronunciationGuide {
    pub language: LanguageCode,
    pub intro: String,
    pub entries: Vec<SoundEntry>,
}
