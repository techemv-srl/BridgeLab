<script lang="ts">
	import type { ValidationIssue } from '$lib/ipc/validation';
	import { t, subscribeLocale } from '$lib/i18n';
	import { ISSUE_PAGE, keyedIssues, listedIssues } from './issue-list';
	import { capNoteText } from '$lib/plugin-notices';
	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }
	/** The issue's text. The note on plugin packs over the cap is about the
	 *  app, not the message, and is shown in the user's language. */
	function messageOf(issue: ValidationIssue): string {
		void localeVersion;
		return issue.rule_id === 'PLUGIN-CAP' ? capNoteText(issue.message) : issue.message;
	}

	interface Props {
		issues: ValidationIssue[];
		errorCount: number;
		warningCount: number;
		infoCount: number;
		/** The message changed since this report was made. */
		stale?: boolean;
		onIssueClick?: (issue: ValidationIssue) => void;
	}

	let { issues, errorCount, warningCount, infoCount, stale = false, onIssueClick }: Props = $props();

	let filterSeverity = $state<string>('all');
	let sortField = $state<string>('severity');

	// If a re-validate clears the last issue of the filtered severity, the
	// badge disappears while the filter stays active — dead-ending on
	// "no issues match the filter". Fall back to All automatically.
	$effect(() => {
		if (filterSeverity !== 'all' && !issues.some((i) => i.severity === filterSeverity)) {
			filterSeverity = 'all';
		}
	});

	let rows = $derived(listedIssues(issues, filterSeverity, sortField));

	// A page at a time (see ISSUE_PAGE): the badges, the filter and the
	// order still cover the whole report. The page count starts over
	// whenever the list changes (a new report after an edit, another filter
	// or sort), so a re-validation never redraws the thousands of rows the
	// user had scrolled through before.
	let limit = $derived.by(() => {
		void rows;
		return ISSUE_PAGE;
	});
	let shown = $derived(keyedIssues(rows.slice(0, limit)));
	let moreEl = $state<HTMLElement | null>(null);
	$effect(() => {
		const el = moreEl;
		if (!el || typeof IntersectionObserver === 'undefined') return;
		const io = new IntersectionObserver((entries) => {
			if (entries.some((e) => e.isIntersecting)) limit += ISSUE_PAGE;
		});
		io.observe(el);
		return () => io.disconnect();
	});

	function severityIcon(severity: string): string {
		switch (severity) {
			case 'error': return '\u2716';
			case 'warning': return '\u26A0';
			case 'info': return '\u2139';
			default: return '\u2022';
		}
	}
</script>

