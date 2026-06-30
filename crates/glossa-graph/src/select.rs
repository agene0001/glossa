//! Selection: what to teach next, and the Review-view summary.
//!
//! `next_best_content` answers the product's only question — "teach me the next
//! most useful thing" (spec §2.6) — by combining frequency (teach common words
//! first) with the learner's current, decayed mastery.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use glossa_core::{
    ContentKind, ContentRequest, GrammarPattern, GrammarState, LanguageCode, LearnerProfile,
    Lexeme, LexemeId, LexemeState, MasteryState, PartOfSpeech, PatternId,
};

use crate::mastery::effective_mastery;
use crate::GraphConfig;

/// Knobs for next-content selection.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct NextContentConfig {
    pub kind: ContentKind,
    /// New words to introduce once the learner has a working vocabulary.
    pub new_word_budget: usize,
    /// How many known words to hand the generator as building material.
    pub known_vocab_window: usize,
    /// Target share of known words, default 0.95 (spec §2.1).
    pub known_ratio: f32,
    /// Below this many known words we're in "cold start" and teach more at once.
    pub cold_start_known_threshold: usize,
    pub cold_start_new_budget: usize,
}

impl Default for NextContentConfig {
    fn default() -> Self {
        Self {
            kind: ContentKind::Story,
            new_word_budget: 2,
            known_vocab_window: 40,
            known_ratio: 0.95,
            cold_start_known_threshold: 10,
            cold_start_new_budget: 5,
        }
    }
}

/// Index states by lexeme id for O(1) lookup.
fn lexeme_state_map(states: &[LexemeState]) -> HashMap<LexemeId, &LexemeState> {
    states.iter().map(|s| (s.lexeme_id, s)).collect()
}

/// Effective mastery for one lexeme right now (decayed; unseen = Unknown).
fn lexeme_mastery(
    lex: &Lexeme,
    states: &HashMap<LexemeId, &LexemeState>,
    cfg: &GraphConfig,
    now: DateTime<Utc>,
) -> MasteryState {
    match states.get(&lex.id) {
        Some(s) => effective_mastery(s.mastery, s.last_seen_at, now, &cfg.mastery),
        None => MasteryState::Unknown,
    }
}

/// Pick the next most useful piece of content for this learner.
pub fn next_best_content(
    profile: &LearnerProfile,
    lexemes: &[Lexeme],
    lexeme_states: &[LexemeState],
    grammar_patterns: &[GrammarPattern],
    grammar_states: &[GrammarState],
    cfg: &GraphConfig,
    now: DateTime<Utc>,
) -> ContentRequest {
    let states = lexeme_state_map(lexeme_states);

    // Building blocks: every word the learner has met (Known or any Partial),
    // most frequent first. Anything still Unknown is a teaching candidate
    // instead — so every lexeme falls into exactly one pool, never a limbo.
    let mut known_vocab: Vec<Lexeme> = lexemes
        .iter()
        .filter(|lex| !lexeme_mastery(lex, &states, cfg, now).is_unknown())
        .cloned()
        .collect();
    known_vocab.sort_by_key(|l| l.frequency_rank);
    known_vocab.truncate(cfg.next_content.known_vocab_window);

    // Candidates to teach: still-unknown words, most frequent first.
    let mut unknown: Vec<Lexeme> = lexemes
        .iter()
        .filter(|lex| lexeme_mastery(lex, &states, cfg, now).is_unknown())
        .cloned()
        .collect();
    unknown.sort_by_key(|l| l.frequency_rank);

    // Cold start (almost nothing known) → introduce more per piece, so the very
    // first stories have enough material to be stories at all.
    let budget = if known_vocab.len() < cfg.next_content.cold_start_known_threshold {
        cfg.next_content.cold_start_new_budget
    } else {
        cfg.next_content.new_word_budget
    };
    let new_targets: Vec<Lexeme> = unknown.into_iter().take(budget).collect();

    // Grammar: target the lowest-id pattern not yet known (seeded in teaching
    // order). Never named to the learner up front — it just shapes the content.
    let grammar_target = pick_grammar_target(grammar_patterns, grammar_states, cfg, now);

    ContentRequest {
        learner_id: profile.id,
        language: profile.target_language.clone(),
        kind: cfg.next_content.kind,
        known_vocab,
        new_targets,
        grammar_target,
        known_ratio: cfg.next_content.known_ratio,
    }
}

fn pick_grammar_target(
    patterns: &[GrammarPattern],
    states: &[GrammarState],
    cfg: &GraphConfig,
    now: DateTime<Utc>,
) -> Option<GrammarPattern> {
    let state_by_id: HashMap<PatternId, &GrammarState> =
        states.iter().map(|s| (s.pattern_id, s)).collect();
    let mut ordered: Vec<&GrammarPattern> = patterns.iter().collect();
    ordered.sort_by_key(|p| p.id);
    ordered
        .into_iter()
        .find(|p| {
            let m = match state_by_id.get(&p.id) {
                Some(s) => effective_mastery(s.mastery, s.last_seen_at, now, &cfg.mastery),
                None => MasteryState::Unknown,
            };
            !m.is_known()
        })
        .cloned()
}

// --- Review-view summary -------------------------------------------------

/// One upcoming word in the priority queue, with a human-readable reason.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueuedItem {
    pub lexeme_id: LexemeId,
    pub lemma: String,
    pub pos: PartOfSpeech,
    pub frequency_rank: u32,
    pub gloss: Option<String>,
    pub reason: String,
}

