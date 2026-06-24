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

	// --- languages ---

	/** Languages that have seeded content: [{ code, name }]. */
	availableLanguages: () => invoke('available_languages'),

	/** Switch the active target language (e.g. "fr"). */
	setTargetLanguage: (code) => invoke('set_target_language', { code })
};

/** True when running inside the Tauri webview (vs. a plain browser tab). */
export function inTauri() {
	return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}
