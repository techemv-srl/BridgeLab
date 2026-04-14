<script lang="ts">
	import { evaluateFhirPath, type FhirPathResult } from '$lib/ipc/fhirpath';

	interface Props {
		messageId: string;
	}

	let { messageId }: Props = $props();

	let expression = $state('');
	let result = $state<FhirPathResult | null>(null);
	let evaluating = $state(false);
	let history = $state<string[]>([]);

	// Common examples
	const examples = [
		"Patient.name.family",
		"Patient.name.given",
		"Patient.name[0].family",
		"Patient.telecom.where(system = 'email').value",
		"Bundle.entry.count()",
		"Bundle.entry.where(resource.resourceType = 'Patient').count()",
		"Bundle.entry.select(resource.resourceType).distinct()",
		"Observation.valueQuantity.value",
	];

	async function evaluate() {
		if (!expression.trim() || !messageId) return;
		evaluating = true;
		try {
			result = await evaluateFhirPath(messageId, expression);
			if (result && !result.error && !history.includes(expression)) {
				history = [expression, ...history.slice(0, 9)];
			}
		} catch (e) {
			result = { expression, results: [], count: 0, error: String(e) };
		}
		evaluating = false;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			evaluate();
		}
	}

	function useExample(ex: string) {
		expression = ex;
		evaluate();
	}

	function formatResult(val: unknown): string {
		if (val === null || val === undefined) return 'null';
		if (typeof val === 'string') return JSON.stringify(val);
		if (typeof val === 'number' || typeof val === 'boolean') return String(val);
		return JSON.stringify(val, null, 2);
	}
</script>

<div class="fp-panel">
	<div class="fp-header">
		<span>FHIRPath Evaluator</span>
	</div>

	<div class="fp-input-area">
		<label for="fp-expr" class="input-label">Expression</label>
		<div class="input-row">
			<input
				id="fp-expr"
				bind:value={expression}
				onkeydown={handleKeydown}
				placeholder="Patient.name.family"
				class="expr-input"
			/>
			<button class="btn btn-primary" onclick={evaluate} disabled={evaluating || !expression.trim()}>
				{evaluating ? 'Eval...' : 'Evaluate'}
			</button>
		</div>

		<div class="examples-row">
			<span class="examples-label">Examples:</span>
			{#each examples.slice(0, 4) as ex}
				<button class="example-chip" onclick={() => useExample(ex)}>{ex}</button>
			{/each}
		</div>
	</div>

	<div class="fp-output">
		{#if result}
			{#if result.error}
				<div class="result-error">
					<strong>Error:</strong> {result.error}
				</div>
			{:else}
				<div class="result-summary">
					<span class="result-count">{result.count} result{result.count !== 1 ? 's' : ''}</span>
					<span class="result-expr">{result.expression}</span>
				</div>
				{#if result.count === 0}
					<div class="result-empty">No results (path returned empty set)</div>
				{:else}
					<div class="result-list">
						{#each result.results as val, i}
							<div class="result-item">
								<span class="result-idx">[{i}]</span>
								<pre class="result-value">{formatResult(val)}</pre>
							</div>
						{/each}
					</div>
				{/if}
			{/if}
		{:else}
			<div class="fp-empty">Enter a FHIRPath expression and press Enter</div>
		{/if}
	</div>

	{#if history.length > 0}
		<div class="fp-history">
			<div class="history-label">Recent:</div>
			{#each history as h}
				<button class="history-chip" onclick={() => useExample(h)}>{h}</button>
			{/each}
		</div>
	{/if}
</div>

<style>
	.fp-panel { display: flex; flex-direction: column; height: 100%; background: var(--color-bg-secondary); font-size: 12px; overflow: hidden; }
	.fp-header { padding: 6px 12px; border-bottom: 1px solid var(--color-border); font-weight: 600; font-size: 11px; color: var(--color-text-secondary); text-transform: uppercase; letter-spacing: 0.5px; flex-shrink: 0; }

	.fp-input-area { padding: 8px 12px; border-bottom: 1px solid var(--color-border); flex-shrink: 0; }
	.input-label { font-size: 10px; color: var(--color-text-secondary); display: block; margin-bottom: 3px; }
	.input-row { display: flex; gap: 6px; align-items: center; }
	.expr-input { flex: 1; padding: 5px 8px; border: 1px solid var(--color-border); border-radius: 3px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-family: 'JetBrains Mono', monospace; font-size: 12px; }
	.btn { padding: 5px 12px; border: 1px solid var(--color-border); border-radius: 3px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-family: inherit; font-size: 11px; cursor: pointer; }
	.btn-primary { background: var(--color-accent); color: var(--color-bg-primary); border-color: var(--color-accent); }
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }

	.examples-row { display: flex; flex-wrap: wrap; gap: 4px; align-items: center; margin-top: 6px; }
	.examples-label { font-size: 10px; color: var(--color-text-secondary); }
	.example-chip { padding: 2px 8px; background: var(--color-bg-tertiary); border: 1px solid var(--color-border); border-radius: 10px; color: var(--color-accent); font-family: 'JetBrains Mono', monospace; font-size: 10px; cursor: pointer; }
	.example-chip:hover { background: var(--color-border); }

	.fp-output { flex: 1; overflow-y: auto; padding: 8px 12px; }
	.fp-empty { padding: 16px; text-align: center; color: var(--color-text-secondary); font-style: italic; }

	.result-error { padding: 8px; background: var(--color-error); color: white; border-radius: 4px; font-size: 11px; }

	.result-summary { display: flex; gap: 12px; align-items: center; padding: 4px 0 8px; border-bottom: 1px solid var(--color-border); margin-bottom: 8px; }
	.result-count { padding: 2px 8px; background: var(--color-success); color: var(--color-bg-primary); border-radius: 10px; font-size: 10px; font-weight: 700; }
	.result-expr { font-family: 'JetBrains Mono', monospace; font-size: 11px; color: var(--color-text-secondary); overflow: hidden; text-overflow: ellipsis; }

	.result-empty { padding: 16px; text-align: center; color: var(--color-text-secondary); font-style: italic; }

	.result-list { display: flex; flex-direction: column; gap: 4px; }
	.result-item { display: flex; gap: 8px; padding: 4px 8px; background: var(--color-bg-tertiary); border-radius: 3px; }
	.result-idx { font-family: 'JetBrains Mono', monospace; font-size: 10px; color: var(--color-text-secondary); font-weight: 700; min-width: 30px; }
	.result-value { flex: 1; margin: 0; font-family: 'JetBrains Mono', monospace; font-size: 11px; white-space: pre-wrap; word-break: break-all; color: var(--color-text-primary); }

	.fp-history { padding: 6px 12px; border-top: 1px solid var(--color-border); display: flex; flex-wrap: wrap; gap: 4px; align-items: center; flex-shrink: 0; }
	.history-label { font-size: 10px; color: var(--color-text-secondary); }
	.history-chip { padding: 2px 8px; background: var(--color-bg-tertiary); border: 1px solid var(--color-border); border-radius: 10px; color: var(--color-text-primary); font-family: 'JetBrains Mono', monospace; font-size: 10px; cursor: pointer; }
	.history-chip:hover { background: var(--color-border); color: var(--color-accent); }
</style>
