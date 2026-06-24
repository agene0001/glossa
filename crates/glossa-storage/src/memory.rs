//! [`FileStore`]: an in-memory model persisted to a JSON file.
//!
//! Single-user, single-process. Writes take a lock, mutate, then atomically
//! rewrite the file (temp + rename). That's plenty for V1's volumes and keeps
//! the on-disk format human-inspectable while you tune the mastery model.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use glossa_core::{
    GrammarPattern, GrammarState, LanguageCode, LearnerId, LearnerProfile, LearningEvent, Lexeme,
    LexemeId, LexemeState, PatternId,
};

use crate::{Result, StorageError, Store, StoredStory};

/// One row of the append-only event log.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventRecord {
    id: Uuid,
    learner_id: LearnerId,
    event: LearningEvent,
    created_at: DateTime<Utc>,
}

/// The entire database, serialized as one JSON document.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Db {
    learners: Vec<LearnerProfile>,
    lexemes: Vec<Lexeme>,
    grammar_patterns: Vec<GrammarPattern>,
    /// learner -> (lexeme -> state)
    lexeme_states: HashMap<LearnerId, HashMap<LexemeId, LexemeState>>,
    /// learner -> (pattern -> state)
    grammar_states: HashMap<LearnerId, HashMap<PatternId, GrammarState>>,
    events: Vec<EventRecord>,
    stories: HashMap<Uuid, StoredStory>,
}

/// File-backed implementation of [`Store`].
pub struct FileStore {
    path: PathBuf,
    inner: RwLock<Db>,
}

impl FileStore {
    /// Open the store at `path`, loading existing data or starting empty.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let db = if path.exists() {
            let bytes = std::fs::read(&path)?;
            serde_json::from_slice(&bytes)?
        } else {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            Db::default()
        };
        Ok(Self {
            path,
            inner: RwLock::new(db),
        })
    }

    /// An ephemeral store backed by a temp file — handy for tests.
    pub fn ephemeral() -> Result<Self> {
        let path = std::env::temp_dir().join(format!("glossa-{}.json", Uuid::new_v4()));
        Self::open(path)
    }

    /// Atomically persist `db` to disk (write temp, then rename).
    fn persist(&self, db: &Db) -> Result<()> {
        let tmp = self.path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(db)?;
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    fn read(&self) -> std::sync::RwLockReadGuard<'_, Db> {
        self.inner.read().expect("storage lock poisoned")
    }

    fn write(&self) -> std::sync::RwLockWriteGuard<'_, Db> {
        self.inner.write().expect("storage lock poisoned")
    }
}

#[async_trait]
impl Store for FileStore {
    async fn get_or_create_default_learner(
        &self,
        target_language: LanguageCode,
        native_language: LanguageCode,
    ) -> Result<LearnerProfile> {
        let mut db = self.write();
        if let Some(existing) = db.learners.first() {
            return Ok(existing.clone());
        }
        let learner = LearnerProfile {
            id: LearnerId::new(),
            target_language,
            native_language,
        };
        db.learners.push(learner.clone());
        self.persist(&db)?;
        Ok(learner)
    }

    async fn get_learner(&self, id: LearnerId) -> Result<Option<LearnerProfile>> {
        Ok(self.read().learners.iter().find(|l| l.id == id).cloned())
    }

    async fn lexemes(&self, language: &LanguageCode) -> Result<Vec<Lexeme>> {
        Ok(self
            .read()
            .lexemes
            .iter()
            .filter(|l| &l.language == language)
            .cloned()
            .collect())
    }

    async fn grammar_patterns(&self, language: &LanguageCode) -> Result<Vec<GrammarPattern>> {
        Ok(self
            .read()
            .grammar_patterns
            .iter()
            .filter(|p| &p.language == language)
            .cloned()
            .collect())
    }

    async fn lexeme_count(&self, language: &LanguageCode) -> Result<usize> {
        Ok(self
            .read()
            .lexemes
            .iter()
            .filter(|l| &l.language == language)
            .count())
    }

    async fn upsert_lexemes(&self, lexemes: &[Lexeme]) -> Result<()> {
        let mut db = self.write();
        for lex in lexemes {
            if let Some(slot) = db.lexemes.iter_mut().find(|l| l.id == lex.id) {
                *slot = lex.clone();
            } else {
                db.lexemes.push(lex.clone());
            }
        }
        self.persist(&db)?;
        Ok(())
    }

