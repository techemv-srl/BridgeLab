<script lang="ts">
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
		errorCount = null,
		warningCount = null,
		onShowValidation,
		onExpandAll,
	}: Props = $props();

	function formatFileSize(bytes: number): string {
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
	}
</script>

<div class="status-bar">
	<div class="status-left">
		{#if format}
			<span class="status-item format">{format}</span>
		{:else}
			<span class="status-item hint">{tr('status.notParsed')}</span>
		{/if}
		{#if messageType}
			<span class="status-item">{messageType}</span>
		{/if}
		{#if version}
			<span class="status-item" title={tr('xsd.version')}>v{version}</span>
		{/if}
		{#if segmentCount > 0}
			<span class="status-item">{tr('status.segments', { count: segmentCount })}</span>
		{/if}
		{#if truncationCount > 0}
			<button
				class="status-item truncated clickable"
				title={tr('status.truncatedHint')}
				onclick={() => onExpandAll?.()}
			>
				{tr('status.truncated', { count: truncationCount })}
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
		<span class="status-item">{tr('status.line', { line: cursorLine })}, {tr('status.column', { col: cursorColumn })}</span>
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
