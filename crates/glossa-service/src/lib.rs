//! `glossa-service` — transport-agnostic orchestration (spec §4.3).
//!
//! Plain async functions over the domain crates. **No Tauri types, no HTTP
//! types.** `src-tauri` calls these from IPC commands today; a future
//! `glossa-api` (Axum) will call the exact same functions over HTTP (spec §9).
//! Keep this crate free of any transport detail.

use std::collections::{HashMap, HashSet};

use chrono::Utc;
use uuid::Uuid;

use glossa_content::ContentGenerator;
use glossa_core::{
    ContentResponse, LanguageCode, LearnerId, LearnerProfile, Lexeme, LexemeId, LexemeState,
    MasteryState, Token, TokenStatus, WordInfo,
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
    let grammar_pattern_id = request.grammar_target.as_ref().map(|g| g.id);

    let generated = generator.generate(&request).await?;

    // --- resolve lemmas → ids, build display tokens -------------------------
    let lex_by_lemma: HashMap<String, &Lexeme> = lexemes
        .iter()
        .map(|l| (l.lemma.to_lowercase(), l))
        .collect();
    let lemma_to_id: HashMap<String, LexemeId> =
        lex_by_lemma.iter().map(|(k, l)| (k.clone(), l.id)).collect();

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

    let (tokens, known_ratio) = tokenize(&generated.text, &new_set, &lemma_to_id, &id_status);

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
        kind: request.kind,
        text: generated.text,
        tokens,
        new_words,
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
        tokens.push(Token {
            text,
            is_word: true,
            status: Some(status),
            lexeme_id,
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
    use glossa_core::PartOfSpeech;
    use glossa_storage::FileStore;

    fn lex(id: i64, lemma: &str, rank: u32) -> Lexeme {
        Lexeme {
            id: LexemeId(id),
            language: LanguageCode::spanish(),
            lemma: lemma.into(),
            pos: PartOfSpeech::Noun,
            frequency_rank: rank,
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
}
