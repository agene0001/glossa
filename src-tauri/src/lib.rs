//! Glossa Tauri shell.
//!
//! Thin adapter over `glossa-service`: builds the app state (store + content
//! generator + the single V1 learner), seeds the inventory on first run, and
//! exposes the service functions as IPC commands. All real logic lives in the
//! domain crates — a future `glossa-api` (Axum) would wrap the same functions
//! (spec §4.3, §9), so keep this file thin.

mod seed;

use std::sync::Arc;

use serde::Serialize;
use tauri::{Manager, State};

use glossa_content::{AnthropicContentGenerator, ContentGenerator, MockContentGenerator};
use glossa_core::{ContentResponse, LanguageCode, LearnerId, LexemeId, MasteryState};
use glossa_graph::{GraphConfig, GraphOverview};
use glossa_service as service;
use glossa_storage::{FileStore, Store};

/// Process-wide state, injected into every command via Tauri's managed state.
struct AppState {
    store: Arc<dyn Store>,
    generator: Arc<dyn ContentGenerator>,
    cfg: GraphConfig,
    learner_id: LearnerId,
    /// "anthropic" if a real API key is configured, else "mock".
    generator_kind: &'static str,
}

#[derive(Serialize)]
struct BackendStatus {
    generator: &'static str,
    language: String,
    streak: u32,
    learner_id: String,
}

#[derive(Serialize)]
struct LanguageOption {
    code: &'static str,
    name: &'static str,
}

/// Status for the UI: engine (live/mock), current target language, streak.
#[tauri::command]
async fn backend_status(state: State<'_, AppState>) -> Result<BackendStatus, String> {
    let (store, learner, kind) = (state.store.clone(), state.learner_id, state.generator_kind);
    let language = store
        .get_learner(learner)
        .await
        .map_err(|e| e.to_string())?
        .map(|p| p.target_language.as_str().to_string())
        .unwrap_or_default();
    let streak = service::streak(store.as_ref(), learner)
        .await
        .map_err(|e| e.to_string())?;
    Ok(BackendStatus {
        generator: kind,
        language,
        streak,
        learner_id: learner.to_string(),
    })
}

/// Languages that have seeded content (for the picker).
#[tauri::command]
fn available_languages() -> Vec<LanguageOption> {
    vec![
        LanguageOption { code: "es", name: "Spanish" },
        LanguageOption { code: "fr", name: "French" },
    ]
}

