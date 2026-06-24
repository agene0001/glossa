-- Glossa — PostgreSQL schema (source of truth, writes).
--
-- This is the production storage target from spec §6. V1 runs on the
-- file-backed `FileStore`, but every field here is already represented in the
-- domain types, so a `PgStore` implementing the `Store` trait is a drop-in.
--
-- DuckDB handles analytics reads via `postgres_scanner` against these tables
-- (mastery trends, frequency-priority queue, session stats) — no separate ETL.
--
-- Apply with:  psql "$DATABASE_URL" -f crates/glossa-storage/schema.sql

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Who is learning, and the language pair. native_language is carried from V1
-- because Phase 4 human matching pairs on (native, target) — spec §2.4, §9.
CREATE TABLE IF NOT EXISTS learners (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    target_language TEXT NOT NULL,
    native_language TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- The seeded vocabulary inventory. frequency_rank is 1-based (1 = most common).
CREATE TABLE IF NOT EXISTS lexemes (
    id             BIGINT PRIMARY KEY,
    language       TEXT NOT NULL,
    lemma          TEXT NOT NULL,
    pos            TEXT NOT NULL,
    frequency_rank INTEGER NOT NULL,
    UNIQUE (language, lemma, pos)
);
CREATE INDEX IF NOT EXISTS lexemes_lang_freq_idx ON lexemes (language, frequency_rank);

-- Grammar patterns tracked exactly like vocabulary (spec §2.2).
CREATE TABLE IF NOT EXISTS grammar_patterns (
    id               BIGINT PRIMARY KEY,
    language         TEXT NOT NULL,
    label            TEXT NOT NULL,
    example_template TEXT NOT NULL,
    UNIQUE (language, label)
);

-- Per-learner mastery state for vocabulary (current state, mutated in place).
CREATE TABLE IF NOT EXISTS learner_lexeme_state (
    learner_id     UUID NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    lexeme_id      BIGINT NOT NULL REFERENCES lexemes(id),
    status         TEXT NOT NULL,                 -- unknown | partial | known
    confidence     REAL NOT NULL DEFAULT 0,       -- meaningful when status = partial
    exposure_count INTEGER NOT NULL DEFAULT 0,
    last_seen_at   TIMESTAMPTZ,
    PRIMARY KEY (learner_id, lexeme_id)
);

-- Per-learner mastery state for grammar patterns.
CREATE TABLE IF NOT EXISTS learner_grammar_state (
    learner_id     UUID NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    pattern_id     BIGINT NOT NULL REFERENCES grammar_patterns(id),
    status         TEXT NOT NULL,
    confidence     REAL NOT NULL DEFAULT 0,
    exposure_count INTEGER NOT NULL DEFAULT 0,
    last_seen_at   TIMESTAMPTZ,
    PRIMARY KEY (learner_id, pattern_id)
);

-- Append-only event log: the only thing that drives mastery changes (spec §9).
CREATE TABLE IF NOT EXISTS events (
    id         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    learner_id UUID NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    payload    JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX IF NOT EXISTS events_learner_time_idx ON events (learner_id, created_at);

-- Generated content. lexeme_ids/new_lexeme_ids let a later "read" event credit
-- exposures without the frontend re-sending the word list.
CREATE TABLE IF NOT EXISTS stories (
    id                 UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    learner_id         UUID NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    language           TEXT NOT NULL,
    content            TEXT NOT NULL,
    lexeme_ids         JSONB NOT NULL DEFAULT '[]',
    new_lexeme_ids     JSONB NOT NULL DEFAULT '[]',
    grammar_pattern_id BIGINT REFERENCES grammar_patterns(id),
    known_word_ratio   REAL NOT NULL,
    generated_at       TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Conversation tables (Phase 2 — schema present so V1 fields stay compatible).
CREATE TABLE IF NOT EXISTS conversations (
    id         UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    learner_id UUID NOT NULL REFERENCES learners(id) ON DELETE CASCADE,
    scenario   TEXT,
    started_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS conversation_turns (
    id              UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    conversation_id UUID NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    speaker         TEXT NOT NULL,                 -- learner | tutor
    text            TEXT NOT NULL,
    new_lexemes     JSONB NOT NULL DEFAULT '[]',
    corrections     JSONB NOT NULL DEFAULT '[]',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Phase 4 human-matching tables are intentionally NOT created here; see spec §6
-- for the sketch confirming V1 fields are forward-compatible.