    async fn upsert_grammar_patterns(&self, patterns: &[GrammarPattern]) -> Result<()> {
        let mut db = self.write();
        for pat in patterns {
            if let Some(slot) = db.grammar_patterns.iter_mut().find(|p| p.id == pat.id) {
                *slot = pat.clone();
            } else {
                db.grammar_patterns.push(pat.clone());
            }
        }
        self.persist(&db)?;
        Ok(())
    }

    async fn lexeme_states(&self, learner: LearnerId) -> Result<Vec<LexemeState>> {
        Ok(self
            .read()
            .lexeme_states
            .get(&learner)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default())
    }

    async fn grammar_states(&self, learner: LearnerId) -> Result<Vec<GrammarState>> {
        Ok(self
            .read()
            .grammar_states
            .get(&learner)
            .map(|m| m.values().cloned().collect())
            .unwrap_or_default())
    }

    async fn upsert_lexeme_states(&self, learner: LearnerId, states: &[LexemeState]) -> Result<()> {
        let mut db = self.write();
        let entry = db.lexeme_states.entry(learner).or_default();
        for st in states {
            entry.insert(st.lexeme_id, st.clone());
        }
        self.persist(&db)?;
        Ok(())
    }

    async fn upsert_grammar_states(
        &self,
        learner: LearnerId,
        states: &[GrammarState],
    ) -> Result<()> {
        let mut db = self.write();
        let entry = db.grammar_states.entry(learner).or_default();
        for st in states {
            entry.insert(st.pattern_id, st.clone());
        }
        self.persist(&db)?;
        Ok(())
    }

    async fn append_event(&self, learner: LearnerId, event: &LearningEvent) -> Result<()> {
        let mut db = self.write();
        db.events.push(EventRecord {
            id: Uuid::new_v4(),
            learner_id: learner,
            event: event.clone(),
            created_at: Utc::now(),
        });
        self.persist(&db)?;
        Ok(())
    }

    async fn save_story(&self, story: &StoredStory) -> Result<()> {
        let mut db = self.write();
        db.stories.insert(story.id, story.clone());
        self.persist(&db)?;
        Ok(())
    }

    async fn get_story(&self, id: Uuid) -> Result<Option<StoredStory>> {
        Ok(self.read().stories.get(&id).cloned())
    }
}

// `NotFound` is part of the public error surface; reference it so a strict
// build doesn't warn while the file store happens to return `Ok(None)` instead.
#[allow(dead_code)]
fn _assert_errors(e: StorageError) -> bool {
    matches!(e, StorageError::NotFound(_))
}

#[cfg(test)]
mod tests {
    use super::*;
    use glossa_core::{MasteryState, PartOfSpeech};

    fn es() -> LanguageCode {
        LanguageCode::spanish()
    }

    #[tokio::test]
    async fn persists_and_reloads_across_reopen() {
        let path = std::env::temp_dir().join(format!("glossa-test-{}.json", Uuid::new_v4()));

        let learner_id;
        {
            let store = FileStore::open(&path).unwrap();
            let learner = store
                .get_or_create_default_learner(es(), LanguageCode::english())
                .await
                .unwrap();
            learner_id = learner.id;

            store
                .upsert_lexemes(&[Lexeme {
                    id: LexemeId(1),
                    language: es(),
                    lemma: "comer".into(),
                    pos: PartOfSpeech::Verb,
                    frequency_rank: 1,
                }])
                .await
                .unwrap();

            store
                .upsert_lexeme_states(
                    learner_id,
                    &[LexemeState {
                        lexeme_id: LexemeId(1),
                        mastery: MasteryState::Partial { confidence: 0.5 },
                        exposure_count: 2,
                        last_seen_at: Some(Utc::now()),
                    }],
                )
                .await
                .unwrap();
        }

        // Reopen from disk — state must survive.
        let store = FileStore::open(&path).unwrap();
        let again = store
            .get_or_create_default_learner(es(), LanguageCode::english())
            .await
            .unwrap();
        assert_eq!(again.id, learner_id, "default learner should be stable");
        assert_eq!(store.lexeme_count(&es()).await.unwrap(), 1);
        let states = store.lexeme_states(learner_id).await.unwrap();
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].exposure_count, 2);

        let _ = std::fs::remove_file(&path);
    }
}
