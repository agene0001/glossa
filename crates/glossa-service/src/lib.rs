//! `glossa-service` — transport-agnostic orchestration (spec §4.3).
//!
//! Plain async functions over the domain crates. **No Tauri types, no HTTP
//! types.** `src-tauri` calls these from IPC commands today; a future
//! `glossa-api` (Axum) will call the exact same functions over HTTP (spec §9).
//! Keep this crate free of any transport detail.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use glossa_content::ContentGenerator;
use glossa_core::{
    ContentRequest, ContentResponse, LanguageCode, LearnerId, LearnerProfile, Lexeme, LexemeId,
    LexemeState, MasteryState, PartOfSpeech, Token, TokenStatus, Unit, WordInfo,
};
use glossa_graph::mastery::{apply_grammar_exposure, apply_lexeme_exposure, effective_mastery};
use glossa_graph::select::{next_best_content, overview};
use glossa_graph::{GraphConfig, GraphOverview};
use glossa_storage::{Store, StoredStory};

/// Errors surfaced to the transport layer.
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error(transparent)]
    Storage(#[from] glossa_storage::StorageError),
    #[error(transparent)]
    Content(#[from] glossa_content::ContentError),
    #[error("not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, ServiceError>;

/// Fetch (creating on first run) the single V1 learner. The seam where
/// auth/session lookup lands in Phase 4.
pub async fn default_learner(
    store: &dyn Store,
    target_language: LanguageCode,
    native_language: LanguageCode,
) -> Result<LearnerProfile> {
    Ok(store
        .get_or_create_default_learner(target_language, native_language)
        .await?)
}

/// Generate the next most useful piece of content for a learner.
///
/// Pipeline (spec §4.3): graph picks the request → generator produces text +
/// structured word lists → we resolve those to ids, tokenize for highlighting,
/// and persist the story so a later "read" event can credit exposures.
pub async fn next_content(
    store: &dyn Store,
    generator: &dyn ContentGenerator,
    cfg: &GraphConfig,
    learner_id: LearnerId,
) -> Result<ContentResponse> {
    let profile = store
        .get_learner(learner_id)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("learner {learner_id}")))?;
    let language = profile.target_language.clone();

    let lexemes = store.lexemes(&language).await?;
    let lexeme_states = store.lexeme_states(learner_id).await?;
    let grammar_patterns = store.grammar_patterns(&language).await?;
    let grammar_states = store.grammar_states(learner_id).await?;

    let now = Utc::now();
    let request = next_best_content(
        &profile,
        &lexemes,
        &lexeme_states,
        &grammar_patterns,
        &grammar_states,
        cfg,
        now,
    );
    generate_from_request(
        store,
        generator,
        cfg,
        learner_id,
        &lexemes,
        &lexeme_states,
        now,
        request,
    )
    .await
}

