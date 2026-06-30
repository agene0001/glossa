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
    ContentRequest, ContentResponse, Deck, DeckId, LanguageCode, LearnerId, LearnerProfile, Lexeme,
    LexemeId, LexemeState, MasteryState, PartOfSpeech, Token, TokenStatus, Unit, WordInfo,
};
use glossa_graph::mastery::{
    apply_grammar_exposure, apply_lexeme_exercise, apply_lexeme_exposure, effective_mastery,
};
use rand::seq::SliceRandom;
use rand::{Rng, RngExt};
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

    // --- resolve surface forms → ids, build display tokens -----------------
    let lex_by_lemma: HashMap<String, &Lexeme> = lexemes
        .iter()
        .map(|l| (l.lemma.to_lowercase(), l))
        .collect();
    // Index that resolves inflected forms (conjugations, plurals) to a lexeme,
    // plus meaning looked up by id.
    let form_index = glossa_lemma::build_form_index(lexemes);
    let id_gloss: HashMap<LexemeId, Option<String>> =
        lexemes.iter().map(|l| (l.id, l.gloss.clone())).collect();
    let id_lemma: HashMap<LexemeId, String> =
        lexemes.iter().map(|l| (l.id, l.lemma.clone())).collect();

    // Effective (decayed) status per lexeme the learner has state for.
    let id_status: HashMap<LexemeId, TokenStatus> = lexeme_states
        .iter()
        .map(|s| {
            let m = effective_mastery(s.mastery, s.last_seen_at, now, &cfg.mastery);
            (s.lexeme_id, mastery_to_token(m))
        })
        .collect();

    // Lemmas the model reports are in the form index too, so resolve through it.
    let resolve = |lemmas: &[String]| -> Vec<LexemeId> {
        lemmas
            .iter()
            .filter_map(|w| form_index.get(&w.to_lowercase()).copied())
            .collect()
    };
    let new_ids = resolve(&generated.new_words_introduced);
    let new_id_set: HashSet<LexemeId> = new_ids.iter().copied().collect();

    let (tokens, known_ratio) = tokenize(
        &generated.text,
        &new_id_set,
        &form_index,
        &id_status,
        &id_gloss,
        &id_lemma,
    );

    // Words used/introduced, resolved to ids for the event-driven mastery model.
    let mut all_ids = resolve(&generated.known_words_used);
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
    /// CEFR level tag, e.g. "A1.1" — shows progression on the roadmap.
    pub level: String,
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

/// A unit's graded reading passage, tokenized for highlighting and tap-to-reveal
/// just like the examples — but at text length (the "books/blogs" core).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LessonReading {
    pub title: String,
    pub tokens: Vec<Token>,
    pub translation: String,
}

/// One row of a present-tense conjugation table (pronoun → form).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConjugationCell {
    pub pronoun: String,
    pub pronoun_gloss: String,
    pub form: String,
}

/// A verb the unit teaches, with its present-tense conjugation — so the lesson
/// explains that `soy`/`eres`/`es` are all `ser`, not separate words.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitConjugation {
    pub lemma: String,
    pub gloss: Option<String>,
    pub cells: Vec<ConjugationCell>,
}

/// The full lesson payload for one unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnitLesson {
    pub id: i64,
    pub title: String,
    pub description: String,
    /// CEFR level tag, e.g. "A1.1".
    pub level: String,
    /// Learner-facing "can-do" objective for this unit.
    pub objective: String,
    pub grammar: Option<String>,
    /// Learner-facing grammar explanation (opt-in tip), if the pattern has one.
    pub grammar_tip: Option<String>,
    /// A short graded reading passage at the unit's level, if authored.
    pub reading: Option<LessonReading>,
    /// Present-tense conjugations for each verb the unit teaches.
    pub conjugations: Vec<UnitConjugation>,
    pub examples: Vec<LessonExample>,
    pub words: Vec<UnitWord>,
    pub percent: u32,
}

/// Outcome of studying a unit, for the post-lesson celebration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LessonResult {
    pub newly_known: usize,
    pub percent: u32,
    pub done: bool,
    pub streak: u32,
}

