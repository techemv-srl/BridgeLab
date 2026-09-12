<script lang="ts">
	import {
		listFhirRules, saveFhirRules, checkFhirRule, testFhirRule, blankRule,
		type FhirRule, type FhirCheck, type FhirRuleTest, type Severity,
	} from '$lib/ipc/fhirRules';
	import { parseUpgradeError } from '$lib/ipc/licensing';
	import { t } from '$lib/i18n';

	interface Props {
		/** Message to test rules against; null when no FHIR resource is open. */
		messageId: string | null;
		onClose: () => void;
	}

	let { messageId, onClose }: Props = $props();

	let rules = $state<FhirRule[]>([]);
	let packPath = $state('');
	let selected = $state<number | null>(null);
	let loading = $state(true);
	let saving = $state(false);
	let error = $state<string | null>(null);
	let ruleError = $state<string | null>(null);
	let testResult = $state<FhirRuleTest | null>(null);
	let dirty = $state(false);

	const CHECK_TYPES: FhirCheck['type'][] = [
		'not_empty', 'cardinality', 'regex', 'one_of', 'contains', 'min_length', 'max_length',
	];

	const SEVERITIES: Severity[] = ['error', 'warning', 'info'];

	/** Starting points that show what each rule form is for. */
	const PRESETS: { label: string; rule: () => FhirRule }[] = [
		{
			label: t('fhirRules.preset.identifier'),
			rule: () => ({
				...blankRule(),
				rule_id: 'patient-has-identifier',
				resource: 'Patient',
				expression: 'identifier.exists()',
				message: t('fhirRules.preset.identifierMsg'),
			}),
		},
		{
			label: t('fhirRules.preset.phone'),
			rule: () => ({
				...blankRule(),
				rule_id: 'phone-format',
				severity: 'warning',
				resource: 'Patient',
				expression: null,
				path: "telecom.where(system = 'phone').value",
				check: { type: 'regex', pattern: '^[+0-9 ()-]{6,}$' },
				message: t('fhirRules.preset.phoneMsg'),
			}),
		},
		{
			label: t('fhirRules.preset.cardinality'),
			rule: () => ({
				...blankRule(),
				rule_id: 'one-name',
				resource: 'Patient',
				expression: null,
				path: 'name',
				check: { type: 'cardinality', min: 1, max: null },
				message: t('fhirRules.preset.cardinalityMsg'),
			}),
		},
	];

	const current = $derived(selected !== null ? rules[selected] : null);
	/** Which of the two forms the selected rule uses. */
	const mode = $derived(current?.path !== null ? 'path' : 'expression');

	$effect(() => {
		load();
	});

	async function load() {
		loading = true;
		try {
			const set = await listFhirRules();
			rules = set.rules;
			packPath = set.path;
			selected = rules.length > 0 ? 0 : null;
		} catch (e) {
			error = String(e);
		}
		loading = false;
	}

	function addRule(rule: FhirRule) {
		rules = [...rules, rule];
		selected = rules.length - 1;
		testResult = null;
		ruleError = null;
		dirty = true;
	}

	function removeRule(index: number | null) {
		if (index === null) return;
		rules = rules.filter((_, i) => i !== index);
		selected = rules.length === 0 ? null : Math.min(index, rules.length - 1);
		testResult = null;
		dirty = true;
	}

	/** Switching form clears the other one, so a rule never carries both. */
	function setMode(next: 'expression' | 'path') {
		if (!current || selected === null) return;
		const updated: FhirRule = next === 'expression'
			? { ...current, expression: current.expression ?? '', path: null, check: null }
			: {
				...current,
				expression: null,
				path: current.path ?? '',
				check: current.check ?? { type: 'not_empty' },
			};
		update(updated);
	}

	function setCheckType(type: FhirCheck['type']) {
		if (!current) return;
		const check: FhirCheck =
			type === 'regex' ? { type, pattern: '' }
			: type === 'one_of' ? { type, values: [] }
			: type === 'contains' ? { type, value: '' }
			: type === 'min_length' ? { type, min: 1 }
			: type === 'max_length' ? { type, max: 64 }
			: type === 'cardinality' ? { type, min: 1, max: null }
			: { type: 'not_empty' };
		update({ ...current, check });
	}

	function update(rule: FhirRule) {
		if (selected === null) return;
		rules = rules.map((r, i) => (i === selected ? rule : r));
		testResult = null;
		ruleError = null;
		dirty = true;
	}

	/** `one_of` is edited as one value per line. */
	function oneOfText(check: FhirCheck | null): string {
		return check?.type === 'one_of' ? check.values.join('\n') : '';
	}

	function setOneOf(text: string) {
		if (!current) return;
		const values = text.split('\n').map((v) => v.trim()).filter((v) => v.length > 0);
		update({ ...current, check: { type: 'one_of', values } });
	}

	/** An empty number input means "no bound", not zero. */
	function numberOrNull(value: string): number | null {
		const trimmed = value.trim();
		if (trimmed === '') return null;
		const n = Number(trimmed);
		return Number.isFinite(n) ? n : null;
	}

	async function validateCurrent() {
		if (!current) return;
		ruleError = null;
		try {
			await checkFhirRule(current);
		} catch (e) {
			ruleError = describe(e);
		}
	}

	async function runTest() {
		if (!current || !messageId) return;
		ruleError = null;
		testResult = null;
		try {
			testResult = await testFhirRule(current, messageId);
		} catch (e) {
			ruleError = describe(e);
		}
	}

	async function save() {
		saving = true;
		error = null;
		ruleError = null;
		try {
			const set = await saveFhirRules(rules);
			rules = set.rules;
			packPath = set.path;
			dirty = false;
		} catch (e) {
			ruleError = describe(e);
		}
		saving = false;
	}

	function describe(e: unknown): string {
		const up = parseUpgradeError(e);
		return up ? t('upgrade.required', { tier: up.tier }) : String(e);
	}

	function preview(value: unknown): string {
		if (typeof value === 'string') return value;
		return JSON.stringify(value);
	}
