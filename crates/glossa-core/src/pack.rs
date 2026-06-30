//! Vocabulary packs — the breadth track.
//!
//! A pack is a themed deck of words (Food, Travel, Feelings…), decoupled from
//! the grammar-ordered [`crate::Unit`] roadmap. Where a unit teaches the
//! language's *structure* (articles, conjugation, function words) in sequence,
//! a pack exists to grow raw vocabulary once that structure is in place — the
//! point past which gains come from word volume, not new grammar.
//!
//! Deliberately light to author: just a title, a theme, and a list of lexemes
//! (drawn from the existing inventory, including words no unit teaches). Mastery
//! still lives in the knowledge graph per word, so packs and units share — and
//! reinforce — the same progress.

use serde::{Deserialize, Serialize};

use crate::ids::{LexemeId, PackId};
use crate::lang::LanguageCode;

/// A themed vocabulary deck.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VocabPack {
    pub id: PackId,
    pub language: LanguageCode,
    pub title: String,
    /// A single emoji for the pack card, e.g. "🍽️".
    pub emoji: String,
    pub description: String,
    /// The words this pack groups; progress is measured over these.
    pub lexemes: Vec<LexemeId>,
}