/// Shared back half of content generation: take a chosen [`ContentRequest`],
/// generate it, resolve words to ids, tokenize for highlighting, persist the
/// story, and build the frontend response. Used by both free practice
/// ([`next_content`]) and unit-scoped practice ([`next_content_for_unit`]).
#[allow(clippy::too_many_arguments)]
async fn generate_from_request(
    store: &dyn Store,
    generator: &dyn ContentGenerator,
    cfg: &GraphConfig,
    learner_id: LearnerId,
    lexemes: &[Lexeme],
    lexeme_states: &[LexemeState],
    now: DateTime<Utc>,
    request: ContentRequest,
) -> Result<ContentResponse> {
    let language = request.language.clone();
    let kind = request.kind;
    let grammar_pattern_id = request.grammar_target.as_ref().map(|g| g.id);

    let generated = generator.generate(&request).await?;

    // --- resolve lemmas → ids, build display tokens -------------------------
    let lex_by_lemma: HashMap<String, &Lexeme> = lexemes
        .iter()
        .map(|l| (l.lemma.to_lowercase(), l))
        .collect();
    let lemma_to_id: HashMap<String, LexemeId> =
        lex_by_lemma.iter().map(|(k, l)| (k.clone(), l.id)).collect();
    let lemma_gloss: HashMap<String, Option<String>> = lex_by_lemma
        .iter()
        .map(|(k, l)| (k.clone(), l.gloss.clone()))
        .collect();

    // Effective (decayed) status per lexeme the learner has state for.
    let id_status: HashMap<LexemeId, TokenStatus> = lexeme_states
        .iter()
        .map(|s| {
            let m = effective_mastery(s.mastery, s.last_seen_at, now, &cfg.mastery);
            (s.lexeme_id, mastery_to_token(m))
        })
        .collect();

    let new_set: HashSet<String> = generated
        .new_words_introduced
        .iter()
        .map(|w| w.to_lowercase())
        .collect();

    let (tokens, known_ratio) =
        tokenize(&generated.text, &new_set, &lemma_to_id, &id_status, &lemma_gloss);

    // Words used/introduced, resolved to ids for the event-driven mastery model.
    let resolve = |lemmas: &[String]| -> Vec<LexemeId> {
        lemmas
            .iter()
            .filter_map(|w| lemma_to_id.get(&w.to_lowercase()).copied())
            .collect()
    };
    let mut all_ids = resolve(&generated.known_words_used);
    let new_ids = resolve(&generated.new_words_introduced);
    all_ids.extend(new_ids.iter().copied());
    all_ids.sort();
    all_ids.dedup();

    let new_words: Vec<WordInfo> = generated
        .new_words_introduced
        .iter()
        .map(|lemma| {
            let lx = lex_by_lemma.get(&lemma.to_lowercase());
            WordInfo {
                lemma: lemma.clone(),
                lexeme_id: lx.map(|l| l.id),
                pos: lx.map(|l| l.pos),
                gloss: lx.and_then(|l| l.gloss.clone()),
            }
        })
        .collect();

    let story_id = Uuid::new_v4();
    store
        .save_story(&StoredStory {
            id: story_id,
            learner_id,
            language: language.clone(),
            text: generated.text.clone(),
            lexeme_ids: all_ids,
            new_lexeme_ids: new_ids,
            grammar_pattern_id,
            known_word_ratio: known_ratio,
            generated_at: now,
        })
        .await?;

    Ok(ContentResponse {
        story_id,
        language,
        kind,
        text: generated.text,
        tokens,
        new_words,
        translation: generated.translation,
        grammar_targeted: generated.grammar_targeted,
        known_ratio,
    })
}

/// Record that the learner read a story, crediting an exposure to every word
/// (and the targeted grammar pattern) it contained. This is the only path that
/// moves mastery — the graph folds the event over current state (spec §9).
pub async fn record_story_read(
    store: &dyn Store,
    cfg: &GraphConfig,
    learner_id: LearnerId,
    story_id: Uuid,
    understood: bool,
) -> Result<()> {
    let story = store
        .get_story(story_id)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("story {story_id}")))?;

    let now = Utc::now();

    // Update lexeme mastery for every word seen.
    let mut states: HashMap<LexemeId, LexemeState> = store
        .lexeme_states(learner_id)
        .await?
        .into_iter()
        .map(|s| (s.lexeme_id, s))
        .collect();
    let mut updated = Vec::with_capacity(story.lexeme_ids.len());
    for id in &story.lexeme_ids {
        let current = states
            .remove(id)
            .unwrap_or_else(|| LexemeState::unseen(*id));
        updated.push(apply_lexeme_exposure(current, understood, now, &cfg.mastery));
    }
    store.upsert_lexeme_states(learner_id, &updated).await?;

    // Update grammar mastery for the targeted pattern, if any.
    if let Some(pattern_id) = story.grammar_pattern_id {
        let current = store
            .grammar_states(learner_id)
            .await?
            .into_iter()
            .find(|s| s.pattern_id == pattern_id)
            .unwrap_or_else(|| glossa_core::GrammarState::unseen(pattern_id));
        let next = apply_grammar_exposure(current, understood, now, &cfg.mastery);
        store.upsert_grammar_states(learner_id, &[next]).await?;
    }

    store
        .append_event(
            learner_id,
            &glossa_core::LearningEvent::StoryRead {
                story_id,
                words_seen: story.lexeme_ids,
                understood,
            },
        )
        .await?;
    Ok(())
}