/// Mastery-weighted progress over a set of lexemes (Known = full, Partial =
/// half). Returns (known, partial, percent). Shared by units and vocab packs.
fn progress_over(
    lexemes: &[LexemeId],
    states: &HashMap<LexemeId, &LexemeState>,
    cfg: &GraphConfig,
    now: DateTime<Utc>,
) -> (usize, usize, u32) {
    let (mut known, mut partial, mut score) = (0usize, 0usize, 0.0f32);
    for id in lexemes {
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
    let total = lexemes.len();
    let percent = if total == 0 {
        0
    } else {
        ((score / total as f32) * 100.0).round() as u32
    };
    (known, partial, percent)
}

/// Mastery-weighted progress over a unit's target words.
fn unit_progress(
    unit: &Unit,
    states: &HashMap<LexemeId, &LexemeState>,
    cfg: &GraphConfig,
    now: DateTime<Utc>,
) -> (usize, usize, u32) {
    progress_over(&unit.target_lexemes, states, cfg, now)
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
            level: u.level.clone(),
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
    let form_index = glossa_lemma::build_form_index(&lexemes);
    let id_gloss: HashMap<LexemeId, Option<String>> =
        lexemes.iter().map(|l| (l.id, l.gloss.clone())).collect();
    let id_lemma: HashMap<LexemeId, String> =
        lexemes.iter().map(|l| (l.id, l.lemma.clone())).collect();
    let id_status: HashMap<LexemeId, TokenStatus> = states_vec
        .iter()
        .map(|s| {
            let m = effective_mastery(s.mastery, s.last_seen_at, now, &cfg.mastery);
            (s.lexeme_id, mastery_to_token(m))
        })
        .collect();

    // A unit target word counts as "new" for highlighting while still unknown.
    let new_id_set: HashSet<LexemeId> = unit
        .target_lexemes
        .iter()
        .copied()
        .filter(|id| id_status.get(id).copied().unwrap_or(TokenStatus::Unknown) == TokenStatus::Unknown)
        .collect();

    let examples = unit
        .examples
        .iter()
        .map(|ex| {
            let (tokens, _) = tokenize(
                &ex.text,
                &new_id_set,
                &form_index,
                &id_status,
                &id_gloss,
                &id_lemma,
            );
            LessonExample {
                tokens,
                translation: ex.translation.clone(),
            }
        })
        .collect();

    let reading = unit.reading.as_ref().map(|r| {
        let (tokens, _) = tokenize(
            &r.text,
            &new_id_set,
            &form_index,
            &id_status,
            &id_gloss,
            &id_lemma,
        );
        LessonReading {
            title: r.title.clone(),
            tokens,
            translation: r.translation.clone(),
        }
    });

    // Present-tense table for every verb the unit teaches — the bridge from the
    // infinitive in the word list to the conjugated forms in the examples.
    let conjugations: Vec<UnitConjugation> = unit
        .target_lexemes
        .iter()
        .filter_map(|id| lex_by_id.get(id).copied())
        .filter_map(|l| {
            let cells: Vec<ConjugationCell> = glossa_lemma::present_tense(l)
                .into_iter()
                .map(|c| ConjugationCell {
                    pronoun: c.pronoun.to_string(),
                    pronoun_gloss: c.gloss.to_string(),
                    form: c.form,
                })
                .collect();
            (!cells.is_empty()).then(|| UnitConjugation {
                lemma: l.lemma.clone(),
                gloss: l.gloss.clone(),
                cells,
            })
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

    let pattern = unit
        .target_pattern
        .and_then(|pid| grammar_patterns.iter().find(|p| p.id == pid));
    let grammar = pattern.map(|p| p.label.clone());
    let grammar_tip = pattern.and_then(|p| p.explanation.clone());

    let (_, _, percent) = unit_progress(&unit, &lexeme_state_map(&states_vec), cfg, now);

    Ok(UnitLesson {
        id: unit.id.0,
        title: unit.title,
        description: unit.description,
        level: unit.level,
        objective: unit.objective,
        grammar,
        grammar_tip,
        reading,
        conjugations,
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
) -> Result<LessonResult> {
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

    let is_known = |st: Option<&LexemeState>| {
        st.map(|s| effective_mastery(s.mastery, s.last_seen_at, now, &cfg.mastery).is_known())
            .unwrap_or(false)
    };
    let known_before = unit
        .target_lexemes
        .iter()
        .filter(|id| is_known(states.get(id)))
        .count();

    let mut updated = Vec::with_capacity(unit.target_lexemes.len());
    for id in &unit.target_lexemes {
        let current = states.remove(id).unwrap_or_else(|| LexemeState::unseen(*id));
        updated.push(apply_lexeme_exposure(current, understood, now, &cfg.mastery));
    }
    store.upsert_lexeme_states(learner_id, &updated).await?;

    let known_after = updated.iter().filter(|s| is_known(Some(s))).count();
    let after_map: HashMap<LexemeId, &LexemeState> =
        updated.iter().map(|s| (s.lexeme_id, s)).collect();
    let (_, _, percent) = unit_progress(&unit, &after_map, cfg, now);

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

    let streak = compute_streak(&store.activity_dates(learner_id).await?, Utc::now());

    Ok(LessonResult {
        newly_known: known_after.saturating_sub(known_before),
        percent,
        done: percent >= 80,
        streak,
    })
}

/// The learner's current daily streak: consecutive days (UTC) with at least one
/// event, ending today or yesterday.
pub async fn streak(store: &dyn Store, learner_id: LearnerId) -> Result<u32> {
    Ok(compute_streak(
        &store.activity_dates(learner_id).await?,
        Utc::now(),
    ))
}

fn compute_streak(dates: &[DateTime<Utc>], now: DateTime<Utc>) -> u32 {
    let days: HashSet<i64> = dates.iter().map(|d| d.timestamp().div_euclid(86_400)).collect();
    if days.is_empty() {
        return 0;
    }
    let today = now.timestamp().div_euclid(86_400);
    let mut day = if days.contains(&today) {
        today
    } else if days.contains(&(today - 1)) {
        today - 1
    } else {
        return 0;
    };
    let mut count = 0u32;
    while days.contains(&day) {
        count += 1;
        day -= 1;
    }
    count
}

/// Switch the learner's active target language (e.g. "es" → "fr").
pub async fn set_target_language(
    store: &dyn Store,
    learner_id: LearnerId,
    code: &str,
) -> Result<()> {
    let mut profile = store
        .get_learner(learner_id)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("learner {learner_id}")))?;
    profile.target_language = LanguageCode::new(code);
    store.update_learner(&profile).await?;
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

// --- vocabulary packs (breadth track) ------------------------------------

/// A pack as shown in the grid: progress over its words.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackSummary {
    pub id: i64,
    pub title: String,
    pub emoji: String,
    pub description: String,
    pub total: usize,
    pub known: usize,
    pub partial: usize,
    pub percent: u32,
}

/// The flashcard deck for one pack: its words with the learner's status.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackLesson {
    pub id: i64,
    pub title: String,
    pub emoji: String,
    pub description: String,
    pub cards: Vec<UnitWord>,
    pub percent: u32,
}

/// Every vocabulary pack with the learner's progress — the breadth-track
/// equivalent of [`roadmap`]. Unlike units, packs aren't locked or ordered;
/// they're browsable by interest.
pub async fn vocab_packs(
    store: &dyn Store,
    cfg: &GraphConfig,
    learner_id: LearnerId,
) -> Result<Vec<PackSummary>> {
    let profile = store
        .get_learner(learner_id)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("learner {learner_id}")))?;
    let language = profile.target_language;

    let mut packs = store.vocab_packs(&language).await?;
    packs.sort_by_key(|p| p.id);
    let states_vec = store.lexeme_states(learner_id).await?;
    let states = lexeme_state_map(&states_vec);
    let now = Utc::now();

    Ok(packs
        .into_iter()
        .map(|p| {
            let (known, partial, percent) = progress_over(&p.lexemes, &states, cfg, now);
            PackSummary {
                id: p.id.0,
                title: p.title,
                emoji: p.emoji,
                description: p.description,
                total: p.lexemes.len(),
                known,
                partial,
                percent,
            }
        })
        .collect())
}