/// Counts by status plus what's queued next and why — backs the Review view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphOverview {
    pub language: LanguageCode,
    pub total_lexemes: usize,
    pub known: usize,
    pub partial: usize,
    pub unknown: usize,
    pub grammar_total: usize,
    pub grammar_known: usize,
    pub grammar_partial: usize,
    pub grammar_unknown: usize,
    pub next_queue: Vec<QueuedItem>,
}

/// Summarize the learner's whole graph for the Review view.
pub fn overview(
    language: &LanguageCode,
    lexemes: &[Lexeme],
    lexeme_states: &[LexemeState],
    grammar_patterns: &[GrammarPattern],
    grammar_states: &[GrammarState],
    cfg: &GraphConfig,
    now: DateTime<Utc>,
    queue_len: usize,
) -> GraphOverview {
    let states = lexeme_state_map(lexeme_states);

    let (mut known, mut partial, mut unknown) = (0usize, 0usize, 0usize);
    let mut unknown_lexemes: Vec<&Lexeme> = Vec::new();
    for lex in lexemes {
        match lexeme_mastery(lex, &states, cfg, now) {
            MasteryState::Known => known += 1,
            MasteryState::Partial { .. } => partial += 1,
            MasteryState::Unknown => {
                unknown += 1;
                unknown_lexemes.push(lex);
            }
        }
    }

    unknown_lexemes.sort_by_key(|l| l.frequency_rank);
    let next_queue = unknown_lexemes
        .into_iter()
        .take(queue_len)
        .map(|l| QueuedItem {
            lexeme_id: l.id,
            lemma: l.lemma.clone(),
            pos: l.pos,
            frequency_rank: l.frequency_rank,
            gloss: l.gloss.clone(),
            reason: format!(
                "Frequency rank #{} — one of the most common words you haven't met yet.",
                l.frequency_rank
            ),
        })
        .collect();

    let grammar_state_by_id: HashMap<PatternId, &GrammarState> =
        grammar_states.iter().map(|s| (s.pattern_id, s)).collect();
    let (mut g_known, mut g_partial, mut g_unknown) = (0usize, 0usize, 0usize);
    for p in grammar_patterns {
        let m = match grammar_state_by_id.get(&p.id) {
            Some(s) => effective_mastery(s.mastery, s.last_seen_at, now, &cfg.mastery),
            None => MasteryState::Unknown,
        };
        match m {
            MasteryState::Known => g_known += 1,
            MasteryState::Partial { .. } => g_partial += 1,
            MasteryState::Unknown => g_unknown += 1,
        }
    }

    GraphOverview {
        language: language.clone(),
        total_lexemes: lexemes.len(),
        known,
        partial,
        unknown,
        grammar_total: grammar_patterns.len(),
        grammar_known: g_known,
        grammar_partial: g_partial,
        grammar_unknown: g_unknown,
        next_queue,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glossa_core::LearnerId;

    fn lex(id: i64, lemma: &str, rank: u32) -> Lexeme {
        Lexeme {
            id: LexemeId(id),
            language: LanguageCode::spanish(),
            lemma: lemma.into(),
            pos: PartOfSpeech::Noun,
            frequency_rank: rank,
            gloss: None,
            transliteration: None,
        }
    }

    fn profile() -> LearnerProfile {
        LearnerProfile {
            id: LearnerId::new(),
            target_language: LanguageCode::spanish(),
            native_language: LanguageCode::english(),
        }
    }

    #[test]
    fn cold_start_introduces_most_frequent_unknown_words() {
        let lexemes = vec![lex(1, "de", 1), lex(2, "que", 2), lex(3, "no", 3)];
        let cfg = GraphConfig::default();
        let req = next_best_content(&profile(), &lexemes, &[], &[], &[], &cfg, Utc::now());

        // Nothing known yet → cold-start budget, most-frequent-first.
        assert!(req.known_vocab.is_empty());
        assert_eq!(req.new_targets.first().unwrap().lemma, "de");
        assert!(req.new_targets.len() <= cfg.next_content.cold_start_new_budget);
    }

    #[test]
    fn known_words_become_building_blocks_not_targets() {
        let lexemes = vec![lex(1, "de", 1), lex(2, "que", 2)];
        let states = vec![LexemeState {
            lexeme_id: LexemeId(1),
            mastery: MasteryState::Known,
            exposure_count: 10,
            last_seen_at: Some(Utc::now()),
        }];
        let req = next_best_content(
            &profile(),
            &lexemes,
            &states,
            &[],
            &[],
            &GraphConfig::default(),
            Utc::now(),
        );
        assert!(req.known_vocab.iter().any(|l| l.lemma == "de"));
        assert!(req.new_targets.iter().all(|l| l.lemma != "de"));
    }

    #[test]
    fn overview_counts_and_serializes() {
        let lexemes = vec![lex(1, "de", 1), lex(2, "que", 2), lex(3, "no", 3)];
        let states = vec![LexemeState {
            lexeme_id: LexemeId(1),
            mastery: MasteryState::Known,
            exposure_count: 10,
            last_seen_at: Some(Utc::now()),
        }];
        let ov = overview(
            &LanguageCode::spanish(),
            &lexemes,
            &states,
            &[],
            &[],
            &GraphConfig::default(),
            Utc::now(),
            10,
        );
        assert_eq!(ov.total_lexemes, 3);
        assert_eq!(ov.known, 1);
        assert_eq!(ov.unknown, 2);
        assert_eq!(ov.next_queue.first().unwrap().lemma, "que");
        // Must round-trip to JSON for the frontend.
        let json = serde_json::to_string(&ov).unwrap();
        assert!(json.contains("\"known\":1"));
    }
}