/// Counts and the priority queue for the Review view.
pub async fn graph_overview(
    store: &dyn Store,
    cfg: &GraphConfig,
    learner_id: LearnerId,
    queue_len: usize,
) -> Result<GraphOverview> {
    let profile = store
        .get_learner(learner_id)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("learner {learner_id}")))?;
    let language = profile.target_language;

    let lexemes = store.lexemes(&language).await?;
    let lexeme_states = store.lexeme_states(learner_id).await?;
    let grammar_patterns = store.grammar_patterns(&language).await?;
    let grammar_states = store.grammar_states(learner_id).await?;

    Ok(overview(
        &language,
        &lexemes,
        &lexeme_states,
        &grammar_patterns,
        &grammar_states,
        cfg,
        Utc::now(),
        queue_len,
    ))
}

/// Manually set a lexeme's mastery (e.g. the learner taps "I already know this"
/// in the Review view). Seeds the graph without waiting for exposures.
pub async fn set_lexeme_status(
    store: &dyn Store,
    learner_id: LearnerId,
    lexeme_id: LexemeId,
    mastery: MasteryState,
) -> Result<()> {
    let existing = store
        .lexeme_states(learner_id)
        .await?
        .into_iter()
        .find(|s| s.lexeme_id == lexeme_id);
    let exposure_count = existing.as_ref().map(|s| s.exposure_count).unwrap_or(0);
    let state = LexemeState {
        lexeme_id,
        mastery,
        exposure_count,
        last_seen_at: Some(Utc::now()),
    };
    store.upsert_lexeme_states(learner_id, &[state]).await?;
    Ok(())
}

// --- curriculum / roadmap ------------------------------------------------

/// A unit as shown on the roadmap: progress + whether it's reachable yet.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RoadmapUnit {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub target_total: usize,
    pub known: usize,
    pub partial: usize,
    /// Mastery-weighted progress, 0..=100 (Known = full, Partial = half).
    pub percent: u32,
    pub state: UnitState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnitState {
    Locked,
    Active,
    Done,
}

/// One of a unit's target words, with the learner's current status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitWord {
    pub lexeme_id: i64,
    pub lemma: String,
    pub pos: PartOfSpeech,
    pub gloss: Option<String>,
    pub status: TokenStatus,
}

/// An authored example, tokenized for highlighting.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LessonExample {
    pub tokens: Vec<Token>,
    pub translation: String,
}

/// The full lesson payload for one unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitLesson {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub grammar: Option<String>,
    pub examples: Vec<LessonExample>,
    pub words: Vec<UnitWord>,
    pub percent: u32,
}

/// Mastery-weighted progress over a unit's target words.
fn unit_progress(
    unit: &Unit,
    states: &HashMap<LexemeId, &LexemeState>,
    cfg: &GraphConfig,
    now: DateTime<Utc>,
) -> (usize, usize, u32) {
    let (mut known, mut partial, mut score) = (0usize, 0usize, 0.0f32);
    for id in &unit.target_lexemes {
        let m = states
            .get(id)
            .map(|s| effective_mastery(s.mastery, s.last_seen_at, now, &cfg.mastery))
            .unwrap_or(MasteryState::Unknown);
        match m {
            MasteryState::Known => {
                known += 1;
                score += 1.0;
            }
            MasteryState::Partial { .. } => {
                partial += 1;
                score += 0.5;
            }
            MasteryState::Unknown => {}
        }
    }
    let total = unit.target_lexemes.len();
    let percent = if total == 0 {
        0
    } else {
        ((score / total as f32) * 100.0).round() as u32
    };
    (known, partial, percent)
}

