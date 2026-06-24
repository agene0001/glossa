//! `glossa-voice` — the speech trait boundary (spec §5, Phase 3).
//!
//! No provider is implemented in V1. Defining the traits now means adding a real
//! STT/TTS provider later (e.g. in Phase 3) is a new impl behind this boundary
//! and does **not** touch `glossa-conversation` or anything upstream.

/// Errors a speech provider may raise.
#[derive(Debug, thiserror::Error)]
pub enum VoiceError {
    #[error("voice provider not configured")]
    NotConfigured,
    #[error("speech provider error: {0}")]
    Provider(String),
}

pub type Result<T> = std::result::Result<T, VoiceError>;

/// Transcribe spoken audio to text.
pub trait SpeechToText: Send + Sync {
    fn transcribe(&self, audio: &[u8]) -> Result<String>;
}

/// Synthesize text into spoken audio.
pub trait TextToSpeech: Send + Sync {
    fn synthesize(&self, text: &str) -> Result<Vec<u8>>;
}