/// The flashcard deck for one pack: each word with its meaning and current
/// status, for the Study phase before the quiz.
pub async fn pack_lesson(
    store: &dyn Store,
    cfg: &GraphConfig,
    learner_id: LearnerId,
    pack_id: i64,
) -> Result<PackLesson> {
    let profile = store
        .get_learner(learner_id)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("learner {learner_id}")))?;
    let language = profile.target_language;

    let pack = store
        .vocab_packs(&language)
        .await?
        .into_iter()
        .find(|p| p.id.0 == pack_id)
        .ok_or_else(|| ServiceError::NotFound(format!("pack {pack_id}")))?;

    let lexemes = store.lexemes(&language).await?;
    let states_vec = store.lexeme_states(learner_id).await?;
    let now = Utc::now();
    let lex_by_id: HashMap<LexemeId, &Lexeme> = lexemes.iter().map(|l| (l.id, l)).collect();
    let id_status: HashMap<LexemeId, TokenStatus> = states_vec
        .iter()
        .map(|s| {
            let m = effective_mastery(s.mastery, s.last_seen_at, now, &cfg.mastery);
            (s.lexeme_id, mastery_to_token(m))
        })
        .collect();

    let cards: Vec<UnitWord> = pack
        .lexemes
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

    let (_, _, percent) = progress_over(&pack.lexemes, &lexeme_state_map(&states_vec), cfg, now);

    Ok(PackLesson {
        id: pack.id.0,
        title: pack.title,
        emoji: pack.emoji,
        description: pack.description,
        cards,
        percent,
    })
}

/// A multiple-choice quiz over a pack's words — the "learn new words" path.
///
/// Unlike [`review_session`] (which only revisits already-met words), this
/// quizzes the pack's vocabulary regardless of status, **weakest/unseen first**,
/// so answering it is how a brand-new word first enters the graph. Answers are
/// recorded with the same [`record_exercise`] used everywhere else.
pub async fn pack_quiz(
    store: &dyn Store,
    cfg: &GraphConfig,
    learner_id: LearnerId,
    pack_id: i64,
    limit: usize,
) -> Result<Vec<Exercise>> {
    let profile = store
        .get_learner(learner_id)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("learner {learner_id}")))?;
    let language = profile.target_language;

    let pack = store
        .vocab_packs(&language)
        .await?
        .into_iter()
        .find(|p| p.id.0 == pack_id)
        .ok_or_else(|| ServiceError::NotFound(format!("pack {pack_id}")))?;

    let lexemes = store.lexemes(&language).await?;
    let states = store.lexeme_states(learner_id).await?;
    let now = Utc::now();
    let lex_by_id: HashMap<LexemeId, &Lexeme> = lexemes.iter().map(|l| (l.id, l)).collect();
    let confidence_of = |id: LexemeId| -> f32 {
        states
            .iter()
            .find(|s| s.lexeme_id == id)
            .map(|s| effective_mastery(s.mastery, s.last_seen_at, now, &cfg.mastery).confidence())
            .unwrap_or(0.0)
    };

    // Pack words that can be quizzed (need a meaning), weakest/unseen first.
    let mut candidates: Vec<&Lexeme> = pack
        .lexemes
        .iter()
        .filter_map(|id| lex_by_id.get(id).copied())
        .filter(|l| l.gloss.is_some())
        .collect();
    candidates.sort_by(|a, b| {
        confidence_of(a.id)
            .partial_cmp(&confidence_of(b.id))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.truncate(limit);

    let glossed: Vec<&Lexeme> = lexemes.iter().filter(|l| l.gloss.is_some()).collect();
    let mut rng = rand::rng();
    Ok(candidates
        .into_iter()
        .map(|lex| build_exercise(lex, pick_kind(confidence_of(lex.id), &mut rng), &glossed, &mut rng))
        .collect())
}

// --- user-authored decks (custom flashcards) -----------------------------

/// First id handed to a user-created deck — a reserved range above any seeded id.
const DECK_ID_BASE: i64 = 1_000_000_000;

/// A deck as shown in the list: progress over its words.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeckSummary {
    pub id: i64,
    pub title: String,
    pub emoji: String,
    pub total: usize,
    pub known: usize,
    pub partial: usize,
    pub percent: u32,
}