/// The learning roadmap: every unit with progress and lock state. A unit is
/// reachable once the previous one is at least half learned (spec direction:
/// give progress a visible shape on top of the graph).
pub async fn roadmap(
    store: &dyn Store,
    cfg: &GraphConfig,
    learner_id: LearnerId,
) -> Result<Vec<RoadmapUnit>> {
    let profile = store
        .get_learner(learner_id)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("learner {learner_id}")))?;
    let language = profile.target_language;

    let mut units = store.units(&language).await?;
    units.sort_by_key(|u| u.id);
    let states_vec = store.lexeme_states(learner_id).await?;
    let states = lexeme_state_map(&states_vec);
    let now = Utc::now();

    let mut out = Vec::with_capacity(units.len());
    let mut prev_percent = 100u32; // the first unit is always unlocked
    for (i, u) in units.iter().enumerate() {
        let (known, partial, percent) = unit_progress(u, &states, cfg, now);
        let unlocked = i == 0 || prev_percent >= 50;
        let state = if !unlocked {
            UnitState::Locked
        } else if percent >= 80 {
            UnitState::Done
        } else {
            UnitState::Active
        };
        out.push(RoadmapUnit {
            id: u.id.0,
            title: u.title.clone(),
            description: u.description.clone(),
            target_total: u.target_lexemes.len(),
            known,
            partial,
            percent,
            state,
        });
        prev_percent = percent;
    }
    Ok(out)
}

/// Build a `HashMap<LexemeId, &LexemeState>` view.
fn lexeme_state_map(states: &[LexemeState]) -> HashMap<LexemeId, &LexemeState> {
    states.iter().map(|s| (s.lexeme_id, s)).collect()
}

/// The lesson for one unit: its authored examples (tokenized + translated) and
/// its target vocabulary with the learner's current status.
pub async fn unit_lesson(
    store: &dyn Store,
    cfg: &GraphConfig,
    learner_id: LearnerId,
    unit_id: i64,
) -> Result<UnitLesson> {
    let profile = store
        .get_learner(learner_id)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("learner {learner_id}")))?;
    let language = profile.target_language;

    let unit = store
        .units(&language)
        .await?
        .into_iter()
        .find(|u| u.id.0 == unit_id)
        .ok_or_else(|| ServiceError::NotFound(format!("unit {unit_id}")))?;

    let lexemes = store.lexemes(&language).await?;
    let grammar_patterns = store.grammar_patterns(&language).await?;
    let states_vec = store.lexeme_states(learner_id).await?;
    let now = Utc::now();

    let lex_by_id: HashMap<LexemeId, &Lexeme> = lexemes.iter().map(|l| (l.id, l)).collect();
    let lemma_to_id: HashMap<String, LexemeId> = lexemes
        .iter()
        .map(|l| (l.lemma.to_lowercase(), l.id))
        .collect();
    let lemma_gloss: HashMap<String, Option<String>> = lexemes
        .iter()
        .map(|l| (l.lemma.to_lowercase(), l.gloss.clone()))
        .collect();
    let id_status: HashMap<LexemeId, TokenStatus> = states_vec
        .iter()
        .map(|s| {
            let m = effective_mastery(s.mastery, s.last_seen_at, now, &cfg.mastery);
            (s.lexeme_id, mastery_to_token(m))
        })
        .collect();

    // A unit target word counts as "new" for highlighting if still unknown.
    let new_set: HashSet<String> = unit
        .target_lexemes
        .iter()
        .filter(|id| id_status.get(id).copied().unwrap_or(TokenStatus::Unknown) == TokenStatus::Unknown)
        .filter_map(|id| lex_by_id.get(id).map(|l| l.lemma.to_lowercase()))
        .collect();

    let examples = unit
        .examples
        .iter()
        .map(|ex| {
            let (tokens, _) =
                tokenize(&ex.text, &new_set, &lemma_to_id, &id_status, &lemma_gloss);
            LessonExample {
                tokens,
                translation: ex.translation.clone(),
            }
        })
        .collect();

    let words: Vec<UnitWord> = unit
        .target_lexemes
        .iter()
        .filter_map(|id| lex_by_id.get(id).map(|l| (id, l)))
        .map(|(id, l)| UnitWord {
            lexeme_id: id.0,
            lemma: l.lemma.clone(),
            pos: l.pos,
            gloss: l.gloss.clone(),
            status: id_status.get(id).copied().unwrap_or(TokenStatus::Unknown),
        })
        .collect();

    let grammar = unit
        .target_pattern
        .and_then(|pid| grammar_patterns.iter().find(|p| p.id == pid).map(|p| p.label.clone()));

    let (_, _, percent) = unit_progress(&unit, &lexeme_state_map(&states_vec), cfg, now);

    Ok(UnitLesson {
        id: unit.id.0,
        title: unit.title,
        description: unit.description,
        grammar,
        examples,
        words,
        percent,
    })
}

