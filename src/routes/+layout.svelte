<script lang="ts">
	import { onMount } from 'svelte';
	import { installPageGuards } from '$lib/page-guard';

	// app.css (Tailwind and its reset) is imported by the main page, not
	// here, so the /manual page keeps the manual's own styles.
	let { children } = $props();

	// Links and the webview's own context menu, in the app and in the
	// manual: see page-guard.ts.
	onMount(() => installPageGuards(window));
</script>

{@render children()}
