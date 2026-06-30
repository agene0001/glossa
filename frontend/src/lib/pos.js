// Short, learner-facing labels for parts of speech, shared across the vocab UI.
// "other"/unknown returns '' so we simply show no badge.
const LABELS = {
	noun: 'noun',
	verb: 'verb',
	adjective: 'adj.',
	adverb: 'adv.',
	pronoun: 'pron.',
	preposition: 'prep.',
	conjunction: 'conj.',
	determiner: 'det.',
	numeral: 'num.',
	interjection: 'interj.'
};

export const posLabel = (pos) => LABELS[pos] ?? '';

/** Options for a part-of-speech picker (value matches the backend enum). */
export const POS_OPTIONS = [
	{ value: '', label: 'type…' },
	{ value: 'noun', label: 'noun' },
	{ value: 'verb', label: 'verb' },
	{ value: 'adjective', label: 'adjective' },
	{ value: 'adverb', label: 'adverb' }
];
