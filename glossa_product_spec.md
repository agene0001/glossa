# Glossa — AI-Native Language Learning Platform
### Product Specification (v0.2)

> "glossa" is a placeholder codename (Greek: tongue/language). Swap freely.

---

## 1. Problem Statement

Existing language apps optimize for daily engagement (streaks, gamified XP) rather than actual acquisition. Users accumulate hundreds of "lesson" days and still can't hold a conversation, because:

- Vocabulary is taught in isolation (flashcard pairs) instead of in context.
- Grammar is taught as explicit rules up front instead of emerging from repeated exposure.
- There's no real model of what the learner actually knows — every learner on "Lesson 47" sees the same content regardless of their actual mastery.
- Conversation practice, the highest-value activity, is usually missing or bolted on as an afterthought.
- Real human conversation — the actual goal — is either absent or so intimidating (native speakers, full speed) that learners avoid it.

**Core thesis:** the moat isn't "has an AI tutor" — everyone will have that within a year. It's an accurate, continuously-updated model of the learner's vocabulary and grammar mastery, used to deterministically generate every piece of content — story, sentence, conversation turn, match difficulty — at exactly the right level.

---

## 2. Platform Vision & Goals

This section describes the full intended platform, including pillars that are **not** part of V1. They're specified here in detail because the data model and service boundaries need to anticipate them now, even though they're built later — see §3 for what's actually in scope per phase.

### 2.1 Pillar 1 — Comprehensible-Input Content Generation

Rather than teaching `Apple = Manzana`, the system generates sentences and short stories using vocabulary the learner already knows, plus a small, deliberate number of new words introduced through context:

```
I eat an apple.
She bought an apple.
The apple is red.
```

The target ratio is configurable but defaults to ~95% known / ~5% new per piece of content. This is the standard comprehensible-input approach associated with Stephen Krashen's work on second language acquisition: learners acquire language by understanding messages slightly above their current level, not by memorizing isolated facts about the language.

Every story, sentence set, and exercise is generated **dynamically from the learner's current knowledge graph state** (§2.6) rather than pulled from a fixed curriculum. There is no "Lesson 47." There is only "the next most useful thing for this specific learner, right now."

### 2.2 Pillar 2 — Implicit Grammar Acquisition

Most apps front-load grammar as a rule to memorize ("Today we learn the preterite tense"), which is immediately forgotten because it was never encountered in meaningful use. Glossa instead:

- Tracks grammar patterns as first-class graph nodes, exactly like vocabulary (`GrammarPattern` / `learner_grammar_state` in §6), each with its own mastery state.
- Lets content generation deliberately target a specific pattern across multiple pieces of content without naming it:

```
Yesterday I ate pizza.
Yesterday I watched TV.
Yesterday I visited my friend.
```

- Surfaces an explicit rule explanation only as **optional, opt-in support** once a pattern has recurred enough that the learner is likely to notice it ("Notice how past actions use this form...") — never as a mandatory gate before content is accessible.

### 2.3 Pillar 3 — Always-On AI Conversation Partner

A conversation engine that speaks at the learner's level, drawing on the same knowledge graph used for content generation:

```
AI:  Hola, ¿cómo estás?
You: Bien.
AI:  Excelente. ¿Qué comiste hoy?
You: Pizza.
AI:  ¿Te gusta la pizza?
```

Requirements:
- Uses known vocabulary by default, introduces new words gradually and in context, same as story generation.
- Corrects mistakes without breaking conversational flow (corrections surfaced as a separate channel, not inline interruption — see §7).
- Persists conversation history per learner so the partner has continuity across sessions, not a cold start every time.
- Supports scenario presets for goal-directed practice: ordering food, job interview, making friends, airport travel, a business meeting, casual small talk. Scenario choice constrains vocabulary domain and conversational register, which also gives the knowledge graph a more targeted exposure signal than open-ended chat.

This pillar is also what makes Pillar 5 (AI fallback) free — it's the same engine, just invoked when no human match is available.

### 2.4 Pillar 4 — Human-to-Human Conversation Practice

AI conversation, however good, doesn't fully replace real communicative stakes — genuine spontaneity, cultural nuance, and the actual experience of being understood by another person. This pillar is the most infrastructure-heavy and is explicitly **deferred to Phase 4** (§9), but is specified in full here because it constrains earlier decisions (native/target language fields, mastery-as-derived-not-self-reported, learner scoping — all already present in the V1 schema for this reason).