/// Record that the learner studied a unit's lesson, crediting an exposure to
/// every target word (and the unit's grammar pattern). Advances roadmap
/// progress through the same event-driven mastery model as story reads.
pub async fn complete_unit_lesson(
    store: &dyn Store,
    cfg: &GraphConfig,
    learner_id: LearnerId,
    unit_id: i64,
    understood: bool,
) -> Result<()> {
    let profile = store
        .get_learner(learner_id)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("learner {learner_id}")))?;
    let language = profile.target_language;

    let unit = store
        .units(&language)
        .await?
        .into_iter()
        .find(|u| u.id.0 == unit_id)
        .ok_or_else(|| ServiceError::NotFound(format!("unit {unit_id}")))?;

    let now = Utc::now();
    let mut states: HashMap<LexemeId, LexemeState> = store
        .lexeme_states(learner_id)
        .await?
        .into_iter()
        .map(|s| (s.lexeme_id, s))
        .collect();
    let mut updated = Vec::with_capacity(unit.target_lexemes.len());
    for id in &unit.target_lexemes {
        let current = states.remove(id).unwrap_or_else(|| LexemeState::unseen(*id));
        updated.push(apply_lexeme_exposure(current, understood, now, &cfg.mastery));
    }
    store.upsert_lexeme_states(learner_id, &updated).await?;

    if let Some(pattern_id) = unit.target_pattern {
        let current = store
            .grammar_states(learner_id)
            .await?
            .into_iter()
            .find(|s| s.pattern_id == pattern_id)
            .unwrap_or_else(|| glossa_core::GrammarState::unseen(pattern_id));
        let next = apply_grammar_exposure(current, understood, now, &cfg.mastery);
        store.upsert_grammar_states(learner_id, &[next]).await?;
    }

    store
        .append_event(
            learner_id,
            &glossa_core::LearningEvent::StoryRead {
                story_id: Uuid::new_v4(),
                words_seen: unit.target_lexemes.clone(),
                understood,
            },
        )
        .await?;
    Ok(())
}

/// Extra AI practice scoped to a unit: new words are drawn only from that
/// unit's still-unknown vocabulary (the rest of the loop is shared).
pub async fn next_content_for_unit(
    store: &dyn Store,
    generator: &dyn ContentGenerator,
    cfg: &GraphConfig,
    learner_id: LearnerId,
    unit_id: i64,
) -> Result<ContentResponse> {
    let profile = store
        .get_learner(learner_id)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("learner {learner_id}")))?;
    let language = profile.target_language;

    let unit = store
        .units(&language)
        .await?
        .into_iter()
        .find(|u| u.id.0 == unit_id)
        .ok_or_else(|| ServiceError::NotFound(format!("unit {unit_id}")))?;

    let lexemes = store.lexemes(&language).await?;
    let lexeme_states = store.lexeme_states(learner_id).await?;
    let grammar_patterns = store.grammar_patterns(&language).await?;
    let now = Utc::now();

    let lex_by_id: HashMap<LexemeId, &Lexeme> = lexemes.iter().map(|l| (l.id, l)).collect();
    let mastery_of = |id: LexemeId| -> MasteryState {
        lexeme_states
            .iter()
            .find(|s| s.lexeme_id == id)
            .map(|s| effective_mastery(s.mastery, s.last_seen_at, now, &cfg.mastery))
            .unwrap_or(MasteryState::Unknown)
    };

    // Building blocks: any word the learner has met, most frequent first.
    let mut known_vocab: Vec<Lexeme> = lexemes
        .iter()
        .filter(|l| !mastery_of(l.id).is_unknown())
        .cloned()
        .collect();
    known_vocab.sort_by_key(|l| l.frequency_rank);
    known_vocab.truncate(cfg.next_content.known_vocab_window);

    // New words: only this unit's still-unknown target words.
    let mut new_targets: Vec<Lexeme> = unit
        .target_lexemes
        .iter()
        .filter_map(|id| lex_by_id.get(id).copied())
        .filter(|l| mastery_of(l.id).is_unknown())
        .cloned()
        .collect();
    new_targets.sort_by_key(|l| l.frequency_rank);
    new_targets.truncate(cfg.next_content.new_word_budget.max(1));

    let grammar_target = unit
        .target_pattern
        .and_then(|pid| grammar_patterns.iter().find(|p| p.id == pid).cloned());

    let request = ContentRequest {
        learner_id,
        language: language.clone(),
        kind: cfg.next_content.kind,
        known_vocab,
        new_targets,
        grammar_target,
        known_ratio: cfg.next_content.known_ratio,
    };

    generate_from_request(
        store,
        generator,
        cfg,
        learner_id,
        &lexemes,
        &lexeme_states,
        now,
        request,
    )
    .await
}