/// Find a deck by id, asserting it belongs to this learner.
async fn owned_deck(store: &dyn Store, learner_id: LearnerId, deck_id: i64) -> Result<Deck> {
    store
        .decks(learner_id)
        .await?
        .into_iter()
        .find(|d| d.id.0 == deck_id)
        .ok_or_else(|| ServiceError::NotFound(format!("deck {deck_id}")))
}

/// Every deck the learner owns (in their active language) with progress.
pub async fn list_decks(
    store: &dyn Store,
    cfg: &GraphConfig,
    learner_id: LearnerId,
) -> Result<Vec<DeckSummary>> {
    let profile = store
        .get_learner(learner_id)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("learner {learner_id}")))?;
    let language = profile.target_language;

    let mut decks = store.decks(learner_id).await?;
    decks.retain(|d| d.language == language);
    decks.sort_by_key(|d| d.id);
    let states_vec = store.lexeme_states(learner_id).await?;
    let states = lexeme_state_map(&states_vec);
    let now = Utc::now();

    Ok(decks
        .into_iter()
        .map(|d| {
            let (known, partial, percent) = progress_over(&d.lexemes, &states, cfg, now);
            DeckSummary {
                id: d.id.0,
                title: d.title,
                emoji: d.emoji,
                total: d.lexemes.len(),
                known,
                partial,
                percent,
            }
        })
        .collect())
}

/// Create a new empty deck in the learner's active language.
pub async fn create_deck(
    store: &dyn Store,
    learner_id: LearnerId,
    title: String,
    emoji: String,
) -> Result<DeckSummary> {
    let profile = store
        .get_learner(learner_id)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("learner {learner_id}")))?;
    let language = profile.target_language;

    let existing = store.decks(learner_id).await?;
    let next = existing
        .iter()
        .map(|d| d.id.0)
        .max()
        .unwrap_or(DECK_ID_BASE - 1)
        + 1;
    let emoji = if emoji.trim().is_empty() { "📒".to_string() } else { emoji };
    let deck = Deck {
        id: DeckId(next),
        learner_id,
        language,
        title: title.trim().to_string(),
        emoji,
        lexemes: Vec::new(),
    };
    store.upsert_deck(&deck).await?;
    Ok(DeckSummary {
        id: deck.id.0,
        title: deck.title,
        emoji: deck.emoji,
        total: 0,
        known: 0,
        partial: 0,
        percent: 0,
    })
}

/// Delete a deck and the user words that belonged only to it.
pub async fn delete_deck(store: &dyn Store, learner_id: LearnerId, deck_id: i64) -> Result<()> {
    let deck = owned_deck(store, learner_id, deck_id).await?;
    store.delete_user_lexemes(&deck.lexemes).await?;
    store.delete_deck(deck.id).await?;
    Ok(())
}

/// Add a word (term + meaning) to a deck, minting a user lexeme for it.
pub async fn add_deck_word(
    store: &dyn Store,
    learner_id: LearnerId,
    deck_id: i64,
    lemma: String,
    gloss: String,
) -> Result<()> {
    let mut deck = owned_deck(store, learner_id, deck_id).await?;
    let lemma = lemma.trim().to_string();
    if lemma.is_empty() {
        return Err(ServiceError::NotFound("a word is required".into()));
    }
    let gloss = gloss.trim().to_string();
    let id = LexemeId(store.reserve_user_lexeme_id().await?);
    let lex = Lexeme {
        id,
        language: deck.language.clone(),
        lemma,
        pos: PartOfSpeech::Other,
        frequency_rank: 0, // user words carry no frequency — kept out of selection
        gloss: (!gloss.is_empty()).then_some(gloss),
    };
    store.upsert_user_lexemes(&[lex]).await?;
    deck.lexemes.push(id);
    store.upsert_deck(&deck).await?;
    Ok(())
}

/// Remove a word from a deck and delete its user lexeme.
pub async fn remove_deck_word(
    store: &dyn Store,
    learner_id: LearnerId,
    deck_id: i64,
    lexeme_id: i64,
) -> Result<()> {
    let mut deck = owned_deck(store, learner_id, deck_id).await?;
    deck.lexemes.retain(|id| id.0 != lexeme_id);
    store.upsert_deck(&deck).await?;
    store.delete_user_lexemes(&[LexemeId(lexeme_id)]).await?;
    Ok(())
}

/// A deck's flashcard deck (words + status) — serves both Study and the editor.
pub async fn deck_lesson(
    store: &dyn Store,
    cfg: &GraphConfig,
    learner_id: LearnerId,
    deck_id: i64,
) -> Result<PackLesson> {
    let deck = owned_deck(store, learner_id, deck_id).await?;

    let user_lexemes = store.user_lexemes(&deck.language).await?;
    let states_vec = store.lexeme_states(learner_id).await?;
    let now = Utc::now();
    let lex_by_id: HashMap<LexemeId, &Lexeme> = user_lexemes.iter().map(|l| (l.id, l)).collect();
    let id_status: HashMap<LexemeId, TokenStatus> = states_vec
        .iter()
        .map(|s| {
            let m = effective_mastery(s.mastery, s.last_seen_at, now, &cfg.mastery);
            (s.lexeme_id, mastery_to_token(m))
        })
        .collect();

    let cards: Vec<UnitWord> = deck
        .lexemes
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

    let (_, _, percent) = progress_over(&deck.lexemes, &lexeme_state_map(&states_vec), cfg, now);
    let n = cards.len();
    Ok(PackLesson {
        id: deck.id.0,
        title: deck.title,
        emoji: deck.emoji,
        description: format!("{n} word{}", if n == 1 { "" } else { "s" }),
        cards,
        percent,
    })
}

