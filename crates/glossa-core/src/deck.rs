//! User-authored decks — the learner's own flashcard sets (Quizlet-style).
//!
//! Where [`crate::VocabPack`]s are seeded reference content, a deck is created
//! and owned by a learner: words they type in themselves (the vocab from a
//! class, a textbook chapter, a trip). The words are real lexemes — they live in
//! a separate, user-owned id range so they never collide with the seeded
//! inventory, and crucially they're kept *out* of the frequency-ranked "teach me
//! next" selection (they have no meaningful frequency). But their mastery flows
//! through the exact same knowledge graph + spaced-repetition as everything else,
//! so a custom word is studied, quizzed, and scheduled just like a seeded one.

use serde::{Deserialize, Serialize};

use crate::ids::{DeckId, LearnerId, LexemeId};
use crate::lang::LanguageCode;

/// A learner's own flashcard set.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Deck {
    pub id: DeckId,
    /// The learner who owns this deck (decks are private to their creator).
    pub learner_id: LearnerId,
    /// The deck's language — V1 ties decks to the learner's active target
    /// language, so a Spanish session only ever sees Spanish decks.
    pub language: LanguageCode,
    pub title: String,
    /// A single emoji for the deck card.
    pub emoji: String,
    /// The user lexemes this deck contains (ids into the user-owned range).
    pub lexemes: Vec<LexemeId>,
}
