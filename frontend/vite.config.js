import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

// Tauri expects a fixed dev port and doesn't want Vite clearing its logs.
export default defineConfig({
	plugins: [sveltekit()],
	clearScreen: false,
	server: {
		port: 5173,
		strictPort: true
	}
});
