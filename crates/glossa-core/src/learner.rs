//! The learner profile.

use serde::{Deserialize, Serialize};

use crate::ids::LearnerId;
use crate::lang::LanguageCode;

/// Who is learning, and the language pair.
///
/// `native_language` is carried from V1 even though V1 never matches humans,
/// because Phase 4 language-exchange matching pairs on `(native, target)` and
/// retrofitting that field later would mean a migration (spec §2.4, §9).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LearnerProfile {
    pub id: LearnerId,
    pub target_language: LanguageCode,
    pub native_language: LanguageCode,
}

impl LearnerProfile {
    pub fn new(target_language: LanguageCode, native_language: LanguageCode) -> Self {
        Self {
            id: LearnerId::new(),
            target_language,
            native_language,
        }
    }
}
