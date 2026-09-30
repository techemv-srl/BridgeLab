<script lang="ts">
	import { onMount } from 'svelte';

	// app.css (Tailwind and its reset) is imported by the main page, not
	// here, so the /manual page keeps the manual's own styles.
	let { children } = $props();

	// A plain click on a mailto: or web link must not navigate the app's own
	// window: on Linux the webview then shows "The URL can't be shown" and the
	// whole app is gone. Every such link, in the app and in the manual, opens
	// with the system's mail client or browser instead.
	onMount(() => {
		const onClick = (e: MouseEvent) => {
			if (e.defaultPrevented || e.button !== 0) return;
			const a = (e.target as Element | null)?.closest?.('a[href]') as HTMLAnchorElement | null;
			if (!a) return;
			const href = a.getAttribute('href') ?? '';
			if (!/^(mailto:|tel:|https?:)/i.test(href)) return;
			if (/^https?:/i.test(href) && new URL(href, location.href).origin === location.origin) return;
			e.preventDefault();
			import('@tauri-apps/plugin-opener')
				.then(({ openUrl }) => openUrl(href))
				.catch(() => window.open(href, '_blank'));
		};
		// Capture phase, so it runs before the opener plugin's own handler,
		// which only takes links with target="_blank".
		document.addEventListener('click', onClick, true);
		return () => document.removeEventListener('click', onClick, true);
	});
</script>

{@render children()}
