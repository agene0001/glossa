//! Language codes and parts of speech.

use std::fmt;

use serde::{Deserialize, Serialize};

/// An ISO 639-1 language code (e.g. `"es"`, `"en"`).
///
/// A newtype rather than an enum so adding a language is data, not a code
/// change — V1 is single-language, but the schema is multi-language ready.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LanguageCode(String);

impl LanguageCode {
    pub fn new(code: impl Into<String>) -> Self {
        Self(code.into().to_lowercase())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn spanish() -> Self {
        Self::new("es")
    }

    pub fn english() -> Self {
        Self::new("en")
    }
}

impl fmt::Display for LanguageCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for LanguageCode {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Coarse part-of-speech tag. Enough to weight selection and prompt the LLM;
/// not a full morphological analysis (that's a post-V1 concern — spec §11.5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PartOfSpeech {
    Noun,
    Verb,
    Adjective,
    Adverb,
    Pronoun,
    Preposition,
    Conjunction,
    Determiner,
    Numeral,
    Interjection,
    Other,
}

impl fmt::Display for PartOfSpeech {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            PartOfSpeech::Noun => "noun",
            PartOfSpeech::Verb => "verb",
            PartOfSpeech::Adjective => "adjective",
            PartOfSpeech::Adverb => "adverb",
            PartOfSpeech::Pronoun => "pronoun",
            PartOfSpeech::Preposition => "preposition",
            PartOfSpeech::Conjunction => "conjunction",
            PartOfSpeech::Determiner => "determiner",
            PartOfSpeech::Numeral => "numeral",
            PartOfSpeech::Interjection => "interjection",
            PartOfSpeech::Other => "other",
        };
        f.write_str(s)
    }
}
