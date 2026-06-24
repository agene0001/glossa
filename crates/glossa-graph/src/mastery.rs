//! Mastery state transitions with recency decay.
//!
//! The model is deliberately a simple, legible heuristic (spec §11.3): a single
//! continuous confidence in `0.0..=1.0`, derived from `MasteryState`, that
//! - **decays** toward 0 with time since `last_seen` (exponential, by half-life),
//! - **rises** with comprehensible exposures and correct exercises,
//! - **falls** with incorrect exercises,
//! then re-buckets into `Unknown / Partial / Known` by threshold.
//!
//! Because `Known == 1.0` and `Unknown == 0.0`, decay naturally turns a long-
//! unseen `Known` word back into `Partial` (i.e. "due for review") without any
//! extra bookkeeping.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use glossa_core::{GrammarState, LexemeState, MasteryState};

/// Knobs for the mastery model. Tune freely (spec §11.3).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MasteryConfig {
    /// Confidence gained from a comprehensible exposure (story understood).
    pub exposure_gain: f32,
    /// Confidence gained from an exposure the learner didn't fully follow.
    pub weak_exposure_gain: f32,
    /// Confidence gained from a correct exercise answer.
    pub correct_gain: f32,
    /// Confidence lost from an incorrect exercise answer.
    pub incorrect_penalty: f32,
    /// At/above this confidence an item is `Known`.
    pub known_threshold: f32,
    /// Days for confidence to halve when unseen.
    pub decay_half_life_days: f32,
}

impl Default for MasteryConfig {
    fn default() -> Self {
        Self {
            exposure_gain: 0.18,
            weak_exposure_gain: 0.05,
            correct_gain: 0.25,
            incorrect_penalty: 0.30,
            known_threshold: 0.85,
            decay_half_life_days: 30.0,
        }
    }
}

/// Bucket a raw confidence into a discrete mastery state.
fn bucket(confidence: f32, cfg: &MasteryConfig) -> MasteryState {
    let c = confidence.clamp(0.0, 1.0);
    if c >= cfg.known_threshold {
        MasteryState::Known
    } else if c <= f32::EPSILON {
        MasteryState::Unknown
    } else {
        MasteryState::Partial { confidence: c }
    }
}

/// Confidence after applying time-decay from `last_seen` to `now`.
fn decay(base: f32, last_seen: Option<DateTime<Utc>>, now: DateTime<Utc>, cfg: &MasteryConfig) -> f32 {
    let Some(seen) = last_seen else {
        return base;
    };
    let elapsed_days = (now - seen).num_seconds().max(0) as f32 / 86_400.0;
    if cfg.decay_half_life_days <= 0.0 {
        return base;
    }
    base * 0.5f32.powf(elapsed_days / cfg.decay_half_life_days)
}

/// The item's *current* mastery, accounting for decay since it was last seen,
/// without recording a new exposure. Used for selection and the Review view.
pub fn effective_mastery(
    mastery: MasteryState,
    last_seen: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    cfg: &MasteryConfig,
) -> MasteryState {
    bucket(decay(mastery.confidence(), last_seen, now, cfg), cfg)
}

/// Apply gain to a (decayed) confidence and return the updated mastery + value.
fn advance(
    current: MasteryState,
    last_seen: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    delta: f32,
    cfg: &MasteryConfig,
) -> MasteryState {
    let decayed = decay(current.confidence(), last_seen, now, cfg);
    bucket(decayed + delta, cfg)
}

/// Record that the learner saw this lexeme in comprehensible context.
pub fn apply_lexeme_exposure(
    mut state: LexemeState,
    understood: bool,
    now: DateTime<Utc>,
    cfg: &MasteryConfig,
) -> LexemeState {
    let delta = if understood {
        cfg.exposure_gain
    } else {
        cfg.weak_exposure_gain
    };
    state.mastery = advance(state.mastery, state.last_seen_at, now, delta, cfg);
    state.exposure_count = state.exposure_count.saturating_add(1);
    state.last_seen_at = Some(now);
    state
}

/// Record an explicit exercise result for this lexeme.
pub fn apply_lexeme_exercise(
    mut state: LexemeState,
    correct: bool,
    now: DateTime<Utc>,
    cfg: &MasteryConfig,
) -> LexemeState {
    let delta = if correct {
        cfg.correct_gain
    } else {
        -cfg.incorrect_penalty
    };
    state.mastery = advance(state.mastery, state.last_seen_at, now, delta, cfg);
    state.exposure_count = state.exposure_count.saturating_add(1);
    state.last_seen_at = Some(now);
    state
}

/// Record that the learner saw this grammar pattern in context (spec §2.2).
pub fn apply_grammar_exposure(
    mut state: GrammarState,
    understood: bool,
    now: DateTime<Utc>,
    cfg: &MasteryConfig,
) -> GrammarState {
    let delta = if understood {
        cfg.exposure_gain
    } else {
        cfg.weak_exposure_gain
    };
    state.mastery = advance(state.mastery, state.last_seen_at, now, delta, cfg);
    state.exposure_count = state.exposure_count.saturating_add(1);
    state.last_seen_at = Some(now);
    state
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;
    use glossa_core::LexemeId;

    fn fresh() -> LexemeState {
        LexemeState::unseen(LexemeId(1))
    }

    #[test]
    fn repeated_exposure_moves_unknown_to_known() {
        let cfg = MasteryConfig::default();
        let mut now = Utc::now();
        let mut state = fresh();
        assert!(state.mastery.is_unknown());

        // A handful of understood exposures, spaced a day apart.
        for _ in 0..8 {
            state = apply_lexeme_exposure(state, true, now, &cfg);
            now += Duration::days(1);
        }
        assert!(state.mastery.is_known(), "got {:?}", state.mastery);
        assert_eq!(state.exposure_count, 8);
    }

    #[test]
    fn one_exposure_yields_partial_not_known() {
        let cfg = MasteryConfig::default();
        let state = apply_lexeme_exposure(fresh(), true, Utc::now(), &cfg);
        matches!(state.mastery, MasteryState::Partial { .. });
        assert!(!state.mastery.is_known());
    }

    #[test]
    fn long_gap_decays_known_back_to_review() {
        let cfg = MasteryConfig::default();
        let now = Utc::now();
        let known = LexemeState {
            lexeme_id: LexemeId(1),
            mastery: MasteryState::Known,
            exposure_count: 10,
            last_seen_at: Some(now - Duration::days(120)),
        };
        // Four half-lives later, confidence ~1/16 — no longer "known".
        let eff = effective_mastery(known.mastery, known.last_seen_at, now, &cfg);
        assert!(!eff.is_known(), "expected decay below threshold, got {eff:?}");
    }

    #[test]
    fn incorrect_answer_reduces_confidence() {
        let cfg = MasteryConfig::default();
        let now = Utc::now();
        let partial = LexemeState {
            lexeme_id: LexemeId(1),
            mastery: MasteryState::Partial { confidence: 0.6 },
            exposure_count: 3,
            last_seen_at: Some(now),
        };
        let after = apply_lexeme_exercise(partial, false, now, &cfg);
        assert!(after.mastery.confidence() < 0.6);
    }
}
