//! `glossa-conversation` — AI tutor chat engine + scenario library (Phase 2).
//!
//! V1 does **not** build the conversation loop (spec §3). The types and the
//! [`ConversationEngine`] trait are defined now so the data model and service
//! boundary are settled — when Phase 2 arrives, the same engine also backs
//! Pillar 5 (AI fallback) with no new code path (spec §2.5, §5).
//!
//! Corrections are a structured side-channel (`corrections`), separate from the
//! reply text, so the frontend renders them in a sidebar without interrupting
//! the transcript (spec §2.3, §7).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use glossa_core::{LanguageCode, LearnerId, LexemeId};

/// Goal-directed practice presets (spec §2.3). Each constrains vocabulary
/// domain and register, which also gives the graph a targeted exposure signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scenario {
    OrderingFood,
    JobInterview,
    MakingFriends,
    AirportTravel,
    BusinessMeeting,
    SmallTalk,
}

/// Who produced a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Speaker {
    Learner,
    Tutor,
}

/// A gentle correction surfaced on the side channel, never inline (spec §2.3).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Correction {
    pub original: String,
    pub suggestion: String,
    pub explanation: String,
}

/// One turn of conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationTurn {
    pub speaker: Speaker,
    pub text: String,
    pub new_lexemes: Vec<LexemeId>,
    pub corrections: Vec<Correction>,
}

/// Context handed to the engine each turn (the API is stateless, so the system
/// prompt is reconstructed per call from this — spec §5).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversationContext {
    pub learner_id: LearnerId,
    pub language: LanguageCode,
    pub scenario: Scenario,
    pub history: Vec<ConversationTurn>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConversationError {
    #[error("conversation engine is not implemented until Phase 2")]
    NotImplemented,
}

/// The tutor chat engine (Phase 2). Also the Pillar-5 AI-fallback engine.
#[async_trait]
pub trait ConversationEngine: Send + Sync {
    async fn respond(
        &self,
        context: &ConversationContext,
        learner_message: &str,
    ) -> Result<ConversationTurn, ConversationError>;
}

/// Placeholder so the crate is usable as a dependency and the trait is exercised
/// before Phase 2. Always returns [`ConversationError::NotImplemented`].
pub struct UnimplementedEngine;

#[async_trait]
impl ConversationEngine for UnimplementedEngine {
    async fn respond(
        &self,
        _context: &ConversationContext,
        _learner_message: &str,
    ) -> Result<ConversationTurn, ConversationError> {
        Err(ConversationError::NotImplemented)
    }
}

/// Reserved for Phase 2: a stable id for a conversation. Defined now so the
/// storage schema in §6 (`conversations`, `conversation_turns`) stays aligned.
pub type ConversationId = Uuid;
