import { sveltekit } from '@sveltejs/kit/vite';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { defineConfig } from 'vitest/config';

export default defineConfig({
	plugins: [sveltekit({ alias: { $lib: 'src/lib' }, preprocess: vitePreprocess() })],
	test: {
		include: ['src/**/*.test.ts'],
		environment: 'node',
	},
});
