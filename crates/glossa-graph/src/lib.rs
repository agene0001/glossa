//! `glossa-graph` — the component everything else depends on (spec §2.6).
//!
//! Two responsibilities, both **pure functions** (no I/O, no global state, keyed
//! entirely by data passed in — spec §9):
//!
//! 1. **Mastery transitions** ([`mastery`]) — how exposures and exercise results
//!    move an item `Unknown → Partial → Known`, with recency decay.
//! 2. **Selection** ([`select`]) — `next_best_content`, the frequency-weighted
//!    choice of what to teach next, and `overview`, the Review-view summary.
//!
//! The service layer loads state from storage, calls these functions, and writes
//! results back. The graph itself never touches a database.

pub mod mastery;
pub mod select;

pub use mastery::MasteryConfig;
pub use select::{GraphOverview, NextContentConfig, QueuedItem};

use serde::{Deserialize, Serialize};

/// Tunables bundled together so the service can pass one config object.
///
/// The mastery transition function is explicitly an open question (spec §11.3):
/// start with this heuristic, then tune the numbers as real usage data arrives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphConfig {
    pub mastery: MasteryConfig,
    pub next_content: NextContentConfig,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            mastery: MasteryConfig::default(),
            next_content: NextContentConfig::default(),
        }
    }
}
