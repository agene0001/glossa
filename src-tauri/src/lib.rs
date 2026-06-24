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
    language: LanguageCode,
    /// "anthropic" if a real API key is configured, else "mock".
    generator_kind: &'static str,
}

#[derive(Serialize)]
struct BackendStatus {
    generator: &'static str,
    language: String,
    learner_id: String,
}

/// Lightweight status for the UI banner (live vs. offline mock).
#[tauri::command]
fn backend_status(state: State<'_, AppState>) -> BackendStatus {
    BackendStatus {
        generator: state.generator_kind,
        language: state.language.as_str().to_string(),
        learner_id: state.learner_id.to_string(),
    }
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
async fn graph_overview(state: State<'_, AppState>) -> Result<GraphOverview, String> {
    let (store, cfg, learner) = (state.store.clone(), state.cfg.clone(), state.learner_id);
    service::graph_overview(store.as_ref(), &cfg, learner, 15)
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
            tauri::async_runtime::block_on(seed::seed_if_empty(store.as_ref(), &language))?;
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
                language,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running Glossa");
}
