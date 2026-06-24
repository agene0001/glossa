//! Strongly-typed identifiers.
//!
//! `LearnerId` is a UUID from day one (per spec §9: a real type even with one
//! row) so multi-user is a data concern, not a refactor. `LexemeId`/`PatternId`
//! are `i64` because they index seeded reference tables ordered by frequency.

use std::fmt;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Identifies a single learner. UUID so it never collides across users/devices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LearnerId(pub Uuid);

impl LearnerId {
    /// Mint a fresh random learner id.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for LearnerId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for LearnerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for LearnerId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

/// Identifies a lexeme (a dictionary headword) in the seeded inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct LexemeId(pub i64);

impl fmt::Display for LexemeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i64> for LexemeId {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

/// Identifies a grammar pattern (tracked exactly like a lexeme — spec §2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct PatternId(pub i64);

impl fmt::Display for PatternId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<i64> for PatternId {
    fn from(value: i64) -> Self {
        Self(value)
    }
}