/// A multiple-choice quiz over a deck's words — the same "learn it" path as
/// packs, but over the learner's own vocabulary. Distractors are drawn from the
/// deck plus the seeded inventory of the same language, so options stay plausible.
pub async fn deck_quiz(
    store: &dyn Store,
    cfg: &GraphConfig,
    learner_id: LearnerId,
    deck_id: i64,
    limit: usize,
) -> Result<Vec<Exercise>> {
    let deck = owned_deck(store, learner_id, deck_id).await?;

    let user_lexemes = store.user_lexemes(&deck.language).await?;
    let states = store.lexeme_states(learner_id).await?;
    let now = Utc::now();
    let lex_by_id: HashMap<LexemeId, &Lexeme> = user_lexemes.iter().map(|l| (l.id, l)).collect();
    let confidence_of = |id: LexemeId| -> f32 {
        states
            .iter()
            .find(|s| s.lexeme_id == id)
            .map(|s| effective_mastery(s.mastery, s.last_seen_at, now, &cfg.mastery).confidence())
            .unwrap_or(0.0)
    };

    let mut candidates: Vec<&Lexeme> = deck
        .lexemes
        .iter()
        .filter_map(|id| lex_by_id.get(id).copied())
        .filter(|l| l.gloss.is_some())
        .collect();
    candidates.sort_by(|a, b| {
        confidence_of(a.id)
            .partial_cmp(&confidence_of(b.id))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates.truncate(limit);

    // Distractor pool: the deck's own words + the seeded inventory, same language.
    let seeded = store.lexemes(&deck.language).await?;
    let glossed: Vec<&Lexeme> = user_lexemes
        .iter()
        .chain(seeded.iter())
        .filter(|l| l.gloss.is_some())
        .collect();
    let mut rng = rand::rng();
    Ok(candidates
        .into_iter()
        .map(|lex| build_exercise(lex, pick_kind(confidence_of(lex.id), &mut rng), &glossed, &mut rng))
        .collect())
}

// --- review / spaced-repetition quiz -------------------------------------

/// What the learner is asked to do for one exercise. Recognition kinds are
/// multiple-choice; production kinds want a typed answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExerciseKind {
    /// Show the word, pick its meaning (easiest — recognition).
    ChooseMeaning,
    /// Show the meaning, pick the word (reverse recognition).
    ChooseWord,
    /// Show the meaning, type the word (production).
    TypeAnswer,
}

/// One exercise. A superset of the old multiple-choice item: MC kinds fill
/// `options`/`answer_index`; the typed kind leaves them empty and is checked
/// against `accepts` (the frontend normalizes the same way, so a missing accent
/// isn't punished). `answer` is always the canonical correct response to reveal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Exercise {
    pub lexeme_id: i64,
    pub kind: ExerciseKind,
    /// Imperative shown above the prompt, e.g. "Type the word".
    pub instruction: String,
    /// The stimulus: a word for recognition, a meaning for production.
    pub prompt: String,
    pub pos: PartOfSpeech,
    /// Choices for multiple-choice kinds; empty for `TypeAnswer`.
    pub options: Vec<String>,
    /// Index of the correct option (unused, 0, for `TypeAnswer`).
    pub answer_index: usize,
    /// The canonical correct answer, revealed after answering.
    pub answer: String,
    /// Normalized accepted answers for `TypeAnswer` (lowercased + a
    /// diacritic-folded variant), so "cafe" matches "café".
    pub accepts: Vec<String>,
}

/// Result of answering one review question.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExerciseResult {
    pub status: TokenStatus,
    pub streak: u32,
}

/// How many learned words are available to review (for a badge/CTA).
pub async fn reviewable_count(
    store: &dyn Store,
    cfg: &GraphConfig,
    learner_id: LearnerId,
) -> Result<usize> {
    let profile = store
        .get_learner(learner_id)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("learner {learner_id}")))?;
    let mut lexemes = store.lexemes(&profile.target_language).await?;
    lexemes.extend(store.user_lexemes(&profile.target_language).await?);
    let has_gloss: HashMap<LexemeId, bool> =
        lexemes.iter().map(|l| (l.id, l.gloss.is_some())).collect();
    let now = Utc::now();
    let n = store
        .lexeme_states(learner_id)
        .await?
        .iter()
        .filter(|s| {
            has_gloss.get(&s.lexeme_id).copied().unwrap_or(false)
                && !effective_mastery(s.mastery, s.last_seen_at, now, &cfg.mastery).is_unknown()
        })
        .count();
    Ok(n)
}

