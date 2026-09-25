<script lang="ts">
	// app.css (Tailwind and its reset) is imported by the main page, not
	// here, so the /manual page keeps the manual's own styles.
	let { children } = $props();
</script>

{@render children()}
