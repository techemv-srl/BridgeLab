import adapter from '@sveltejs/adapter-static';
import { sveltekit } from '@sveltejs/kit/vite';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
	plugins: [
		tailwindcss(),
		// SvelteKit 3 takes its configuration here (svelte.config.js is no
		// longer read). A static SPA: the Tauri shell serves index.html.
		sveltekit({
			// SvelteKit 3 no longer defines `$lib`; keep it rather than rewrite
			// every import to `#lib/….ts` (deprecated, still supported in 3.x).
			alias: { $lib: 'src/lib' },
			preprocess: vitePreprocess(),
			adapter: adapter({
				fallback: 'index.html'
			}),
			// The app ships inside the desktop bundle and never changes while
			// it runs: no polling for a new version (SvelteKit 3 polls hourly).
			version: { pollInterval: 0 }
		})
	],
	clearScreen: false,
	server: {
		port: 1420,
		strictPort: true,
		watch: {
			ignored: ['**/src-tauri/**']
		}
	}
});
