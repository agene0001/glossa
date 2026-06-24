// Word/sentence pronunciation via the webview's built-in Web Speech API.
// No backend, no API cost — uses the OS's installed voices. (Provider-based
// TTS/STT for conversation is a separate Phase-3 concern in glossa-voice.)

const LANG_BCP47 = { es: 'es-ES', fr: 'fr-FR', en: 'en-US' };

export function canSpeak() {
	return typeof window !== 'undefined' && 'speechSynthesis' in window;
}

export function speak(text, code = 'es') {
	if (!canSpeak() || !text) return;
	try {
		const u = new SpeechSynthesisUtterance(text);
		u.lang = LANG_BCP47[code] ?? code;
		u.rate = 0.92;
		window.speechSynthesis.cancel(); // stop any in-flight utterance
		window.speechSynthesis.speak(u);
	} catch {
		/* speech unavailable — ignore */
	}
}
