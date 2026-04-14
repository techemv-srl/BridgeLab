<script lang="ts">
	interface Props {
		messageType?: string;
		version?: string;
		format?: string;
		segmentCount?: number;
		fileSize?: number;
		truncationCount?: number;
		cursorLine?: number;
		cursorColumn?: number;
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
	}: Props = $props();

	function formatFileSize(bytes: number): string {
		if (bytes === 0) return '';
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
	}
</script>

<div class="status-bar">
	<div class="status-left">
		{#if format}
			<span class="status-item format">{format}</span>
		{/if}
		{#if messageType}
			<span class="status-item">{messageType}</span>
		{/if}
		{#if version}
			<span class="status-item">v{version}</span>
		{/if}
		{#if segmentCount > 0}
			<span class="status-item">{segmentCount} segments</span>
		{/if}
		{#if truncationCount > 0}
			<span class="status-item truncated">{truncationCount} truncated</span>
		{/if}
	</div>
	<div class="status-right">
		{#if fileSize > 0}
			<span class="status-item">{formatFileSize(fileSize)}</span>
		{/if}
		<span class="status-item">Ln {cursorLine}, Col {cursorColumn}</span>
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

	.status-item.format {
		font-weight: 700;
	}

	.status-item.truncated {
		opacity: 1;
		font-weight: 600;
	}
</style>
