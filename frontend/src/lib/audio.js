// Word/sentence pronunciation via the webview's built-in Web Speech API.
// No backend, no API cost — uses the OS's installed voices. (Provider-based
// TTS/STT for conversation is a separate Phase-3 concern in glossa-voice.)

const LANG_BCP47 = { es: 'es-ES', fr: 'fr-FR', de: 'de-DE', ru: 'ru-RU', en: 'en-US' };

// Isolated single-letter words get read as letter *names* by the TTS engine
// ("y" → "i griega" / "why" instead of the conjunction /i/). Respell the few
// function words that hit this so they're spoken, not spelled. Trick: a Spanish
// voice reads the letter "i" as "ee" — exactly how the conjunction "y" sounds.
const OVERRIDES = {
	es: { y: 'i' },
	fr: { y: 'i', a: 'ah' }
};

export function canSpeak() {
	return typeof window !== 'undefined' && 'speechSynthesis' in window;
}

// Pick (and cache) an installed voice for a language. Setting `u.lang` alone
// lets the engine fall back to the default system voice (often English), which
// anglicizes pronunciation; pinning an actual voice avoids that.
const voiceCache = {};
function voiceFor(bcp47) {
	if (bcp47 in voiceCache) return voiceCache[bcp47];
	const prefix = bcp47.slice(0, 2).toLowerCase();
	const voices = window.speechSynthesis.getVoices();
	// Prefer an exact locale match, else any voice for the same language.
	const v =
		voices.find((x) => x.lang?.toLowerCase() === bcp47.toLowerCase()) ??
		voices.find((x) => x.lang?.toLowerCase().startsWith(prefix)) ??
		null;
	// getVoices() can be empty until the engine loads them — only cache a hit.
	if (v || voices.length) voiceCache[bcp47] = v;
	return v;
}

// Voices often aren't ready on first call; warm the cache when they arrive.
if (canSpeak() && typeof window.speechSynthesis.addEventListener === 'function') {
	window.speechSynthesis.addEventListener('voiceschanged', () => {
		for (const k of Object.keys(voiceCache)) delete voiceCache[k];
	});
}

export function speak(text, code = 'es') {
	if (!canSpeak() || !text) return;
	try {
		const spoken = OVERRIDES[code]?.[text.trim().toLowerCase()] ?? text;
		const u = new SpeechSynthesisUtterance(spoken);
		const bcp47 = LANG_BCP47[code] ?? code;
		u.lang = bcp47;
		const voice = voiceFor(bcp47);
		if (voice) u.voice = voice;
		u.rate = 0.92;
		window.speechSynthesis.cancel(); // stop any in-flight utterance
		window.speechSynthesis.speak(u);
	} catch {
		/* speech unavailable — ignore */
	}
}
