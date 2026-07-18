//! Curated external resources — the "immersion" layer.
//!
//! Static, hand-curated links to **free** third-party material (comprehensible-
//! input video, podcasts, readers, exchange apps, tools) that provide the
//! massive real input, listening, and interaction Glossa can't replicate itself.
//! Curation, not auto-discovery — we point at known-good channels and sites.

use serde::{Deserialize, Serialize};

use crate::lang::LanguageCode;

/// One external resource (a channel, podcast, site, or tool).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternalResource {
    /// Grouping heading: "Watch", "Listen", "Read", "Speak & exchange", "Tools".
    pub category: String,
    pub title: String,
    pub description: String,
    pub url: String,
    /// A short badge, e.g. "free", "freemium", "comprehensible input".
    pub tag: String,
}

/// A language's immersion guide: its resources plus the universal tools.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceGuide {
    pub language: LanguageCode,
    pub resources: Vec<ExternalResource>,
}