**Two matching modes:**

1. **Language exchange (reciprocal).** Pair a Spanish-learner-who-speaks-English with an English-learner-who-speaks-Spanish. Session time splits between both languages, or sessions alternate which language is "in focus." Requires matching on complementary `(native_language, target_language)` pairs.

2. **Skill-matched pairing (shared target language).** Pair two learners at similar proficiency in the *same* target language — e.g., two A1 Spanish learners — for mutual practice with no native-speaker fluency gap and no performance pressure. Mastery level for matching purposes is **derived from the knowledge graph**, not self-reported, since self-reported level is notoriously unreliable and gameable.

**Matching engine inputs:** target language, native language, derived mastery level/band, availability window (timezone-aware), topic/scenario preference, prior session history (avoid re-matching the same pair every time unless they opt in to a recurring partner).

**Cold-start problem.** With a small user base, matches are sparse once you cross-cut by language pair, proficiency band, and timezone. This is precisely what Pillar 5's AI fallback exists to solve: if no human match is available within a configurable wait window, the AI conversation partner seamlessly steps in at the matched difficulty level instead of leaving the learner with an empty queue. The product never *requires* a human to be available; humans are additive, not load-bearing.

**Trust & safety.** Real-time chat (and later, voice/video) between strangers is a genuine cost center, not a checkbox: reporting/blocking, rate limiting on match requests, and (once voice is involved) a meaningfully larger abuse surface. This is flagged explicitly because it's easy to under-scope in a spec and expensive to retrofit — Phase 4 planning should budget real time for it, not treat it as a UI afterthought.

### 2.5 Pillar 5 — AI Fallback for Cold Start

Functionally, this is Pillar 3 (the AI conversation partner) invoked in fallback mode rather than a separate system. Worth calling out as its own goal because it's the thing that makes Pillar 4 viable at small scale: a learner should never see "no one is available" — they should seamlessly drop into an AI-run version of the same scenario at the same difficulty level, with the option to be matched with a human later if one becomes available mid-session or next time.

### 2.6 Pillar 6 — The Learner Knowledge Graph (the moat)

The component every other pillar depends on. A continuously updated model of:

```
Known Vocabulary:        ✓ comer, casa, agua
Partially Known:         ~ trabajo, importante
Unknown:                 ✗ aeropuerto, alquiler
Grammar (same structure): preterite-regular-ar: partial, ser/estar: known, ...
```

Every lesson, story, conversation turn, and (eventually) match difficulty is generated *from* this graph rather than from a fixed curriculum sequence. The product's fundamental question is always "teach me the next most useful thing," never "Lesson 47: Colors." Full design detail in §6 and §8.

### 2.7 Business Model & Monetization

**Free tier:**
- Reading content generation (stories, sentences)
- Vocabulary and grammar tracking / graph review
- Text-based AI conversation (rate-limited)

**Premium tier:**
- Unlimited / higher-limit voice conversations (Phase 3+)
- Specialized tutor scenarios (business language, exam prep, etc.)
- Pronunciation feedback (Phase 3+, depends on STT quality)
- Human conversation matching (Phase 4)
- Human tutor marketplace (Phase 4+, stretch — not detailed further here)

**Cost structure note:** LLM and voice API calls are the dominant variable cost, not infrastructure. Two architecture-level mitigations are already baked into earlier sections rather than left as a business afterthought:
- Content caching keyed on `(learner state hash, request type)` in `glossa-content` (§7) — regenerating from scratch every session is wasteful once graph state is mostly stable session-to-session.
- Free-tier rate limiting is implemented as a wrapper around the content/conversation service calls (a decorator-style trait, consistent with the pluggable-trait pattern used for `glossa-voice`), so gating logic lives in one place and isn't smeared across UI code.

---

## 3. Scope & Phasing Summary

| Pillar | V1 (personal use) | Phase 2 | Phase 3 | Phase 4 |
|---|---|---|---|---|
| 2.1 Comprehensible-input content | ✅ Built | | | |
| 2.2 Implicit grammar | ✅ Built (graph tracks patterns) | | | |
| 2.3 AI conversation partner (text) | | ✅ Built | | |
| 2.5 AI fallback | | ✅ Built (same engine as 2.3) | | |
| 2.6 Knowledge graph | ✅ Built (foundation for everything else) | | | |
| Voice (STT/TTS) | Trait stub only | | ✅ Implemented | |
| 2.4 Human-to-human matching | Schema designed for, not built | | | ✅ Built |
| Multi-user / auth / billing | | | | ✅ Built |
| 2.7 Monetization gating | | | | ✅ Built |
| Website deployment (vs. desktop/mobile app) | | | | ✅ Built |

