<script lang="ts">
	// The user manual as an app page. The manual window (HelpWindow.svelte)
	// loads `manual?lang=…&keys=…` like any other asset of the app, so it
	// renders the same on WebView2, WKWebView and WebKitGTK and needs no IPC.
	// A blob: URL created in the main window, as used before, is not
	// loadable from a second webview on every platform (WebKitGTK showed an
	// empty window).
	import { generateManualParts, parseManualQuery } from '$lib/components/layout/helpContent';

	const { locale, liveShortcuts } = parseManualQuery(
		typeof location !== 'undefined' ? location.search : '',
	);
	const parts = generateManualParts(locale, liveShortcuts);
	const style = `<style>${parts.css}</style>`;

	$effect(() => {
		document.documentElement.lang = parts.lang;
	});
</script>

<svelte:head>
	<title>{parts.title}</title>
	{@html style}
</svelte:head>

{@html parts.body}