<div class="validation-panel">
	<div class="validation-header">
		<div class="validation-summary">
			<button
				class="summary-badge"
				class:active={filterSeverity === 'all'}
				onclick={() => { filterSeverity = 'all'; }}
			>
				{tr('validation.all')} ({issues.length})
			</button>
			{#if errorCount > 0}
				<button
					class="summary-badge error"
					class:active={filterSeverity === 'error'}
					onclick={() => { filterSeverity = filterSeverity === 'error' ? 'all' : 'error'; }}
				>
					{severityIcon('error')} {errorCount}
				</button>
			{/if}
			{#if warningCount > 0}
				<button
					class="summary-badge warning"
					class:active={filterSeverity === 'warning'}
					onclick={() => { filterSeverity = filterSeverity === 'warning' ? 'all' : 'warning'; }}
				>
					{severityIcon('warning')} {warningCount}
				</button>
			{/if}
			{#if infoCount > 0}
				<button
					class="summary-badge info"
					class:active={filterSeverity === 'info'}
					onclick={() => { filterSeverity = filterSeverity === 'info' ? 'all' : 'info'; }}
				>
					{severityIcon('info')} {infoCount}
				</button>
			{/if}
		</div>
		<div class="validation-actions">
			<select class="sort-select" bind:value={sortField}>
				<option value="severity">{tr('validation.bySeverity')}</option>
				<option value="segment">{tr('validation.bySegment')}</option>
			</select>
		</div>
	</div>

	{#if stale}
		<div class="stale-note" role="status">{tr('validation.stale')}</div>
	{/if}

	<div class="validation-list">
		{#if rows.length === 0}
			<div class="validation-empty">
				{#if issues.length === 0}
					{tr('validation.noIssues')}
				{:else}
					{tr('validation.noMatch')}
				{/if}
			</div>
		{:else}
			{#each shown as { key, issue } (key)}
				<button
					class="issue-row {issue.severity}"
					onclick={() => onIssueClick?.(issue)}
				>
					<span class="issue-icon">{severityIcon(issue.severity)}</span>
					<span
						class="issue-location"
						title={issue.segment_type ? `${issue.segment_type}${issue.field_position ? `-${issue.field_position}` : ''}` : ''}
					>
						{#if issue.segment_type}
							{issue.segment_type}{issue.field_position ? `-${issue.field_position}` : ''}
							<!-- Which occurrence: numbered like the tree's rows, so two
							     identical issues on two OBX can be told apart. -->
							{#if issue.segment_idx !== null && issue.segment_idx !== undefined}
								<span class="issue-seg">({issue.segment_idx})</span>
							{/if}
						{:else}
							—
						{/if}
					</span>
					<span class="issue-message" title={messageOf(issue)}>{messageOf(issue)}</span>
					<span class="issue-rule" title={issue.rule_id}>{issue.rule_id}</span>
				</button>
			{/each}
			{#if rows.length > shown.length}
				<button class="validation-more" bind:this={moreEl} onclick={() => (limit += ISSUE_PAGE)}>
					{tr('validation.more', { shown: shown.length, total: rows.length })}
				</button>
			{/if}
		{/if}
	</div>
</div>

<style>
	.validation-panel {
		display: flex;
		flex-direction: column;
		height: 100%;
		background-color: var(--color-bg-secondary);
		font-size: 12px;
	}

	.stale-note {
		padding: 3px 8px;
		font-size: 11px;
		color: var(--color-warning, #d7ba7d);
		border-bottom: 1px solid var(--color-border);
		background-color: var(--color-bg-tertiary);
	}

	.issue-seg {
		opacity: var(--text-dim, 0.6);
		font-weight: normal;
	}

	.validation-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 4px 8px;
		border-bottom: 1px solid var(--color-border);
		flex-shrink: 0;
	}

	.validation-summary {
		display: flex;
		gap: 4px;
	}

	.summary-badge {
		padding: 2px 8px;
		border: 1px solid var(--color-border);
		border-radius: 10px;
		background: none;
		color: var(--color-text-secondary);
		font-size: 11px;
		font-family: inherit;
		cursor: pointer;
	}

	.summary-badge.active {
		background-color: var(--color-bg-tertiary);
		color: var(--color-text-primary);
	}

	.summary-badge.error { color: var(--color-error); border-color: var(--color-error); }
	.summary-badge.warning { color: var(--color-warning); border-color: var(--color-warning); }
	.summary-badge.info { color: var(--color-accent); border-color: var(--color-accent); }

	.validation-actions {
		display: flex;
		gap: 4px;
	}

	.sort-select {
		padding: 2px 4px;
		border: 1px solid var(--color-border);
		border-radius: 3px;
		background-color: var(--color-bg-tertiary);
		color: var(--color-text-primary);
		font-size: 11px;
		font-family: inherit;
	}

	.validation-list {
		flex: 1;
		overflow-y: auto;
	}

	.validation-empty {
		padding: 16px;
		text-align: center;
		color: var(--color-text-secondary);
		font-style: italic;
	}

	.validation-more {
		margin: 6px 8px;
		padding: 2px 8px;
		background: none;
		border: 1px solid var(--color-border);
		border-radius: 3px;
		color: var(--color-text-secondary);
		font-size: 12px;
		font-family: inherit;
		cursor: pointer;
	}

	.validation-more:hover {
		color: var(--color-accent);
	}

	.issue-row {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		padding: 4px 8px;
		background: none;
		border: none;
		border-bottom: 1px solid var(--color-border);
		color: var(--color-text-primary);
		font-size: 12px;
		font-family: inherit;
		text-align: left;
		cursor: pointer;
	}

	.issue-row:hover {
		background-color: var(--color-bg-tertiary);
	}

	.issue-icon {
		flex-shrink: 0;
		width: 16px;
		text-align: center;
	}

	.issue-row.error .issue-icon { color: var(--color-error); }
	.issue-row.warning .issue-icon { color: var(--color-warning); }
	.issue-row.info .issue-icon { color: var(--color-accent); }

	/* "PID-5" fits in 60px; a FHIR path such as
	   Bundle.entry[0].resource.destination[0].endpoint does not, and must
	   not paint over the message beside it. It grows to a cap, then
	   ellipsises; the full path is in the tooltip. */
	.issue-location {
		flex: 0 1 auto;
		min-width: 60px;
		max-width: 280px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		font-weight: 600;
		color: var(--color-segment);
		font-family: 'JetBrains Mono', monospace;
		font-size: 11px;
	}

	.issue-message {
		flex: 1;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.issue-rule {
		flex-shrink: 0;
		font-size: 10px;
		color: var(--color-text-secondary);
		font-family: 'JetBrains Mono', monospace;
	}
</style>
