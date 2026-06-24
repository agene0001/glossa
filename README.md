# Glossa

AI-native language learning, built around an accurate, continuously-updated model
of what *you* actually know. Every sentence, story, and (later) conversation is
generated from your **knowledge graph** at exactly your level — there is no
"Lesson 47", only "the next most useful thing for you, right now."

This repository implements **V1** of [`glossa_product_spec.md`](./glossa_product_spec.md):
the single-user reading loop (Pillars 2.1 comprehensible input, 2.2 implicit
grammar, 2.6 the knowledge graph), with the data model and crate boundaries
already shaped for the later phases (conversation, voice, human matching, web).

Stack: **Rust** domain logic → **Tauri 2** desktop shell → **SvelteKit** UI,
with the **Anthropic API** for generation.

---

## What works in V1

- **Roadmap (Learn tab)** — an ordered path of themed units (Duolingo-style)
  with visible progress and lock state, so you always know where you are and
  what's next. A unit unlocks once the previous one is half learned.
- **Unit lessons** — each unit teaches a small set of words + a grammar focus
  through **hand-authored, coherent example sentences** (with meanings and
  translations) — so lessons read well even with no API key. Tap any word for
  its meaning, tap **🔊 to hear it** (or the whole sentence) via the system
  voice, and read an opt-in **grammar tip**. When an API key is set, a unit also
  offers **AI practice** that introduces only that unit's words. Finishing a
  lesson shows a completion celebration and keeps a **daily streak**.
- **Multiple languages** — Spanish and French ship with full content; switch the
  target language from the sidebar. The data model is language-namespaced, so
  adding a language is just a frequency list + units (no schema changes).
- **Onboarding** — first launch asks whether you're a beginner or already know
  some words; the placement step lets you tick known words so the roadmap starts
  at the right level.
- **Knowledge graph** — frequency-weighted next-word selection, mastery
  transitions with recency decay, all driven by an append-only event log.
- **Review view** — known/learning/unseen counts, grammar-pattern progress, and
  the priority queue of what's coming next (and why).
- **Persistence** — your progress is saved to disk and survives restarts.
- **Runs with zero config** — no API key? It uses a built-in offline generator
  so the whole loop is usable immediately. Add a key for real content.

Chat (Phase 2) and Stats (DuckDB analytics) are present as placeholders; the
types/engine boundaries exist but the features are not built in V1.

---

## Architecture

A Cargo workspace of small, single-responsibility crates (spec §4.2). The key
decision is `glossa-service`: plain async functions with **no Tauri or HTTP
types**, so a future website is a thin `glossa-api` (Axum) over the same
functions — not a rewrite (spec §4.3, §9).

```
crates/
  glossa-core/          domain types, no I/O
  glossa-graph/         mastery transitions + next-best-content (pure, tested)
  glossa-content/       Anthropic generation (structured JSON) + offline mock
  glossa-conversation/  Phase 2 — scenarios/engine trait (stub)
  glossa-voice/         Phase 3 — STT/TTS trait boundary (stub)
  glossa-storage/       Store trait + file-backed store; Postgres schema.sql
  glossa-service/       transport-agnostic orchestration (the website seam)
src-tauri/              Tauri shell: state, IPC commands, first-run seeding
frontend/              SvelteKit SPA (Reading / Review / Chat / Stats)
```

Data flow for "give me the next thing to read":

```
frontend  invoke('next_content')
  → glossa-service::next_content()
       → glossa-graph::next_best_content(state)     // what to teach
       → glossa-content::generate(request)          // text + structured words
       → glossa-storage::save_story(...)            // so a later read credits it
  ← ContentResponse  (tokens tagged by mastery, new-word glossary, ratio)
```

---

## Prerequisites

- **Rust** (stable) and **Cargo**
- **Node** 18+ and **npm**
- macOS users: Xcode Command Line Tools (for the WebView). Linux/Windows: see
  [Tauri prerequisites](https://tauri.app/start/prerequisites/).
- *Optional:* PostgreSQL, only if you swap in the Postgres store (see below).

---

## Run it

```bash
# from the repo root
npm run setup      # installs the Tauri CLI + the frontend deps
npm run dev        # launches the desktop app (starts Vite, then Tauri)
```

`npm run dev` runs `tauri dev`, which boots the SvelteKit dev server and opens
the Glossa window. First launch seeds a small Spanish frequency list and creates
your learner profile automatically.

### Live content (optional)

By default the app uses an **offline mock** generator (the sidebar shows a
`mock` badge). To generate real content, set an Anthropic API key before
launching:

```bash
cp .env.example .env        # then edit it, or just export the var:
export ANTHROPIC_API_KEY=sk-ant-...
npm run dev                 # sidebar now shows a `live` badge
```

The default model is `claude-opus-4-8`. For the high-volume reading loop you can
choose a cheaper/faster model with `GLOSSA_MODEL` (e.g. `claude-haiku-4-5`).

---

## Tests

```bash
cargo test --workspace     # 15 tests: graph mastery, selection, storage, full loop, seed
```

The graph and the end-to-end service loop (generate → read → mastery advances)
are covered without needing a network or an API key.

---

## Where your data lives

V1 persists to a single JSON file in the OS app-data directory (macOS:
`~/Library/Application Support/com.glossa.app/glossa.json`). Delete it to reset
your progress. The on-disk format is human-readable while you tune the model.

---

## Deliberate V1 scoping (and how to grow it)

These are conscious V1 simplifications, each with the seam to extend already in
place:

| Area | V1 | Next step |
|---|---|---|
| Storage | File-backed `Store` (zero setup, persists) | Implement `PgStore` against `crates/glossa-storage/schema.sql` behind the same `Store` trait — nothing else changes (spec §6, §9). |
| Analytics | None | DuckDB read path over the append-only events (spec §5, Stats view). |
| Content caching | None | Decorator over `ContentGenerator` keyed on `(graph-state hash, request type)` (spec §2.7, §7). |
| Word matching | Flat lemma, lowercased surface forms | Morphology so `comí`/`comiendo` credit `comer` (spec §11.5). |
| Conversation / Voice | Trait stubs | Phase 2 / Phase 3. |
| Languages | Spanish only | The schema is already multi-language; add a frequency list + grammar set. |

The Postgres schema for the full data model (including the Phase-2 conversation
tables and a note on Phase-4 matching) is in
[`crates/glossa-storage/schema.sql`](./crates/glossa-storage/schema.sql).

---

## Note: building on an exFAT/NTFS volume (this machine)

This repo lives on an exFAT volume, which can't store the extended attributes
macOS attaches to files — so macOS scatters `._*` AppleDouble sidecars into the
tree, and Tauri's build script chokes parsing them (it can't be built directly
on exFAT). Two things handle it:

- **`./target` is a symlink to an APFS location** (`~/Library/Caches/glossa/target`),
  so build artifacts appear in the project dir but the bytes live on a
  filesystem that supports xattrs. Recreate it with:
  ```bash
  mkdir -p ~/Library/Caches/glossa/target && ln -s ~/Library/Caches/glossa/target ./target
  ```
  If your repo is on an APFS/HFS+ volume, delete the symlink and let Cargo use a
  normal `./target` directory.
- The Tauri `beforeDev/BuildCommand` runs `dot_clean` first to strip sidecars
  from the source tree (e.g. `capabilities/`).

If a build ever fails with `._something: stream did not contain valid UTF-8`,
run `dot_clean -m .` from the repo root and rebuild.

---

## License

MIT OR Apache-2.0.