fn mastery_to_token(m: MasteryState) -> TokenStatus {
    match m {
        MasteryState::Known => TokenStatus::Known,
        MasteryState::Partial { .. } => TokenStatus::Partial,
        MasteryState::Unknown => TokenStatus::Unknown,
    }
}

/// Split text into renderable tokens, tagging each word by the learner's status,
/// and return the measured fraction of word-tokens that are Known or Partial.
///
/// V1 matches surface forms to lemmas by lowercasing only — inflected forms
/// won't match the flat lemma table (acceptable per spec §11.5).
fn tokenize(
    text: &str,
    new_lemmas: &HashSet<String>,
    lemma_to_id: &HashMap<String, LexemeId>,
    id_status: &HashMap<LexemeId, TokenStatus>,
    lemma_gloss: &HashMap<String, Option<String>>,
) -> (Vec<Token>, f32) {
    // Group consecutive chars by alphabetic-ness so punctuation/whitespace is
    // preserved verbatim as non-word tokens.
    let mut groups: Vec<(String, bool)> = Vec::new();
    for ch in text.chars() {
        let is_word = ch.is_alphabetic();
        match groups.last_mut() {
            Some((s, w)) if *w == is_word => s.push(ch),
            _ => groups.push((ch.to_string(), is_word)),
        }
    }

    let mut tokens = Vec::with_capacity(groups.len());
    let mut word_count = 0usize;
    let mut known_like = 0usize;

    for (text, is_word) in groups {
        if !is_word {
            tokens.push(Token {
                text,
                is_word: false,
                status: None,
                lexeme_id: None,
                gloss: None,
            });
            continue;
        }
        word_count += 1;
        let norm = text.to_lowercase();
        let (status, lexeme_id) = if new_lemmas.contains(&norm) {
            (TokenStatus::New, lemma_to_id.get(&norm).copied())
        } else if let Some(id) = lemma_to_id.get(&norm) {
            let st = id_status.get(id).copied().unwrap_or(TokenStatus::Unknown);
            (st, Some(*id))
        } else {
            (TokenStatus::Unknown, None)
        };
        if matches!(status, TokenStatus::Known | TokenStatus::Partial) {
            known_like += 1;
        }
        let gloss = lemma_gloss.get(&norm).cloned().flatten();
        tokens.push(Token {
            text,
            is_word: true,
            status: Some(status),
            lexeme_id,
            gloss,
        });
    }

    let ratio = if word_count == 0 {
        1.0
    } else {
        known_like as f32 / word_count as f32
    };
    (tokens, ratio)
}

#[cfg(test)]
mod tests {
    use super::*;
    use glossa_content::MockContentGenerator;
    use glossa_core::{ExampleSentence, PartOfSpeech, UnitId};
    use glossa_storage::FileStore;