#[tauri::command]
async fn set_target_language(state: State<'_, AppState>, code: String) -> Result<(), String> {
    let (store, learner) = (state.store.clone(), state.learner_id);
    service::set_target_language(store.as_ref(), learner, &code)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn next_content(state: State<'_, AppState>) -> Result<ContentResponse, String> {
    let (store, generator, cfg, learner) = (
        state.store.clone(),
        state.generator.clone(),
        state.cfg.clone(),
        state.learner_id,
    );
    service::next_content(store.as_ref(), generator.as_ref(), &cfg, learner)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn record_story_read(
    state: State<'_, AppState>,
    story_id: String,
    understood: bool,
) -> Result<(), String> {
    let id = uuid::Uuid::parse_str(&story_id).map_err(|e| e.to_string())?;
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::record_story_read(store.as_ref(), &cfg, learner, id, understood)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn graph_overview(
    state: State<'_, AppState>,
    queue_limit: Option<usize>,
) -> Result<GraphOverview, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::graph_overview(store.as_ref(), &cfg, learner, queue_limit.unwrap_or(15))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn set_lexeme_status(
    state: State<'_, AppState>,
    lexeme_id: i64,
    status: String,
) -> Result<(), String> {
    let mastery = match status.as_str() {
        "known" => MasteryState::Known,
        "unknown" => MasteryState::Unknown,
        "partial" => MasteryState::Partial { confidence: 0.5 },
        other => return Err(format!("unknown status: {other}")),
    };
    let (store, learner) = (state.store.clone(), state.learner_id);
    service::set_lexeme_status(store.as_ref(), learner, LexemeId(lexeme_id), mastery)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn roadmap(state: State<'_, AppState>) -> Result<Vec<service::RoadmapUnit>, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::roadmap(store.as_ref(), &cfg, learner)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn unit_lesson(
    state: State<'_, AppState>,
    unit_id: i64,
) -> Result<service::UnitLesson, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::unit_lesson(store.as_ref(), &cfg, learner, unit_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn complete_unit_lesson(
    state: State<'_, AppState>,
    unit_id: i64,
    understood: bool,
) -> Result<service::LessonResult, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::complete_unit_lesson(store.as_ref(), &cfg, learner, unit_id, understood)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn next_content_for_unit(
    state: State<'_, AppState>,
    unit_id: i64,
) -> Result<ContentResponse, String> {
    let (store, generator, cfg, learner) = (
        state.store.clone(),
        state.generator.clone(),
        state.cfg.clone(),
        state.learner_id,
    );
    service::next_content_for_unit(store.as_ref(), generator.as_ref(), &cfg, learner, unit_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn vocab_packs(state: State<'_, AppState>) -> Result<Vec<service::PackSummary>, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::vocab_packs(store.as_ref(), &cfg, learner)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn pack_lesson(
    state: State<'_, AppState>,
    pack_id: i64,
) -> Result<service::PackLesson, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::pack_lesson(store.as_ref(), &cfg, learner, pack_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn pack_quiz(
    state: State<'_, AppState>,
    pack_id: i64,
    limit: Option<usize>,
) -> Result<Vec<service::Exercise>, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::pack_quiz(store.as_ref(), &cfg, learner, pack_id, limit.unwrap_or(12))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn list_decks(state: State<'_, AppState>) -> Result<Vec<service::DeckSummary>, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::list_decks(store.as_ref(), &cfg, learner)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn create_deck(
    state: State<'_, AppState>,
    title: String,
    emoji: String,
) -> Result<service::DeckSummary, String> {
    let (store, learner) = (state.store.clone(), state.learner_id);
    service::create_deck(store.as_ref(), learner, title, emoji)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn delete_deck(state: State<'_, AppState>, deck_id: i64) -> Result<(), String> {
    let (store, learner) = (state.store.clone(), state.learner_id);
    service::delete_deck(store.as_ref(), learner, deck_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn add_deck_word(
    state: State<'_, AppState>,
    deck_id: i64,
    lemma: String,
    gloss: String,
) -> Result<(), String> {
    let (store, learner) = (state.store.clone(), state.learner_id);
    service::add_deck_word(store.as_ref(), learner, deck_id, lemma, gloss)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn remove_deck_word(
    state: State<'_, AppState>,
    deck_id: i64,
    lexeme_id: i64,
) -> Result<(), String> {
    let (store, learner) = (state.store.clone(), state.learner_id);
    service::remove_deck_word(store.as_ref(), learner, deck_id, lexeme_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn deck_lesson(
    state: State<'_, AppState>,
    deck_id: i64,
) -> Result<service::PackLesson, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::deck_lesson(store.as_ref(), &cfg, learner, deck_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn deck_quiz(
    state: State<'_, AppState>,
    deck_id: i64,
    limit: Option<usize>,
) -> Result<Vec<service::Exercise>, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::deck_quiz(store.as_ref(), &cfg, learner, deck_id, limit.unwrap_or(12))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn grammar_track(
    state: State<'_, AppState>,
) -> Result<Vec<service::GrammarTrackItem>, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::grammar_track(store.as_ref(), &cfg, learner)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn grammar_lesson(
    state: State<'_, AppState>,
    pattern_id: i64,
) -> Result<service::GrammarLesson, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::grammar_lesson(store.as_ref(), &cfg, learner, pattern_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn record_grammar_exercise(
    state: State<'_, AppState>,
    pattern_id: i64,
    correct: bool,
) -> Result<service::ExerciseResult, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::record_grammar_exercise(store.as_ref(), &cfg, learner, pattern_id, correct)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn review_session(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<Vec<service::Exercise>, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::review_session(store.as_ref(), &cfg, learner, limit.unwrap_or(12))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn record_exercise(
    state: State<'_, AppState>,
    lexeme_id: i64,
    correct: bool,
) -> Result<service::ExerciseResult, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::record_exercise(store.as_ref(), &cfg, learner, lexeme_id, correct)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn learner_stats(state: State<'_, AppState>) -> Result<service::Stats, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::learner_stats(store.as_ref(), &cfg, learner)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
async fn reviewable_count(state: State<'_, AppState>) -> Result<usize, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::reviewable_count(store.as_ref(), &cfg, learner)
        .await
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // File-backed store in the OS app-data dir — progress persists.
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let store: Arc<dyn Store> = Arc::new(FileStore::open(data_dir.join("glossa.json"))?);

            // V1 is single target language (Spanish) — see spec §11.1.
            let language = LanguageCode::spanish();

            // First-run seeding + resolve the single learner. These touch only
            // storage, so blocking briefly during setup is fine.
            tauri::async_runtime::block_on(seed::sync_inventory(store.as_ref()))?;
            let learner = tauri::async_runtime::block_on(service::default_learner(
                store.as_ref(),
                language.clone(),
                LanguageCode::english(),
            ))?;

            // Real generator if a key is set, otherwise the offline mock so the
            // app is fully usable with zero configuration.
            let (generator, generator_kind): (Arc<dyn ContentGenerator>, &'static str) =
                match AnthropicContentGenerator::try_from_env() {
                    Some(g) => (Arc::new(g), "anthropic"),
                    None => (Arc::new(MockContentGenerator), "mock"),
                };

            app.manage(AppState {
                store,
                generator,
                cfg: GraphConfig::default(),
                learner_id: learner.id,
                generator_kind,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            backend_status,
            next_content,
            record_story_read,
            graph_overview,
            set_lexeme_status,
            roadmap,
            unit_lesson,
            complete_unit_lesson,
            next_content_for_unit,
            vocab_packs,
            pack_lesson,
            pack_quiz,
            list_decks,
            create_deck,
            delete_deck,
            add_deck_word,
            remove_deck_word,
            deck_lesson,
            deck_quiz,
            grammar_track,
            grammar_lesson,
            record_grammar_exercise,
            available_languages,
            set_target_language,
            review_session,
            record_exercise,
            reviewable_count,
            learner_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Glossa");
}
