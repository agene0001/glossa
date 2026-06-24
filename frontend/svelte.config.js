import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	preprocess: vitePreprocess(),
	kit: {
		// SPA mode: a single index.html fallback, embedded by Tauri as frontendDist.
		// The same build also serves a real website later (spec §5) — only the
		// adapter changes, not the components.
		adapter: adapter({ fallback: 'index.html' })
	}
};

export default config;