V1 is single-user (you), single target language, no auth, no billing. The goal of V1 is to validate that the knowledge-graph-driven content generation and grammar emergence actually produce a better learning experience than existing apps — before investing in the infrastructure-heavy pillars (4, multi-user, billing).

---

## 4. System Architecture

### 4.1 Why Tauri

Tauri wraps a Rust backend with a web-technology frontend (HTML/CSS/JS via a framework of choice) inside a native webview shell. Three reasons this fits better than a Rust-native TUI/GUI for this specific project:

1. **The frontend is already web tech.** If this becomes a real website later, the frontend code is largely reusable — the transition is closer to swapping a deployment target than rewriting the UI layer.
2. **Tauri 2.x supports mobile targets** (iOS/Android) from the same codebase as desktop. The original product vision assumed a mobile-first conversation app; Tauri gets you most of the way there without a separate mobile codebase.
3. **The Rust backend stays Rust.** Domain logic (graph, content generation, conversation state) doesn't need to be reimplemented in JS — it's exposed to the frontend via Tauri's IPC command layer, keeping your actual expertise (Rust) where the actual complexity (the knowledge graph and content generation) lives.

### 4.2 Workspace Layout

```
glossa/
├── crates/
│   ├── glossa-core/          # domain types, no I/O
│   ├── glossa-graph/         # knowledge graph, mastery state, next-best-content selection
│   ├── glossa-content/       # LLM-backed story/sentence generation
│   ├── glossa-conversation/  # AI tutor chat engine + scenario library
│   ├── glossa-voice/         # pluggable STT/TTS trait (stub until Phase 3)
│   ├── glossa-storage/       # Postgres (writes) + DuckDB (analytics reads)
│   └── glossa-service/       # transport-agnostic orchestration layer (see 4.3)
├── src-tauri/                 # Tauri shell: registers glossa-service calls as IPC commands
└── frontend/                  # web frontend (SvelteKit recommended — see 5)
```

### 4.3 Data Flow & the Transport-Agnostic Service Layer

The key design decision for "might become a website" is `glossa-service`: a crate of plain async functions (no Tauri types, no HTTP types) that orchestrate the domain crates. Both Tauri commands *and* a future HTTP API are thin wrappers around the same functions.

```
frontend/ (SvelteKit, in Tauri webview today, in a browser later)
  → invoke('next_content')                         // Tauri IPC, today
       -- or --
  → fetch('/api/next-content')                      // HTTP, Phase 4
  → glossa-service::next_content(learner_id)         // same function either way
       → glossa-graph::next_best_content(state)
       → glossa-content::generate(request, vocab_window)
       → glossa-storage::record_event(...)
  ← ContentResponse
```

When Phase 4 arrives, the only new code is a `glossa-api` crate (Axum) that calls the same `glossa-service` functions over HTTP, plus auth/session middleware. `glossa-graph`, `glossa-content`, `glossa-conversation`, and `glossa-storage` don't change at all.

---

## 5. Crate / Module Breakdown

### `glossa-core`
Shared domain types, no I/O.

```rust
pub enum MasteryState { Unknown, Partial { confidence: f32 }, Known }

pub struct Lexeme {
    pub id: LexemeId,
    pub language: LanguageCode,
    pub lemma: String,
    pub pos: PartOfSpeech,
    pub frequency_rank: u32,
}

pub struct GrammarPattern {
    pub id: PatternId,
    pub label: String,           // e.g. "preterite-regular-ar"
    pub example_template: String,
}

pub struct LearnerProfile {
    pub id: LearnerId,
    pub target_language: LanguageCode,
    pub native_language: LanguageCode,
}

pub enum LearningEvent {
    StoryRead { story_id: Uuid, words_seen: Vec<LexemeId> },
    ChatTurn { conversation_id: Uuid, new_lexemes: Vec<LexemeId>, corrected: bool },
    ExerciseAnswered { lexeme_id: LexemeId, correct: bool },
}
```