/// Build a spaced-repetition review session: the learner's already-met words,
/// **weakest first** so the decay model drives the schedule (words you haven't
/// seen in a while have decayed and surface first). Each item is a
/// multiple-choice question (word → meaning) with distractor glosses.
pub async fn review_session(
    store: &dyn Store,
    cfg: &GraphConfig,
    learner_id: LearnerId,
    limit: usize,
) -> Result<Vec<Exercise>> {
    let profile = store
        .get_learner(learner_id)
        .await?
        .ok_or_else(|| ServiceError::NotFound(format!("learner {learner_id}")))?;
    let language = profile.target_language;

    let mut lexemes = store.lexemes(&language).await?;
    lexemes.extend(store.user_lexemes(&language).await?);
    let states = store.lexeme_states(learner_id).await?;
    let now = Utc::now();
    let lex_by_id: HashMap<LexemeId, &Lexeme> = lexemes.iter().map(|l| (l.id, l)).collect();

    let mut candidates: Vec<(&Lexeme, f32)> = states
        .iter()
        .filter_map(|s| {
            let lex = lex_by_id.get(&s.lexeme_id)?;
            lex.gloss.as_ref()?; // can only quiz words that have a meaning
            let eff = effective_mastery(s.mastery, s.last_seen_at, now, &cfg.mastery);
            if eff.is_unknown() {
                return None;
            }
            Some((*lex, eff.confidence()))
        })
        .collect();
    candidates.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    candidates.truncate(limit);

    let glossed: Vec<&Lexeme> = lexemes.iter().filter(|l| l.gloss.is_some()).collect();
    let mut rng = rand::rng();
    let items = candidates
        .into_iter()
        .map(|(lex, conf)| build_exercise(lex, pick_kind(conf, &mut rng), &glossed, &mut rng))
        .collect();
    Ok(items)
}

/// Fold common Spanish/French/German diacritics to ASCII, so typed answers are
/// matched leniently ("cafe" == "café", "uben" == "üben").
fn fold_diacritics(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ä' | 'ã' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            other => other,
        })
        .collect()
}

/// Choose an exercise kind by how well the learner knows the word: recognition
/// while it's weak, production once it's strong — you produce what you can
/// already recognize. A little randomness keeps a session varied.
fn pick_kind(confidence: f32, rng: &mut impl Rng) -> ExerciseKind {
    let r: f32 = rng.random();
    if confidence < 0.34 {
        if r < 0.8 { ExerciseKind::ChooseMeaning } else { ExerciseKind::ChooseWord }
    } else if confidence < 0.7 {
        if r < 0.45 {
            ExerciseKind::ChooseWord
        } else if r < 0.75 {
            ExerciseKind::ChooseMeaning
        } else {
            ExerciseKind::TypeAnswer
        }
    } else if r < 0.6 {
        ExerciseKind::TypeAnswer
    } else {
        ExerciseKind::ChooseWord
    }
}

/// Up to `n` distinct distractor strings from the pool, skipping the answer.
fn distractors(values: impl Iterator<Item = String>, answer: &str, n: usize) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for v in values {
        if v != answer && !out.contains(&v) {
            out.push(v);
            if out.len() == n {
                break;
            }
        }
    }
    out
}

/// Build one exercise of the requested kind for `target`, drawing distractors
/// from `glossed` (other words that have a meaning).
fn build_exercise(
    target: &Lexeme,
    kind: ExerciseKind,
    glossed: &[&Lexeme],
    rng: &mut impl Rng,
) -> Exercise {
    let lemma = target.lemma.clone();
    let gloss = target.gloss.clone().unwrap_or_default();
    let mut pool: Vec<&Lexeme> = glossed.iter().copied().filter(|l| l.id != target.id).collect();
    pool.shuffle(rng);

    let mut mc = |prompt: String,
                  answer: String,
                  mut options: Vec<String>,
                  instruction: &str|
     -> Exercise {
        options.push(answer.clone());
        options.shuffle(rng);
        let answer_index = options.iter().position(|o| o == &answer).unwrap_or(0);
        Exercise {
            lexeme_id: target.id.0,
            kind,
            instruction: instruction.into(),
            prompt,
            pos: target.pos,
            options,
            answer_index,
            answer,
            accepts: Vec::new(),
        }
    };

    match kind {
        ExerciseKind::ChooseMeaning => {
            let opts = distractors(pool.iter().filter_map(|l| l.gloss.clone()), &gloss, 3);
            mc(lemma, gloss, opts, "Pick the meaning")
        }
        ExerciseKind::ChooseWord => {
            let opts = distractors(pool.iter().map(|l| l.lemma.clone()), &lemma, 3);
            mc(gloss, lemma, opts, "Pick the word")
        }
        ExerciseKind::TypeAnswer => {
            let n = lemma.trim().to_lowercase();
            let folded = fold_diacritics(&n);
            let accepts = if folded == n { vec![n] } else { vec![n, folded] };
            Exercise {
                lexeme_id: target.id.0,
                kind,
                instruction: "Type the word".into(),
                prompt: gloss,
                pos: target.pos,
                options: Vec::new(),
                answer_index: 0,
                answer: lemma,
                accepts,
            }
        }
    }
}

