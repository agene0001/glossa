# Expanding Glossa into a real language-learning platform

A living roadmap for turning Glossa from a *strong vocab + grammar trainer* into
something you can genuinely learn (not just review) a language with. Pick up
where the last session left off — see **Progress log** at the bottom.

## The honest starting point (adversarial audit, 2026-07-18)

Glossa today is a **competent trainer/reviewer, not a standalone path to
fluency.** You can build vocabulary, grammar knowledge, and reading/listening
*recognition*; you can't yet reach communicative competence (speaking,
understanding real speech, conversing). That's the ceiling of this class of app
unless we close specific gaps.

Verified state at audit time:

| Language | Words | Units | Grammar lessons |
|---|---|---|---|
| Spanish | 211 | 15 | 6 (A1) |
| German | 123 | 8 | 23 (A1–B2) |
| Russian | 87 | 4 | 2 (A1) |
| French | 82 | 5 | 5 (A1) |

Exercise engine is genuinely good: recognition (choose meaning/word), production
(type), and dictation/listening (listen→choose, listen→type), mixed by
confidence. Mastery is a homegrown decay heuristic (not a real SRS).

### The real gaps (why you can't *acquire* a language with it yet)
1. **Input volume far too low** — a dozen short authored passages/lang. Acquisition needs *hundreds of hours* of level-appropriate input.
2. **Vocabulary breadth ~1–3% of functional** — need ~2–3k words for basic speech, ~8k families for comfortable reading. We seed 82–211.
3. **No real listening** — listening exercises are single-word synthetic TTS, not connected native speech.
4. **No speaking / output in the wild** — STT researched and shelved (see README "Future" section); typing a word ≠ producing speech.
5. **No interaction** — `/chat` is a stub. No negotiated meaning / pushed output.
6. **Retrieval is isolated & de-contextualized** — mostly single words; grammar is single-blank drills (awareness, not automatization).
7. **Progress ≠ proficiency** — "known words" is the heuristic's opinion; no CEFR-aligned assessment.
8. **Content-correctness risk** — foreign content is single-model-authored, largely unverifiable by the user.

## Roadmap (highest leverage first)

> The #1 lever: **turn the LLM into an endless graded-reading + conversation
> engine.** It attacks input-volume, context, and interaction at once, and the
> architecture is already closest to supporting it. Everything else refines
> around that.

### P0 — Vocabulary + examples at real scale  *(biggest agreed need)*
- Expand each language's inventory toward the **top ~1,000–2,000 frequency words** (from a license-clean source — see "Content sourcing"), with gloss, POS, and (non-Latin) transliteration.
- More **example sentences** and **more units** so the added words are actually taught, not just listed.
- Group new vocab into more **themed packs** and into the **course units** by level.
- **Correctness strategy:** draft with the LLM at build time, but review carefully; prefer a vetted frequency list + dictionary over free-typing; spot-check genders/inflections. Flag uncertain entries.

### P1 — Graded reading/listening library (the big lever)
- A **library of level-appropriate passages** (LLM-generated graded readers, keyed to the learner's known-vocab so they're ~95% comprehensible), with tap-gloss, audio, and translation — and crucially **re-encounter of the learner's vocab in varied contexts** (how words actually stick).
- Add **passage-length listening** (TTS of whole passages at natural rate) — better than single words even before real native audio.
- Persist a reading history so words get credited (exposure model already exists).

### P2 — Sentence-level production
- New exercise types: **sentence cloze** (blank in a real sentence), **translate a sentence** (both directions), **word-order / sentence building**. Moves retrieval from isolated words to usage. Reuses the `Exercise` engine + `build_exercise` seam.

### P3 — LLM conversation mode (finish `/chat`)
- Text (later voice) conversation with correction — the one realistic way to add interaction/output. Uses the existing `glossa-content` Anthropic seam.

### P4 — Curated external resources / immersion  ✅ DONE (see Progress log)
- Hand-curated **free** YouTube channels + podcasts + tools per language, with links (static curation, **not** auto-discovery). Points learners at massive real input we can't replicate, and at speaking/exchange (Tandem/HelloTalk) and retention (Anki/FSRS).

### P5 — Retention: FSRS
- Adopt `fsrs-rs` scoped to the Quiz/Review scheduler (see memory `followup-fsrs-scheduler`). Optional; approximate today.

### P6 — Proficiency check
- A periodic mixed assessment mapped to CEFR so "progress" means ability, not activity.

## Content sourcing (for P0/P1)
- **Frequency lists:** need a permissively-licensed top-N list per language (the existing seeds are hand-authored/curated — keep that ethos, or vet an open list like a CC-licensed frequency corpus). Avoid copyright-encumbered lists.
- **Dictionaries:** the offline EN→target dictionaries (~162 entries each es/fr/de) can grow the same curated way; Wiktionary/Wiktextract is CC BY-SA if we ever want scale (attribution + share-alike caveats).
- **Correctness:** the user reached ~B2 German and can nitpick DE; ES/FR/RU are harder to verify — be conservative, prefer well-known words, and mark/skip when unsure.

## Architecture seams to reuse
- Static reference content (no learner state): `seed.rs` fn → Tauri command → frontend page. Templates: `pronunciation_guide`, and now `external_resources`.
- Per-language content: `seed_language` at a fresh base (es=0, fr=1000, de=2000, ru=3000, next=4000). Add `<lang>_units/_grammar/_packs`, optional `glossa-lemma` rules.
- Exercises: extend `ExerciseKind` + `build_exercise`; everything downstream (record, UI) already generalizes.
- LLM: `glossa-content::ContentGenerator` (Anthropic + Mock). New capabilities = new trait methods, degrade gracefully in mock/offline.

## Progress log
- **2026-07-18** — Roadmap doc created. **P4 (external resources) shipped**: `ExternalResource`/`ResourceGuide` in core, curated free YouTube/podcast/tool links per language (es/fr/de/ru) + a universal set in `seed.rs`, `external_resources` command, `/immerse` page + nav "Immerse". Links open externally (verify the packaged app opens the system browser — may need the tauri opener plugin; noted below).
  - **Next up:** P0 vocabulary expansion. Start with German (user can verify) — grow `de_frequency.json` toward the top ~500–1000, then more DE units/packs to teach them. Then ES/FR/RU conservatively.
  - **Open item:** confirm external links open the system browser in the packaged Tauri app; if not, wire `@tauri-apps/plugin-opener`.