    fn lex(id: i64, lemma: &str, rank: u32) -> Lexeme {
        Lexeme {
            id: LexemeId(id),
            language: LanguageCode::spanish(),
            lemma: lemma.into(),
            pos: PartOfSpeech::Noun,
            frequency_rank: rank,
            gloss: Some(format!("{lemma}-en")),
        }
    }

    #[tokio::test]
    async fn full_loop_generates_then_records_and_moves_mastery() {
        let store = FileStore::ephemeral().unwrap();
        let cfg = GraphConfig::default();
        store
            .upsert_lexemes(&[lex(1, "yo", 1), lex(2, "comer", 2), lex(3, "pizza", 3)])
            .await
            .unwrap();
        let learner = default_learner(&store, LanguageCode::spanish(), LanguageCode::english())
            .await
            .unwrap();

        // Generate (mock) — picks unknown words as new targets.
        let resp = next_content(&store, &MockContentGenerator, &cfg, learner.id)
            .await
            .unwrap();
        assert!(!resp.tokens.is_empty());

        // Read it understood, several times → at least one word becomes Known.
        for _ in 0..8 {
            let r = next_content(&store, &MockContentGenerator, &cfg, learner.id)
                .await
                .unwrap();
            record_story_read(&store, &cfg, learner.id, r.story_id, true)
                .await
                .unwrap();
        }
        let ov = graph_overview(&store, &cfg, learner.id, 10).await.unwrap();
        assert!(ov.known + ov.partial >= 1, "overview: {ov:?}");
    }

    #[tokio::test]
    async fn manual_status_seeds_the_graph() {
        let store = FileStore::ephemeral().unwrap();
        store.upsert_lexemes(&[lex(1, "agua", 1)]).await.unwrap();
        let learner = default_learner(&store, LanguageCode::spanish(), LanguageCode::english())
            .await
            .unwrap();
        set_lexeme_status(&store, learner.id, LexemeId(1), MasteryState::Known)
            .await
            .unwrap();
        let ov = graph_overview(&store, &GraphConfig::default(), learner.id, 10)
            .await
            .unwrap();
        assert_eq!(ov.known, 1);
    }

    #[tokio::test]
    async fn roadmap_progress_and_unit_lesson() {
        let store = FileStore::ephemeral().unwrap();
        let cfg = GraphConfig::default();
        store
            .upsert_lexemes(&[lex(1, "yo", 1), lex(2, "comer", 2), lex(3, "pan", 3)])
            .await
            .unwrap();
        let learner = default_learner(&store, LanguageCode::spanish(), LanguageCode::english())
            .await
            .unwrap();
        store
            .upsert_units(&[Unit {
                id: UnitId(1),
                language: LanguageCode::spanish(),
                title: "Eating".into(),
                description: "food".into(),
                target_lexemes: vec![LexemeId(1), LexemeId(2), LexemeId(3)],
                target_pattern: None,
                examples: vec![ExampleSentence {
                    text: "Yo como pan.".into(),
                    translation: "I eat bread.".into(),
                }],
            }])
            .await
            .unwrap();

        // Fresh learner → unit is active, 0%.
        let rm = roadmap(&store, &cfg, learner.id).await.unwrap();
        assert_eq!(rm.len(), 1);
        assert_eq!(rm[0].state, UnitState::Active);
        assert_eq!(rm[0].percent, 0);

        // Lesson tokenizes the authored example and lists the vocab.
        let lesson = unit_lesson(&store, &cfg, learner.id, 1).await.unwrap();
        assert!(!lesson.examples[0].tokens.is_empty());
        assert_eq!(lesson.words.len(), 3);

        // Studying the unit advances its words → unit becomes done.
        for _ in 0..6 {
            complete_unit_lesson(&store, &cfg, learner.id, 1, true)
                .await
                .unwrap();
        }
        let rm2 = roadmap(&store, &cfg, learner.id).await.unwrap();
        assert_eq!(rm2[0].state, UnitState::Done, "got {:?}", rm2[0]);
    }
}
