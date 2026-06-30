// Thin wrapper over Tauri IPC. Every call maps 1:1 to a `glossa-service`
// function exposed as a command in `src-tauri/src/lib.rs`. Swapping these for
// `fetch('/api/...')` is the only change needed to run against a future HTTP
// backend (spec §9) — the rest of the UI is transport-agnostic.
import { invoke } from '@tauri-apps/api/core';

export const api = {
	/** { generator: "anthropic"|"mock", language, learner_id } */
	backendStatus: () => invoke('backend_status'),

	/** Generate the next best ContentResponse for the learner. */
	nextContent: () => invoke('next_content'),

	/** Record that a story was read (credits exposures + advances mastery). */
	recordStoryRead: (storyId, understood) =>
		invoke('record_story_read', { storyId, understood }),

	/** Counts + the priority queue. `queueLimit` sizes the queue (placement uses more). */
	graphOverview: (queueLimit = 15) => invoke('graph_overview', { queueLimit }),

	/** Manually set a word's mastery: status = "known" | "partial" | "unknown". */
	setLexemeStatus: (lexemeId, status) =>
		invoke('set_lexeme_status', { lexemeId, status }),

	// --- curriculum / roadmap ---

	/** The learning roadmap: units with progress + lock state. */
	roadmap: () => invoke('roadmap'),

	/** Full lesson payload for one unit (authored examples + vocab). */
	unitLesson: (unitId) => invoke('unit_lesson', { unitId }),

	/** Record that a unit's lesson was studied (advances its words). */
	completeUnitLesson: (unitId, understood) =>
		invoke('complete_unit_lesson', { unitId, understood }),

	/** Extra AI practice scoped to a unit's vocabulary. */
	nextContentForUnit: (unitId) => invoke('next_content_for_unit', { unitId }),

	// --- vocabulary packs (breadth track) ---

	/** Themed vocab packs with progress: [{ id, title, emoji, percent, ... }]. */
	vocabPacks: () => invoke('vocab_packs'),

	/** A pack's flashcard deck (words + status). */
	packLesson: (packId) => invoke('pack_lesson', { packId }),

	/** A multiple-choice quiz over a pack's words (weakest/unseen first). */
	packQuiz: (packId, limit = 12) => invoke('pack_quiz', { packId, limit }),

	// --- user-authored decks (custom flashcards) ---

	/** The learner's own decks with progress: [{ id, title, emoji, percent, ... }]. */
	listDecks: () => invoke('list_decks'),

	/** Create a new empty deck; returns its summary. */
	createDeck: (title, emoji) => invoke('create_deck', { title, emoji }),

	/** Delete a deck and the words that belonged only to it. */
	deleteDeck: (deckId) => invoke('delete_deck', { deckId }),

	/** Add a word (term + meaning) to a deck. */
	addDeckWord: (deckId, lemma, gloss) => invoke('add_deck_word', { deckId, lemma, gloss }),

	/** Remove a word from a deck. */
	removeDeckWord: (deckId, lexemeId) => invoke('remove_deck_word', { deckId, lexemeId }),

	/** A deck's flashcard deck (words + status). */
	deckLesson: (deckId) => invoke('deck_lesson', { deckId }),

	/** A multiple-choice quiz over a deck's words. */
	deckQuiz: (deckId, limit = 12) => invoke('deck_quiz', { deckId, limit }),

	// --- languages ---

	/** Languages that have seeded content: [{ code, name }]. */
	availableLanguages: () => invoke('available_languages'),

	/** Switch the active target language (e.g. "fr"). */
	setTargetLanguage: (code) => invoke('set_target_language', { code }),

	// --- grammar track ---

	/** Grammar lessons with mastery + prerequisite lock state. */
	grammarTrack: () => invoke('grammar_track'),

	/** One grammar lesson: explanation + drills. */
	grammarLesson: (patternId) => invoke('grammar_lesson', { patternId }),

	/** Record a grammar drill answer for a pattern. */
	recordGrammarExercise: (patternId, correct) =>
		invoke('record_grammar_exercise', { patternId, correct }),

	// --- review / spaced-repetition quiz ---

	/** A spaced-repetition review session (weakest words first). */
	reviewSession: (limit = 12) => invoke('review_session', { limit }),

	/** Record a quiz answer for a word. */
	recordExercise: (lexemeId, correct) => invoke('record_exercise', { lexemeId, correct }),

	/** How many learned words are available to review. */
	reviewableCount: () => invoke('reviewable_count')
};

/** True when running inside the Tauri webview (vs. a plain browser tab). */
export function inTauri() {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}