/// Record a quiz answer: folds an exercise result into mastery (correct boosts,
/// incorrect penalizes — spec §5 `ExerciseAnswered`) and logs the event.
pub async fn record_exercise(
    store: &dyn Store,
    cfg: &GraphConfig,
    learner_id: LearnerId,
    lexeme_id: i64,
    correct: bool,
) -> Result<ExerciseResult> {
    let id = LexemeId(lexeme_id);
    let now = Utc::now();
    let current = store
        .lexeme_states(learner_id)
        .await?
        .into_iter()
        .find(|s| s.lexeme_id == id)
        .unwrap_or_else(|| LexemeState::unseen(id));
    let updated = apply_lexeme_exercise(current, correct, now, &cfg.mastery);
    store.upsert_lexeme_states(learner_id, &[updated.clone()]).await?;
    store
        .append_event(
            learner_id,
            &glossa_core::LearningEvent::ExerciseAnswered {
                lexeme_id: id,
                correct,
            },
        )
        .await?;

    let streak = compute_streak(&store.activity_dates(learner_id).await?, now);
    let status = mastery_to_token(effective_mastery(
        updated.mastery,
        updated.last_seen_at,
        now,
        &cfg.mastery,
    ));
    Ok(ExerciseResult { status, streak })
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
/// Surface forms are resolved to a lexeme via `form_index` (from `glossa-lemma`),
/// so conjugations and plurals match their base word.
fn tokenize(
    text: &str,
    new_ids: &HashSet<LexemeId>,
    form_index: &HashMap<String, LexemeId>,
    id_status: &HashMap<LexemeId, TokenStatus>,
    id_gloss: &HashMap<LexemeId, Option<String>>,
    id_lemma: &HashMap<LexemeId, String>,
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
                lemma: None,
            });
            continue;
        }
        word_count += 1;
        let norm = text.to_lowercase();
        let lexeme_id = form_index.get(&norm).copied();
        let status = match lexeme_id {
            Some(id) if new_ids.contains(&id) => TokenStatus::New,
            Some(id) => id_status.get(&id).copied().unwrap_or(TokenStatus::Unknown),
            None => TokenStatus::Unknown,
        };
        if matches!(status, TokenStatus::Known | TokenStatus::Partial) {
            known_like += 1;
        }
        let gloss = lexeme_id.and_then(|id| id_gloss.get(&id).cloned().flatten());
        // Surface the dictionary form only when it isn't what's shown (so the
        // UI can say "soy → ser" but stays quiet for words already in lemma form).
        let lemma = lexeme_id
            .and_then(|id| id_lemma.get(&id))
            .filter(|l| l.to_lowercase() != norm)
            .cloned();
        tokens.push(Token {
            text,
            is_word: true,
            status: Some(status),
            lexeme_id,
            gloss,
            lemma,
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
                level: "A1.1".into(),
                objective: "Say what you eat.".into(),
                target_lexemes: vec![LexemeId(1), LexemeId(2), LexemeId(3)],
                target_pattern: None,
                reading: Some(glossa_core::ReadingPassage {
                    title: "El pan".into(),
                    text: "Yo como pan.".into(),
                    translation: "I eat bread.".into(),
                }),
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
        assert!(!lesson.objective.is_empty());
        assert_eq!(lesson.level, "A1.1");
        let reading = lesson.reading.expect("authored reading passage");
        assert!(!reading.tokens.is_empty());
        assert_eq!(reading.title, "El pan");

        // Studying the unit advances its words → unit becomes done.
        for _ in 0..6 {
            complete_unit_lesson(&store, &cfg, learner.id, 1, true)
                .await
                .unwrap();
        }
        let rm2 = roadmap(&store, &cfg, learner.id).await.unwrap();
        assert_eq!(rm2[0].state, UnitState::Done, "got {:?}", rm2[0]);
    }

    #[tokio::test]
    async fn vocab_pack_progress_lesson_and_quiz() {
        use glossa_core::{PackId, VocabPack};
        let store = FileStore::ephemeral().unwrap();
        let cfg = GraphConfig::default();
        store
            .upsert_lexemes(&[lex(1, "pan", 1), lex(2, "leche", 2), lex(3, "café", 3)])
            .await
            .unwrap();
        let learner = default_learner(&store, LanguageCode::spanish(), LanguageCode::english())
            .await
            .unwrap();
        store
            .upsert_vocab_packs(&[VocabPack {
                id: PackId(1),
                language: LanguageCode::spanish(),
                title: "Food".into(),
                emoji: "🍽️".into(),
                description: "food words".into(),
                lexemes: vec![LexemeId(1), LexemeId(2), LexemeId(3)],
            }])
            .await
            .unwrap();

        // Fresh learner → pack at 0%, all three cards present.
        let packs = vocab_packs(&store, &cfg, learner.id).await.unwrap();
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].percent, 0);
        let lesson = pack_lesson(&store, &cfg, learner.id, 1).await.unwrap();
        assert_eq!(lesson.cards.len(), 3);
        assert_eq!(lesson.emoji, "🍽️");

        // The quiz offers the pack's words (weakest/unseen first) and answering
        // one moves it into the graph — the "learn new words" path.
        let quiz = pack_quiz(&store, &cfg, learner.id, 1, 10).await.unwrap();
        assert_eq!(quiz.len(), 3, "all glossed pack words are quizzable");
        record_exercise(&store, &cfg, learner.id, quiz[0].lexeme_id, true)
            .await
            .unwrap();
        let after = vocab_packs(&store, &cfg, learner.id).await.unwrap();
        assert!(after[0].percent > 0, "answering seeds pack progress");
    }

    #[tokio::test]
    async fn custom_deck_lifecycle() {
        let store = FileStore::ephemeral().unwrap();
        let cfg = GraphConfig::default();
        // A seeded word exists so distractors have a pool to draw from.
        store.upsert_lexemes(&[lex(1, "casa", 1)]).await.unwrap();
        let learner = default_learner(&store, LanguageCode::spanish(), LanguageCode::english())
            .await
            .unwrap();

        // Create a deck and add the learner's own words.
        let deck = create_deck(&store, learner.id, "German class".into(), "🇩🇪".into())
            .await
            .unwrap();
        add_deck_word(&store, learner.id, deck.id, "perro".into(), "dog".into())
            .await
            .unwrap();
        add_deck_word(&store, learner.id, deck.id, "gato".into(), "cat".into())
            .await
            .unwrap();

        // The deck lists with two cards and lives in a reserved id range.
        let decks = list_decks(&store, &cfg, learner.id).await.unwrap();
        assert_eq!(decks.len(), 1);
        assert_eq!(decks[0].total, 2);
        assert!(decks[0].id >= 1_000_000_000, "deck id is in the reserved range");
        let lesson = deck_lesson(&store, &cfg, learner.id, deck.id).await.unwrap();
        assert_eq!(lesson.cards.len(), 2);
        assert!(lesson.cards[0].lexeme_id >= 1_000_000_000, "user lexeme id reserved");

        // Quizzing a deck word seeds its mastery via the shared exercise path.
        let quiz = deck_quiz(&store, &cfg, learner.id, deck.id, 10).await.unwrap();
        assert_eq!(quiz.len(), 2);
        record_exercise(&store, &cfg, learner.id, quiz[0].lexeme_id, true)
            .await
            .unwrap();
        let after = list_decks(&store, &cfg, learner.id).await.unwrap();
        assert!(after[0].percent > 0, "answering moves deck progress");

        // The custom word is now reviewable in the global quiz too.
        assert!(reviewable_count(&store, &cfg, learner.id).await.unwrap() >= 1);

        // Removing a word and deleting the deck cleans up its user lexemes.
        remove_deck_word(&store, learner.id, deck.id, quiz[1].lexeme_id)
            .await
            .unwrap();
        assert_eq!(
            deck_lesson(&store, &cfg, learner.id, deck.id).await.unwrap().cards.len(),
            1
        );
        delete_deck(&store, learner.id, deck.id).await.unwrap();
        assert!(list_decks(&store, &cfg, learner.id).await.unwrap().is_empty());
        assert!(store.user_lexemes(&LanguageCode::spanish()).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn review_session_quizzes_known_words_only() {
        let store = FileStore::ephemeral().unwrap();
        store
            .upsert_lexemes(&[
                lex(1, "yo", 1),
                lex(2, "comer", 2),
                lex(3, "pizza", 3),
                lex(4, "agua", 4),
                lex(5, "casa", 5),
            ])
            .await
            .unwrap();
        let learner = default_learner(&store, LanguageCode::spanish(), LanguageCode::english())
            .await
            .unwrap();
        let cfg = GraphConfig::default();

        // Nothing learned yet → nothing to review.
        assert!(review_session(&store, &cfg, learner.id, 10)
            .await
            .unwrap()
            .is_empty());

        set_lexeme_status(&store, learner.id, LexemeId(1), MasteryState::Known)
            .await
            .unwrap();
        let items = review_session(&store, &cfg, learner.id, 10).await.unwrap();
        assert_eq!(items.len(), 1);
        let it = &items[0];
        assert_eq!(it.lexeme_id, 1);
        // Whatever kind was chosen, the exercise must be internally consistent.
        match it.kind {
            ExerciseKind::ChooseMeaning => {
                assert_eq!(it.prompt, "yo"); // shown the word
                assert_eq!(it.answer, "yo-en"); // answer is its meaning
                assert_eq!(it.options[it.answer_index], "yo-en");
            }
            ExerciseKind::ChooseWord => {
                assert_eq!(it.prompt, "yo-en"); // shown the meaning
                assert_eq!(it.answer, "yo"); // answer is the word
                assert_eq!(it.options[it.answer_index], "yo");
            }
            ExerciseKind::TypeAnswer => {
                assert_eq!(it.prompt, "yo-en");
                assert_eq!(it.answer, "yo");
                assert!(it.options.is_empty());
                assert!(it.accepts.contains(&"yo".to_string()));
            }
        }

        record_exercise(&store, &cfg, learner.id, 1, true)
            .await
            .unwrap();
    }

    #[test]
    fn exercise_kinds_are_well_formed() {
        let mut rng = rand::rng();
        let target = lex(1, "café", 1); // gloss "café-en", has a diacritic
        let other = lex(2, "agua", 2);
        let glossed: Vec<&Lexeme> = vec![&target, &other];

        // Recognition: word shown, meaning is the answer among the options.
        let m = build_exercise(&target, ExerciseKind::ChooseMeaning, &glossed, &mut rng);
        assert_eq!(m.prompt, "café");
        assert_eq!(m.options[m.answer_index], m.answer);

        // Reverse recognition: meaning shown, word is the answer.
        let w = build_exercise(&target, ExerciseKind::ChooseWord, &glossed, &mut rng);
        assert_eq!(w.answer, "café");
        assert_eq!(w.options[w.answer_index], "café");

        // Production: typed, and accent-folded so "cafe" is accepted.
        let t = build_exercise(&target, ExerciseKind::TypeAnswer, &glossed, &mut rng);
        assert!(t.options.is_empty());
        assert_eq!(t.answer, "café");
        assert!(t.accepts.contains(&"cafe".to_string()));
        assert!(t.accepts.contains(&"café".to_string()));
    }
}
