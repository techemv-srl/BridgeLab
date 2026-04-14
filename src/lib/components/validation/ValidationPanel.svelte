<script lang="ts">
	import type { ValidationIssue } from '$lib/ipc/validation';
	import { t, subscribeLocale } from '$lib/i18n';
	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		issues: ValidationIssue[];
		errorCount: number;
		warningCount: number;
		infoCount: number;
		onIssueClick?: (issue: ValidationIssue) => void;
	}

	let { issues, errorCount, warningCount, infoCount, onIssueClick }: Props = $props();

	let filterSeverity = $state<string>('all');
	let sortField = $state<string>('severity');

	let filteredIssues = $derived.by(() => {
		let filtered = filterSeverity === 'all'
			? issues
			: issues.filter((i) => i.severity === filterSeverity);

		return [...filtered].sort((a, b) => {
			if (sortField === 'severity') {
				const order = { error: 0, warning: 1, info: 2 };
				return (order[a.severity] ?? 3) - (order[b.severity] ?? 3);
			}
			if (sortField === 'segment') {
				return (a.segment_idx ?? 999) - (b.segment_idx ?? 999);
			}
			return 0;
		});
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
				All ({issues.length})
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
				<option value="severity">By Severity</option>
				<option value="segment">By Segment</option>
			</select>
		</div>
	</div>

	<div class="validation-list">
		{#if filteredIssues.length === 0}
			<div class="validation-empty">
				{#if issues.length === 0}
					No validation issues found
				{:else}
					No issues match the current filter
				{/if}
			</div>
		{:else}
			{#each filteredIssues as issue (issue.rule_id + (issue.segment_idx ?? '') + (issue.field_position ?? ''))}
				<button
					class="issue-row {issue.severity}"
					onclick={() => onIssueClick?.(issue)}
				>
					<span class="issue-icon">{severityIcon(issue.severity)}</span>
					<span class="issue-location">
						{#if issue.segment_type}
							{issue.segment_type}{issue.field_position ? `-${issue.field_position}` : ''}
						{:else}
							—
						{/if}
					</span>
					<span class="issue-message">{issue.message}</span>
					<span class="issue-rule">{issue.rule_id}</span>
				</button>
			{/each}
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

	.issue-location {
		flex-shrink: 0;
		width: 60px;
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