</script>

<div class="modal-backdrop" role="presentation" onclick={onClose}>
	<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
	<div class="modal rules-modal" onclick={(e) => e.stopPropagation()}>
		<div class="modal-header">
			<h2>{t('fhirRules.title')}</h2>
			<button class="close-btn" onclick={onClose} aria-label="Close">×</button>
		</div>

		<div class="modal-body">
			<p class="intro">{t('fhirRules.intro')}</p>

			{#if error}
				<div class="banner error">{error}</div>
			{/if}

			{#if loading}
				<div class="banner">{t('fhirRules.loading')}</div>
			{:else}
				<div class="rules-layout">
					<aside class="rules-list">
						<div class="list-head">
							<span>{t('fhirRules.rules')} ({rules.length})</span>
						</div>
						{#each rules as rule, i (i)}
							<button
								class="rule-row"
								class:active={selected === i}
								onclick={() => { selected = i; testResult = null; ruleError = null; }}
							>
								<span class="sev sev-{rule.severity}"></span>
								<span class="rule-name">{rule.rule_id || t('fhirRules.untitled')}</span>
								{#if rule.resource}<span class="rule-res">{rule.resource}</span>{/if}
							</button>
						{/each}
						{#if rules.length === 0}
							<p class="list-empty">{t('fhirRules.none')}</p>
						{/if}
						<div class="list-actions">
							<span class="presets-label">{t('fhirRules.addFrom')}</span>
							{#each PRESETS as preset}
								<button class="preset-chip" onclick={() => addRule(preset.rule())}>
									{preset.label}
								</button>
							{/each}
							<button class="preset-chip blank" onclick={() => addRule(blankRule())}>
								{t('fhirRules.blank')}
							</button>
						</div>
					</aside>

					<section class="rule-editor">
						{#if current && selected !== null}
							<div class="field-row">
								<label class="field">
									<span>{t('fhirRules.id')}</span>
									<input
										value={current.rule_id}
										oninput={(e) => update({ ...current, rule_id: e.currentTarget.value })}
										placeholder="patient-has-identifier"
									/>
								</label>
								<label class="field narrow">
									<span>{t('fhirRules.severity')}</span>
									<select
										value={current.severity}
										onchange={(e) => update({ ...current, severity: e.currentTarget.value as Severity })}
									>
										{#each SEVERITIES as s}<option value={s}>{s}</option>{/each}
									</select>
								</label>
								<label class="field narrow">
									<span>{t('fhirRules.resource')}</span>
									<input
										value={current.resource ?? ''}
										oninput={(e) => update({ ...current, resource: e.currentTarget.value.trim() || null })}
										placeholder={t('fhirRules.anyResource')}
									/>
								</label>
							</div>

							<div class="mode-row">
								<button class="mode-btn" class:active={mode === 'expression'} onclick={() => setMode('expression')}>
									{t('fhirRules.modeExpression')}
								</button>
								<button class="mode-btn" class:active={mode === 'path'} onclick={() => setMode('path')}>
									{t('fhirRules.modePath')}
								</button>
							</div>

							{#if mode === 'expression'}
								<label class="field">
									<span>{t('fhirRules.expression')}</span>
									<input
										class="mono"
										value={current.expression ?? ''}
										oninput={(e) => update({ ...current, expression: e.currentTarget.value })}
										onblur={validateCurrent}
										placeholder="identifier.exists()"
									/>
								</label>
								<p class="hint">{t('fhirRules.expressionHint')}</p>
							{:else}
								<label class="field">
									<span>{t('fhirRules.path')}</span>
									<input
										class="mono"
										value={current.path ?? ''}
										oninput={(e) => update({ ...current, path: e.currentTarget.value })}
										onblur={validateCurrent}
										placeholder="telecom.where(system = 'phone').value"
									/>
								</label>

								<div class="field-row">
									<label class="field narrow">
										<span>{t('fhirRules.check')}</span>
										<select
											value={current.check?.type ?? 'not_empty'}
											onchange={(e) => setCheckType(e.currentTarget.value as FhirCheck['type'])}
										>
											{#each CHECK_TYPES as ct}
												<option value={ct}>{t(`fhirRules.check.${ct}`)}</option>
											{/each}
										</select>
									</label>

									{#if current.check?.type === 'regex'}
										<label class="field">
											<span>{t('fhirRules.pattern')}</span>
											<input
												class="mono"
												value={current.check.pattern}
												oninput={(e) => update({ ...current, check: { type: 'regex', pattern: e.currentTarget.value } })}
											/>
										</label>
									{:else if current.check?.type === 'contains'}
										<label class="field">
											<span>{t('fhirRules.substring')}</span>
											<input
												value={current.check.value}
												oninput={(e) => update({ ...current, check: { type: 'contains', value: e.currentTarget.value } })}
											/>
										</label>
									{:else if current.check?.type === 'min_length'}
										<label class="field narrow">
											<span>{t('fhirRules.min')}</span>
											<input
												type="number"
												value={current.check.min}
												oninput={(e) => update({ ...current, check: { type: 'min_length', min: Number(e.currentTarget.value) || 0 } })}
											/>
										</label>
									{:else if current.check?.type === 'max_length'}
										<label class="field narrow">
											<span>{t('fhirRules.max')}</span>
											<input
												type="number"
												value={current.check.max}
												oninput={(e) => update({ ...current, check: { type: 'max_length', max: Number(e.currentTarget.value) || 0 } })}
											/>
										</label>
									{:else if current.check?.type === 'cardinality'}
										<label class="field narrow">
											<span>{t('fhirRules.min')}</span>
											<input
												type="number"
												value={current.check.min ?? ''}
												oninput={(e) => update({ ...current, check: { type: 'cardinality', min: numberOrNull(e.currentTarget.value), max: current.check?.type === 'cardinality' ? current.check.max : null } })}
											/>
										</label>
										<label class="field narrow">
											<span>{t('fhirRules.max')}</span>
											<input
												type="number"
												value={current.check.max ?? ''}
												oninput={(e) => update({ ...current, check: { type: 'cardinality', min: current.check?.type === 'cardinality' ? current.check.min : null, max: numberOrNull(e.currentTarget.value) } })}
											/>
										</label>
									{/if}
								</div>

								{#if current.check?.type === 'one_of'}
									<label class="field">
										<span>{t('fhirRules.allowed')}</span>
										<textarea
											class="mono"
											rows="3"
											value={oneOfText(current.check)}
											oninput={(e) => setOneOf(e.currentTarget.value)}
										></textarea>
									</label>
								{/if}
								<p class="hint">{t('fhirRules.pathHint')}</p>
							{/if}

							<label class="field">
								<span>{t('fhirRules.message')}</span>
								<input
									value={current.message}
									oninput={(e) => update({ ...current, message: e.currentTarget.value })}
									placeholder={t('fhirRules.messagePlaceholder')}
								/>
							</label>

							{#if ruleError}
								<div class="banner error">{ruleError}</div>
							{/if}

							<div class="editor-actions">
								<button class="btn" onclick={runTest} disabled={!messageId}>
									{t('fhirRules.test')}
								</button>
								{#if !messageId}
									<span class="hint inline">{t('fhirRules.testNeedsResource')}</span>
								{/if}
								<button class="btn danger" onclick={() => removeRule(selected)}>
									{t('fhirRules.delete')}
								</button>
							</div>

							{#if testResult}
								<div class="test-result" class:pass={testResult.passed && testResult.applied}>
									{#if !testResult.applied}
										<!-- A rule scoped to a type the open resource is not
										     would pass for the wrong reason; say so. -->
										<strong>{t('fhirRules.notApplied')}</strong>
									{:else if testResult.passed}
										<strong>{t('fhirRules.passed')}</strong>
									{:else}
										<strong>{t('fhirRules.failed')}</strong>
										<ul>
											{#each testResult.issues as issue}
												<li>{issue.message} <code>{issue.path}</code></li>
											{/each}
										</ul>
									{/if}
									{#if mode === 'path' && testResult.applied}
										<div class="selected-values">
											<span>{t('fhirRules.selected')} ({testResult.selected.length})</span>
											{#each testResult.selected.slice(0, 8) as value}
												<code>{preview(value)}</code>
											{/each}
										</div>
									{/if}
								</div>
							{/if}
						{:else}
							<p class="editor-empty">{t('fhirRules.pickOne')}</p>
						{/if}
					</section>
				</div>

				<p class="pack-path" title={packPath}>{t('fhirRules.storedIn')} <code>{packPath}</code></p>
			{/if}
		</div>

		<div class="modal-footer">
			<button class="btn" onclick={onClose}>{t('common.close')}</button>
			<button class="btn btn-primary" onclick={save} disabled={saving || !dirty}>
				{saving ? t('fhirRules.saving') : t('fhirRules.save')}
			</button>
		</div>
	</div>
</div>

<style>
	.modal-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 1000; }
	.modal { background: var(--color-bg-secondary); border: 1px solid var(--color-border); border-radius: 8px; display: flex; flex-direction: column; max-height: 90vh; box-shadow: 0 8px 32px rgba(0,0,0,0.4); }
	.rules-modal { width: min(1000px, 94vw); }
	.modal-header { display: flex; align-items: center; justify-content: space-between; padding: 12px 16px; border-bottom: 1px solid var(--color-border); }
	.modal-header h2 { margin: 0; font-size: 14px; }
	.close-btn { background: none; border: none; color: var(--color-text-secondary); font-size: 20px; cursor: pointer; line-height: 1; }
	.modal-body { padding: 12px 16px; overflow-y: auto; font-size: 12px; }
	.modal-footer { display: flex; justify-content: flex-end; gap: 8px; padding: 10px 16px; border-top: 1px solid var(--color-border); }

	.intro { margin: 0 0 10px; color: var(--color-text-secondary); }
	.banner { padding: 8px 10px; border-radius: 4px; background: var(--color-bg-tertiary); margin-bottom: 8px; }
	.banner.error { background: rgba(220, 60, 60, 0.15); color: var(--color-error, #e06c6c); }

	.rules-layout { display: grid; grid-template-columns: 250px 1fr; gap: 14px; align-items: start; }
	.rules-list { border: 1px solid var(--color-border); border-radius: 6px; overflow: hidden; }
	.list-head { padding: 6px 10px; background: var(--color-bg-tertiary); font-weight: 600; font-size: 11px; }
	.rule-row { display: flex; align-items: center; gap: 6px; width: 100%; padding: 6px 10px; background: none; border: none; border-bottom: 1px solid var(--color-border); color: var(--color-text-primary); font-family: inherit; font-size: 12px; text-align: left; cursor: pointer; }
	.rule-row.active { background: var(--color-bg-tertiary); }
	.rule-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
	.rule-res { font-size: 10px; color: var(--color-text-secondary); }
	.sev { width: 8px; height: 8px; border-radius: 50%; flex: none; }
	.sev-error { background: #e06c6c; }
	.sev-warning { background: #e0b36c; }
	.sev-info { background: #6ca8e0; }
	.list-empty { padding: 10px; margin: 0; color: var(--color-text-secondary); }
	.list-actions { display: flex; flex-wrap: wrap; gap: 4px; padding: 8px 10px; align-items: center; }
	.presets-label { font-size: 10px; color: var(--color-text-secondary); width: 100%; }
	.preset-chip { padding: 3px 7px; border: 1px solid var(--color-border); border-radius: 10px; background: none; color: var(--color-text-primary); font-family: inherit; font-size: 10px; cursor: pointer; }
	.preset-chip.blank { border-style: dashed; }

	.rule-editor { display: flex; flex-direction: column; gap: 8px; }
	.field { display: flex; flex-direction: column; gap: 3px; flex: 1; }
	.field > span { font-size: 10px; color: var(--color-text-secondary); }
	.field.narrow { flex: 0 0 130px; }
	.field-row { display: flex; gap: 8px; flex-wrap: wrap; align-items: flex-end; }
	input, select, textarea { padding: 5px 7px; border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-bg-primary); color: var(--color-text-primary); font-family: inherit; font-size: 12px; }
	.mono { font-family: var(--font-mono, monospace); }

	.mode-row { display: flex; gap: 4px; }
	.mode-btn { padding: 4px 10px; border: 1px solid var(--color-border); border-radius: 4px; background: none; color: var(--color-text-secondary); font-family: inherit; font-size: 11px; cursor: pointer; }
	.mode-btn.active { border-color: var(--color-accent); color: var(--color-text-primary); }

	.hint { margin: 0; font-size: 10px; color: var(--color-text-secondary); }
	.hint.inline { align-self: center; }
	.editor-actions { display: flex; gap: 8px; align-items: center; }
	.btn { padding: 5px 12px; border: 1px solid var(--color-border); border-radius: 4px; background: none; color: var(--color-text-primary); font-family: inherit; font-size: 12px; cursor: pointer; }
	.btn-primary { background: var(--color-accent); border-color: var(--color-accent); color: #fff; }
	.btn.danger { margin-left: auto; color: #e06c6c; }
	.btn:disabled { opacity: 0.5; cursor: default; }

	.test-result { padding: 8px 10px; border-radius: 4px; background: rgba(220, 60, 60, 0.12); }
	.test-result.pass { background: rgba(80, 180, 120, 0.15); }
	.test-result ul { margin: 4px 0 0; padding-left: 18px; }
	.test-result code { font-family: var(--font-mono, monospace); font-size: 11px; }
	.selected-values { margin-top: 6px; display: flex; flex-wrap: wrap; gap: 4px; align-items: center; font-size: 10px; color: var(--color-text-secondary); }
	.selected-values code { padding: 1px 5px; background: var(--color-bg-tertiary); border-radius: 3px; color: var(--color-text-primary); }

	.editor-empty { color: var(--color-text-secondary); }
	.pack-path { margin: 10px 0 0; font-size: 10px; color: var(--color-text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

	@media (max-width: 720px) {
		.rules-layout { grid-template-columns: 1fr; }
	}
</style>
