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
- **Frequency lists — DECIDED (option b):** vet an open, permissively-licensed frequency corpus. Using **hermitdave/FrequencyWords** (OpenSubtitles-derived, **MIT**): `https://raw.githubusercontent.com/hermitdave/FrequencyWords/master/content/2018/<lang>/<lang>_50k.txt` (format: `word count`, most-frequent first). Process = fetch top-N → **vet** (drop noise: names, fragments, non-words, English loanwords, bare particles) → dedupe against current inventory → **author gloss + POS + gender/translit ourselves** (list gives ordering/selection only). Selecting facts from an MIT list is clean.
- **Append-only constraint:** lexeme ids are `base + list_index`, and `lexeme_states` (progress) are keyed by id — so **never reorder existing entries**, only append. New words therefore get higher `frequency_rank` (lower "up next" priority) even if truly common; acceptable for now. A future stable-id scheme would let us re-sort by true frequency.
- **Dictionaries:** the offline EN→target dictionaries (~162 entries each es/fr/de) can grow the same curated way; Wiktionary/Wiktextract is CC BY-SA if we ever want scale (attribution + share-alike caveats).
- **Correctness:** the user reached ~B2 German and can nitpick DE; ES/FR/RU are harder to verify — be conservative, prefer well-known words, and mark/skip when unsure.

## Architecture seams to reuse
- Static reference content (no learner state): `seed.rs` fn → Tauri command → frontend page. Templates: `pronunciation_guide`, and now `external_resources`.
- Per-language content: `seed_language` at a fresh base (es=0, fr=1000, de=2000, ru=3000, next=4000). Add `<lang>_units/_grammar/_packs`, optional `glossa-lemma` rules.
- Exercises: extend `ExerciseKind` + `build_exercise`; everything downstream (record, UI) already generalizes.
- LLM: `glossa-content::ContentGenerator` (Anthropic + Mock). New capabilities = new trait methods, degrade gracefully in mock/offline.

## Progress log
- **2026-07-18** — Roadmap doc created. **P4 (external resources) shipped**: `ExternalResource`/`ResourceGuide` in core, curated free YouTube/podcast/tool links per language (es/fr/de/ru) + a universal set in `seed.rs`, `external_resources` command, `/immerse` page + nav "Immerse". Links open externally (verify the packaged app opens the system browser — may need the tauri opener plugin; noted below).
  - **Open item:** confirm external links open the system browser in the packaged Tauri app (we use a custom `open_external` command via the `open` crate — should work; verify).
- **2026-07-18 (later)** — **P0 batch 1 (German)**: sourcing decided (option b, hermitdave MIT list — see Content sourcing). Vetted the top German frequency words and appended **73 high-frequency words** to `de_frequency.json` (123 → **196**): question words (was/wer/wie/wo/wann/warum), quantifiers/adverbs (viel/mehr/schon/noch/nur/immer/nie/oft/wieder/vielleicht/wirklich/genau/ganz/einfach/sicher/klar/schnell/langsam), connectors (weil/wenn/dass/als/damit/ob), prepositions (von/aus/nach/vor/bei/über/unter/durch/um/bis/ohne), modals + core verbs (werden/müssen/sollen/dürfen/mögen/sagen/wissen/geben/nehmen/finden/denken/glauben/reden/meinen/lassen/bleiben/heißen/leben/spielen/passieren/tun), nouns (Leben/Leute/Herr/Gott). Added 2 DE packs (7 Question & Function Words, 8 More Common Verbs) so they're studiable. Append-only (ids preserved). **Felix to spot-check the German glosses/genders.**
- **2026-07-18 (P0 batch 2, German)** — appended **~100 concrete words** to `de_frequency.json` (196 → **298**, all unique): animals, food/drink, household, **body** (Auge/Ohr/Nase/Mund/Fuß/Bein/Arm/Herz/Haar/Gesicht), **nature & weather** (Baum/Blume/Sonne/Mond/Regen/Berg/Meer/Fluss/Himmel/Wald/Wetter/Feuer), **places** (Garten/Park/Geschäft/Restaurant/Krankenhaus/Bahnhof/Kirche/Wohnung), plus common adjectives (kalt/warm/heiß/teuer/billig/lang/kurz/stark/schwach/voll/leer/früh/spät/richtig/falsch/krank/gesund/nett) and verbs (schlafen/laufen/schreiben/hören/fühlen/warten/fragen/antworten/öffnen/schließen/bringen/zeigen/stehen/sitzen/treffen/kennen/anfangen/verlieren). Added 4 packs (9 Animals, 10 The Body, 11 Nature & Weather, 12 Places in Town). German inventory: **123 → 298** across batches 1–2. **Felix to spot-check genders.**
- **2026-07-18 (P0, German teaching units)** — added **7 German units** (9–15) that actively teach the batch-2 vocabulary with objectives, graded readings, and examples: 9 Animals, 10 The Body, 11 Nature & Weather, 12 Around Town (two-way prepositions), 13 More Food & Drink, 14 Describing Things (comparative), 15 Everyday Verbs. German course is now **8 → 15 units**. **Felix to spot-check the German reading/example sentences.**
- **2026-07-18 (P0, German B1/B2 depth)** — batch 3: +61 abstract/B1 words (359 total) — feelings (Liebe/Angst/Freude/Gefühl/Meinung), abstract nouns (Zukunft/Gesellschaft/Regierung/Krieg/Frieden/Erfahrung/Möglichkeit/Sprache/Geschichte), B1 verbs (erklären/beschreiben/entscheiden/versuchen/bedeuten/erinnern/freuen/interessieren/verbessern…), adjectives (möglich/gefährlich/frei/ruhig/laut/dunkel/hell/arm/reich/traurig/wütend…), adverbs (plötzlich/endlich/manchmal/trotzdem/deshalb…). Added **4 higher-level units** with longer readings that exercise the advanced grammar: 16 Feelings & Opinions (reflexive), 17 Telling a Story (Perfekt narration), 18 Making Plans (Konjunktiv II), 19 Society & the World (passive + connectors). +2 packs (13 Feelings & Opinions, 14 Society & Ideas). **German now: 359 words, 19 units A1→B2, 14 packs, 23 grammar lessons.** Felix to spot-check the B1/B2 readings.
  - **Next up:** German is now a genuinely deep course — good stopping point. Next major moves: **ES/FR/RU** vocab+unit expansion via the hermitdave process (conservatively); the re-rank/stable-id migration; and **P1** (LLM graded-reading library — the biggest remaining lever).