### `glossa-graph`
The component everything else depends on. Owns:
- Mastery state transitions — how many correct/contextual exposures move `Unknown → Partial → Known`, with recency decay.
- `next_best_content(profile) -> ContentRequest` — frequency-weighted selection of which unknown/partial lexemes and grammar patterns to target next, given a target budget of 1–3 new items per piece of content.
- A real graph structure (not a flat table), so morphological relationships are usable later — e.g. mastering `comer` should boost confidence on `comí`, `comiendo`, `comido` — without that being required for V1's first pass.

### `glossa-content`
Wraps the Anthropic API for generation. Responsibilities:
- Builds prompts embedding the learner's known/partial vocab window (top-N most relevant, not the whole graph — token budget matters).
- Requests **structured JSON output**, not free text, so new vs. reinforced words are logged deterministically rather than parsed from prose:

```json
{
  "text": "Ayer comí pizza con mi amigo.",
  "known_words_used": ["ayer", "comer", "con", "mi", "amigo"],
  "new_words_introduced": ["pizza"],
  "grammar_targeted": "preterite-regular-ar"
}
```
- Enforces the known/new word ratio for stories (default 95/5, configurable).
- Caches generated content per `(learner state hash, request type)` to control LLM spend.

### `glossa-conversation`
- Maintains conversation state across turns (system prompt reconstructed each call with current learner vocab window + conversation history, since the API is stateless).
- Houses the scenario preset library (ordering food, job interview, airport travel, business meeting, etc. — see §2.3).
- Emits corrections as a structured side-channel field rather than interrupting conversational flow.
- This is also the engine invoked for Pillar 5 (AI fallback) — same code path, just triggered by "no human match available" instead of "no human matching exists yet."

### `glossa-voice` (stub until Phase 3)
Trait-based so providers are swappable later:

```rust
pub trait SpeechToText { fn transcribe(&self, audio: &[u8]) -> Result<String>; }
pub trait TextToSpeech { fn synthesize(&self, text: &str) -> Result<Vec<u8>>; }
```
No implementation in V1 — just the trait boundary, so adding a provider later doesn't touch `glossa-conversation`.

### `glossa-storage`
Same dual-database pattern as your sports betting app:
- **PostgreSQL** — source of truth, append-only event log + current state tables.
- **DuckDB** — analytics reads (mastery trends, frequency-priority queues), via `postgres_scanner` against the Postgres tables directly, or periodic materialization.

### `glossa-service`
Transport-agnostic orchestration (§4.3). This is the crate that makes the eventual website transition cheap — keep Tauri-specific and (later) HTTP-specific code entirely out of it.

### `src-tauri`
Registers `glossa-service` functions as Tauri commands. Owns app config, window setup, and (eventually) mobile build targets.

### `frontend`
Recommendation: **SvelteKit**. Reasoning:
- Small runtime footprint, good fit for a content-reading + chat UI.
- Has both a static-adapter path (good for a Tauri-embedded SPA) and a Node-adapter path (good for an eventual real website) without changing the component code — only the build target changes.
- React + Vite is a perfectly fine alternative if you'd rather stay in more familiar ecosystem territory; the architecture above doesn't depend on which one you pick, since the frontend only talks to `glossa-service` via `invoke()` either way.

