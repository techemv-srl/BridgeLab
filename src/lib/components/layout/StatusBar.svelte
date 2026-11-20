<script lang="ts">
	import { formatSize } from '$lib/large-content';
	import { t, subscribeLocale } from '$lib/i18n';
	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		messageType?: string;
		version?: string;
		format?: string;
		segmentCount?: number;
		fileSize?: number;
		truncationCount?: number;
		cursorLine?: number;
		cursorColumn?: number;
		isModified?: boolean;
		/** The tab's text is still being read or parsed (a restored tab):
		 *  not "Not parsed", which asked for F6 while it was on its way. */
		loading?: boolean;
		/** False with no tab open: nothing to describe, so the parse state and
		 *  the cursor position are left out. */
		hasTab?: boolean;
		/** Why the current text did not parse, when what is shown comes
		 *  from an earlier version of it. */
		parseStale?: string | null;
		/** Validation summary from the last run; null = not validated yet. */
		errorCount?: number | null;
		warningCount?: number | null;
		/** Click on the error/warning summary opens the validation panel. */
		onShowValidation?: () => void;
		/** Click on the truncation badge expands all truncated fields. */
		onExpandAll?: () => void;
	}

	let {
		messageType = '',
		version = '',
		format = '',
		segmentCount = 0,
		fileSize = 0,
		truncationCount = 0,
		cursorLine = 1,
		cursorColumn = 1,
		isModified = false,
		loading = false,
		hasTab = true,
		parseStale = null,
		errorCount = null,
		warningCount = null,
		onShowValidation,
		onExpandAll,
	}: Props = $props();

	/** "1 segment" / "3 segments": the key with a `.one` variant for 1. */
	function counted(key: string, count: number): string {
		return tr(count === 1 ? `${key}.one` : key, { count });
	}

	// Decimals in the interface language ("1,5 MB" in Italian).
	const formatFileSize = formatSize;
</script>

<div class="status-bar">
	<div class="status-left">
		{#if format}
			<span class="status-item format">{format}</span>
		{:else if loading}
			<span class="status-item hint">{tr('status.loading')}</span>
		{:else if hasTab}
			<span class="status-item hint">{tr('status.notParsed')}</span>
		{/if}
		{#if parseStale}
			<span class="status-item stale" title={parseStale}>⚠ {tr('status.parseStale')}</span>
		{/if}
		{#if messageType}
			<span class="status-item">{messageType}</span>
		{/if}
		{#if version}
			<span class="status-item" title={tr('xsd.version')}>v{version}</span>
		{/if}
		{#if segmentCount > 0}
			<!-- A FHIR resource has elements, not segments. -->
			<span class="status-item">{counted(format.startsWith('FHIR') ? 'status.elements' : 'status.segments', segmentCount)}</span>
		{/if}
		{#if truncationCount > 0}
			<button
				class="status-item truncated clickable"
				title={tr('status.truncatedHint')}
				onclick={() => onExpandAll?.()}
			>
				{counted('status.truncated', truncationCount)}
			</button>
		{/if}
		{#if errorCount !== null || warningCount !== null}
			<button
				class="status-item validation clickable"
				class:has-errors={(errorCount ?? 0) > 0}
				title={tr('status.validationHint')}
				onclick={() => onShowValidation?.()}
			>
				✖ {errorCount ?? 0} ⚠ {warningCount ?? 0}
			</button>
		{/if}
	</div>
	<div class="status-right">
		{#if isModified}
			<span class="status-item modified" title={tr('editor.modified')}>●</span>
		{/if}
		{#if fileSize > 0}
			<span class="status-item">{formatFileSize(fileSize)}</span>
		{/if}
		{#if hasTab}
			<span class="status-item">{tr('status.line', { line: cursorLine })}, {tr('status.column', { col: cursorColumn })}</span>
		{/if}
	</div>
</div>

<style>
	.status-bar {
		display: flex;
		justify-content: space-between;
		align-items: center;
		height: 24px;
		padding: 0 12px;
		background-color: var(--color-accent);
		color: var(--color-bg-primary);
		font-size: 11px;
		font-family: 'JetBrains Mono', monospace;
		flex-shrink: 0;
	}

	.status-left,
	.status-right {
		display: flex;
		gap: 12px;
		align-items: center;
	}

	.status-item {
		opacity: 0.9;
	}

	.status-item.hint {
		opacity: 0.7;
		font-style: italic;
	}

	.status-item.format {
		font-weight: 700;
	}

	.status-item.stale {
		opacity: 1;
		font-weight: 600;
	}

	.status-item.truncated {
		opacity: 1;
		font-weight: 600;
	}

	.status-item.modified {
		opacity: 1;
	}

	.clickable {
		background: none;
		border: none;
		color: inherit;
		font: inherit;
		padding: 2px 6px;
		margin: 0 -6px;
		border-radius: 3px;
		cursor: pointer;
	}

	.clickable:hover {
		background: rgba(0, 0, 0, 0.15);
	}

	.status-item.validation {
		opacity: 1;
		font-weight: 600;
	}

	.status-item.validation.has-errors {
		text-decoration: underline;
	}
</style>