Views needed: Reading (story with known/new word highlighting), Chat (AI tutor + corrections sidebar), Review (graph state: known/partial/unknown counts, what's queued next and why), Stats (mastery trend over time).

---

## 6. Data Model

```sql
-- Postgres (writes, source of truth)
learners(id, target_language, native_language, created_at)
lexemes(id, language, lemma, pos, frequency_rank)
learner_lexeme_state(learner_id, lexeme_id, status, confidence, exposure_count, last_seen_at)
grammar_patterns(id, language, label, example_template)
learner_grammar_state(learner_id, pattern_id, status, exposure_count)
events(id, learner_id, event_type, payload jsonb, created_at)   -- append-only
conversations(id, learner_id, scenario, started_at)
conversation_turns(id, conversation_id, speaker, text, new_lexemes jsonb, corrections jsonb, created_at)
stories(id, learner_id, content, known_word_ratio, generated_at)
```

```sql
-- DuckDB (reads, via postgres_scanner)
-- mastery_summary: counts by status/POS, trend over time
-- next_priority_queue: highest-frequency unknown/partial lexemes, ranked
-- session_stats: words encountered per session, exposure → mastery latency
```

```sql
-- Phase 4 schema sketch (NOT built in V1 — shown to confirm V1 fields are compatible)
match_requests(id, learner_id, target_language, native_language, mastery_band, mode, availability_window, created_at)
match_sessions(id, learner_a_id, learner_b_id, language_a, language_b, started_at, ended_at)
reports(id, match_session_id, reporter_id, reason, created_at)
```

`mastery_band` in `match_requests` is derived from `learner_lexeme_state` / `learner_grammar_state` at match-request time — never self-reported, per §2.4.

---

## 7. LLM Integration & Prompt Design

- System prompt per request includes: target language, the learner's current known/partial vocab window (top-N by relevance, not the full graph), the specific grammar pattern being targeted (if any), and the scenario context (for conversation).
- Output is requested as structured JSON (see §5, `glossa-content`) so the app never has to regex-parse prose to figure out what vocabulary was used.
- Corrections (conversation mode) are a separate JSON field from the conversational reply itself, so the frontend can render them in a sidebar without interrupting the chat transcript.
- Caching key: hash of `(learner_id, graph_state_version, request_type, scenario)`. Graph state changes relatively slowly within a session, so this meaningfully cuts redundant generation calls.

---

## 8. Tech Stack

| Concern | Choice |
|---|---|
| Core language | Rust |
| Async runtime | Tokio |
| LLM | Anthropic API (Claude) — generation + conversation |
| App shell | Tauri 2.x (desktop now, mobile-capable later) |
| Frontend framework | SvelteKit (recommended) or React + Vite |
| DB (writes) | PostgreSQL |
| DB (reads/analytics) | DuckDB |
| Serialization | serde / serde_json |
| Voice (Phase 3) | Pluggable trait — provider TBD |
| Future HTTP API (Phase 4) | Axum, calling `glossa-service` |

---

## 9. Designing for Multi-User & the Website Transition

Cheap V1 decisions that avoid a rewrite later:
- `LearnerId` is a real type from day one, even with only one row in `learners`.
- `events`, `conversation_turns`, etc. are already learner-scoped and append-only — the hard part of multi-tenancy, free to get right now.
- No global mutable state in `glossa-graph` — everything keyed by `LearnerId`, so it's already concurrency-safe for multiple learners.
- `glossa-service` contains zero Tauri-specific or HTTP-specific types — `src-tauri` and a future `glossa-api` are both thin adapters over it.
- The frontend's only backend-coupling point is the `invoke()` calls — swapping those for `fetch()` calls against `glossa-api` is a mechanical change, not a redesign, *if* the frontend framework choice (§5) supports a non-Tauri build target.

---

## 10. Roadmap

**Phase 0 — Foundation**
`glossa-core` + `glossa-storage` + `glossa-graph` skeleton. Seed lexeme/frequency data for your target language (need a licensing-clean frequency list).

**Phase 1 — MVP (reading loop)** — Pillars 2.1, 2.2, 2.6
`glossa-content` + `glossa-service` + Tauri shell + frontend Reading/Review views. Usable end-to-end at this point, for you, one language.

**Phase 2 — Conversation** — Pillars 2.3, 2.5
`glossa-conversation` + frontend Chat view. Tests whether the level-matching actually feels right — this is where the core "moat" claim gets validated.

**Phase 3 — Voice**
Implement `glossa-voice` against a real STT/TTS provider.

**Phase 4 — Multi-user / human matching / monetization** — Pillar 2.4, 2.7
`glossa-api` (Axum), auth, billing, matching engine, trust & safety tooling, real website deployment of `frontend/`. Only worth doing if Phases 1–3 prove the core loop works.

---

## 11. Open Questions

1. Target language(s) for V1 — single language or multi from the start? (Recommend single, to avoid generalizing the graph schema prematurely.)
2. Frequency list source — need one with a license that permits redistribution/derivation.
3. Mastery transition function — what exposure count / correctness pattern actually moves `Unknown → Partial → Known`? Start with a simple heuristic (N correct contextual exposures + recency decay) rather than a learned model.
4. SvelteKit vs. React for `frontend/` — does either conflict with tooling you already use elsewhere?
5. Should `glossa-graph` model morphological relationships (conjugations, derivations) in V1, or stay flat per-lemma and add that later? Flat is simpler and probably fine to start.
6. Phase 4 trust & safety (§2.4) is flagged but not designed — worth a dedicated pass when that phase actually gets scheduled, not retrofitted under time pressure.
