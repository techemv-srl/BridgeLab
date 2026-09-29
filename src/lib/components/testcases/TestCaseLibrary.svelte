<script lang="ts">
	import {
		getTestCases, saveTestCase, deleteTestCase, scanTestCasesPhi, exportTestCases,
		previewTestCaseImport, importTestCases, importCounts,
		type TestCase, type PhiScan, type ImportPreview, type ConflictChoice,
	} from '$lib/ipc/testcases';
	import { parseMessage } from '$lib/ipc/parser';
	import { validateMessage, validateFhir, parseFhirMessage } from '$lib/ipc/validation';
	import { dialogStore } from '$lib/stores/dialog.svelte';
	import { parseUpgradeError, getAvailableFeatures } from '$lib/ipc/licensing';
	import { t, subscribeLocale } from '$lib/i18n';

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		currentContent?: string;
		currentLabel?: string;
		onLoad: (testCase: TestCase) => void;
		onClose: () => void;
	}

	let { currentContent = '', currentLabel = '', onLoad, onClose }: Props = $props();

	let cases = $state<TestCase[]>([]);
	let selectedId = $state<string | null>(null);
	let search = $state('');
	let mode = $state<'list' | 'edit' | 'new' | 'export' | 'import'>('list');
	let loading = $state(true);
	let loadError = $state('');
	let searchInputEl: HTMLInputElement | undefined = $state();
	let nameInputEl: HTMLInputElement | undefined = $state();

	// Edit/new form state
	let formName = $state('');
	let formDescription = $state('');
	let formCategory = $state('general');
	let formTags = $state('');
	let formContent = $state('');
	let formExpectedType = $state('');
	let formExpectedResult = $state('valid');
	// Snapshot taken when the form opens; Cancel with unsaved changes confirms.
	let formSnapshot = '';

	let formDirty = $derived(
		JSON.stringify([formName, formDescription, formCategory, formTags, formContent, formExpectedType, formExpectedResult]) !== formSnapshot
	);

	let loaded = false;
	$effect(() => {
		if (loaded || typeof window === 'undefined') return;
		loaded = true;
		load();
	});

	// Autofocus: search box in list mode, name field in the form
	$effect(() => {
		if (mode === 'list') searchInputEl?.focus();
		else if (mode === 'edit' || mode === 'new') nameInputEl?.focus();
	});

	async function load() {
		loading = true;
		loadError = '';
		try {
			cases = await getTestCases();
		} catch (e) {
			// Don't masquerade a backend error as an empty library
			cases = [];
			loadError = String(e);
		} finally {
			loading = false;
		}
	}

	let filtered = $derived.by(() => {
		if (!search.trim()) return cases;
		const q = search.toLowerCase();
		return cases.filter(c =>
			c.name.toLowerCase().includes(q) ||
			c.description.toLowerCase().includes(q) ||
			c.tags.toLowerCase().includes(q) ||
			c.category.toLowerCase().includes(q)
		);
	});

	let byCategory = $derived.by(() => {
		const map: Record<string, TestCase[]> = {};
		for (const c of filtered) {
			if (!map[c.category]) map[c.category] = [];
			map[c.category].push(c);
		}
		return Object.entries(map).sort(([a], [b]) => a.localeCompare(b));
	});

	let selected = $derived(cases.find(c => c.id === selectedId));

	function snapshotForm() {
		formSnapshot = JSON.stringify([formName, formDescription, formCategory, formTags, formContent, formExpectedType, formExpectedResult]);
	}

	function startNew() {
		formName = currentLabel || '';
		formDescription = '';
		formCategory = 'general';
		formTags = '';
		formContent = currentContent;
		formExpectedType = '';
		formExpectedResult = 'valid';
		snapshotForm();
		mode = 'new';
	}

	function startEdit(tc: TestCase) {
		formName = tc.name;
		formDescription = tc.description;
		formCategory = tc.category;
		formTags = tc.tags;
		formContent = tc.content;
		formExpectedType = tc.expected_message_type;
		formExpectedResult = tc.expected_validation_result || 'valid';
		selectedId = tc.id;
		snapshotForm();
		mode = 'edit';
	}

	async function cancelForm() {
		if (formDirty && !(await dialogStore.confirm(tr('dialog.unsavedChanges')))) return;
		mode = 'list';
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Escape') {
			e.preventDefault();
			if (mode === 'list') onClose();
			else if (mode === 'export' || mode === 'import') mode = 'list';
			else void cancelForm();
		}
	}

	// Existing categories for the datalist — free text otherwise silently
	// splits "orders" and "Orders" into two groups.
	let existingCategories = $derived([...new Set(cases.map((c) => c.category).filter(Boolean))].sort());

	async function handleSave() {
		if (!formName.trim() || !formContent.trim()) return;
		try {
			await saveTestCase({
				id: mode === 'edit' ? (selectedId ?? undefined) : undefined,
				name: formName,
				description: formDescription,
				category: formCategory || 'general',
				tags: formTags,
				content: formContent,
				expected_message_type: formExpectedType.trim(),
				expected_validation_result: formExpectedResult,
			});
			// A stored check result now describes the previous content /
			// expectations — drop it so the badge doesn't lie.
			if (mode === 'edit' && selectedId && selectedId in checkResults) {
				const { [selectedId]: _stale, ...rest } = checkResults;
				checkResults = rest;
			}
			await load();
			mode = 'list';
		} catch (e) {
			const up = parseUpgradeError(e);
			if (up) {
				await dialogStore.error(
					tr('tc.limitTitle'),
					tr('upgrade.required', { tier: up.tier }),
				);
			} else {
				await dialogStore.error(tr('dialog.saveFailed'), undefined, String(e));
			}
		}
	}

	// --- Expected-outcome checks: the "test" part of the test case library ---
	interface CheckResult { pass: boolean; detail: string; }
	let checkResults = $state<Record<string, CheckResult>>({});
	let runningAll = $state(false);

	/** True when the content is a FHIR resource rather than HL7 v2 —
	 *  the library accepts both, so the runner must route accordingly. */
	function isFhirContent(content: string): boolean {
		const t = content.trim();
		return (t.startsWith('{') && t.includes('"resourceType"')) || t.startsWith('<');
	}

	/** Parse + validate a case's content and compare against its
	 *  expectations. Parse failure counts as an invalid message. */
	async function runCheck(tc: TestCase): Promise<CheckResult> {
		let messageType = '';
		let errorCount: number | null = null;
		let parseError = '';
		try {
			if (isFhirContent(tc.content)) {
				const parsed = await parseFhirMessage(tc.content);
				messageType = parsed.message_type ?? ''; // resource type, e.g. "Patient"
				const report = await validateFhir(tc.content);
				errorCount = report.error_count;
			} else {
				const parsed = await parseMessage(tc.content);
				messageType = parsed.message_type ?? '';
				const report = await validateMessage(parsed.message_id);
				errorCount = report.error_count;
			}
		} catch (e) {
			parseError = String(e);
		}

		const problems: string[] = [];
		const expType = tc.expected_message_type.trim().toUpperCase();
		if (expType) {
			const actual = messageType.toUpperCase();
			// "ADT" matches "ADT^A01"; "ADT^A01" requires the full type.
			const ok = expType.includes('^')
				? actual === expType
				: actual === expType || actual.startsWith(expType + '^');
			if (!ok) problems.push(tr('tc.checkTypeMismatch', { expected: tc.expected_message_type, actual: messageType || '—' }));
		}
		const expResult = tc.expected_validation_result || 'valid';
		const isValid = parseError === '' && errorCount === 0;
		if (expResult === 'valid' && !isValid) {
			problems.push(parseError
				? tr('tc.checkParseFailed', { error: parseError })
				: tr('tc.checkUnexpectedErrors', { count: errorCount ?? 0 }));
		} else if (expResult === 'invalid' && isValid) {
			problems.push(tr('tc.checkUnexpectedlyValid'));
		}

		return problems.length === 0
			? { pass: true, detail: tr('tc.checkPass') }
			: { pass: false, detail: problems.join(' · ') };
	}

	async function handleRunCheck(tc: TestCase) {
		checkResults = { ...checkResults, [tc.id]: await runCheck(tc) };
	}

	async function handleRunAll() {
		runningAll = true;
		for (const tc of filtered) {
			checkResults = { ...checkResults, [tc.id]: await runCheck(tc) };
		}
		runningAll = false;
	}

	let runSummary = $derived.by(() => {
		const ids = filtered.map((c) => c.id).filter((id) => id in checkResults);
		if (ids.length === 0) return null;
		const passed = ids.filter((id) => checkResults[id].pass).length;
		return { passed, total: ids.length };
	});

	// --- Packs: share test cases as a .bltests.json file ---

	const PACK_FILTERS = () => [{ name: tr('tc.packFilter'), extensions: ['json'] }];

	// Export: the test cases in view (all, or the ones matching the search).
	let exportIds = $state<string[]>([]);
	let exportCount = $state(0);
	let exportPhi = $state<PhiScan[]>([]);
	let exportAnonymize = $state(false);
	let canMask = $state(false);
	let busy = $state(false);

	async function startExport() {
		exportIds = search.trim() ? filtered.map((c) => c.id) : [];
		exportCount = search.trim() ? filtered.length : cases.length;
		exportAnonymize = false;
		busy = true;
		try {
			const [phi, features] = await Promise.all([
				scanTestCasesPhi(exportIds),
				getAvailableFeatures().catch(() => [] as string[]),
			]);
			exportPhi = phi;
			canMask = features.includes('anonymize_mask');
			mode = 'export';
		} catch (e) {
			await dialogStore.error(tr('tc.packError'), undefined, String(e));
		} finally {
			busy = false;
		}
	}

	async function doExport() {
		const { save } = await import('@tauri-apps/plugin-dialog');
		const path = await save({ title: tr('tc.export'), defaultPath: 'bridgelab-test-cases.bltests.json', filters: PACK_FILTERS() });
		if (!path) return;
		busy = true;
		try {
			const r = await exportTestCases(exportIds, path, exportAnonymize);
			const file = fileName(path);
			mode = 'list';
			await dialogStore.info(r.anonymized
				? tr('tc.exportDoneMasked', { count: r.exported, file, masked: r.anonymized })
				: tr('tc.exportDone', { count: r.exported, file }), tr('tc.export'));
		} catch (e) {
			const up = parseUpgradeError(e);
			if (up) await dialogStore.error(tr('tc.export'), tr('upgrade.required', { tier: up.tier }));
			else await dialogStore.error(tr('tc.packError'), undefined, String(e));
		} finally {
			busy = false;
		}
	}

	// Import: preview first, conflicts resolved by the user.
	let importPath = $state('');
	let importPreview = $state<ImportPreview | null>(null);
	let importChoices = $state<Record<number, ConflictChoice>>({});
	let importTotals = $derived(importPreview ? importCounts(importPreview.items, importChoices) : { add: 0, update: 0, skip: 0 });
	let importOverLimit = $derived(importPreview?.room != null && importTotals.add > importPreview.room);

	async function startImport() {
		const { open } = await import('@tauri-apps/plugin-dialog');
		const picked = await open({ title: tr('tc.import'), multiple: false, filters: PACK_FILTERS() });
		if (typeof picked !== 'string') return;
		busy = true;
		try {
			importPreview = await previewTestCaseImport(picked);
			importPath = picked;
			importChoices = {};
			mode = 'import';
		} catch (e) {
			await dialogStore.error(tr('tc.packError'), undefined, String(e));
		} finally {
			busy = false;
		}
	}

	async function doImport() {
		busy = true;
		try {
			if (!importPreview) return;
			const r = await importTestCases(importPath, importPreview.fingerprint, importChoices);
			// Replaced test cases have new content: their check badges are stale.
			checkResults = {};
			await load();
			mode = 'list';
			await dialogStore.info(tr('tc.importDone', { add: r.added, update: r.updated, skip: r.skipped }), tr('tc.import'));
		} catch (e) {
			const up = parseUpgradeError(e);
			if (up) await dialogStore.error(tr('tc.limitTitle'), tr('upgrade.required', { tier: up.tier }));
			else await dialogStore.error(tr('tc.packError'), undefined, String(e));
		} finally {
			busy = false;
		}
	}

	function fileName(path: string): string {
		return path.split(/[\\/]/).pop() ?? path;
	}

	async function handleDelete(tc: TestCase) {
		if (!(await dialogStore.confirm(tr('dialog.deleteConfirm', { name: tc.name })))) return;
		try {
			await deleteTestCase(tc.id);
			await load();
			if (selectedId === tc.id) selectedId = null;
		} catch (e) {
			await dialogStore.error(tr('tc.delete'), undefined, String(e));
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="tc-library">
	<div class="tc-header">
		<span>{tr('tc.title')}</span>
		<div class="header-actions">
			{#if mode === 'list'}
				<button class="btn" onclick={startImport} disabled={busy}>{tr('tc.import')}</button>
				<button class="btn" onclick={startExport} disabled={busy || filtered.length === 0}
					title={search.trim() ? tr('tc.exportFilteredHint', { count: filtered.length }) : tr('tc.exportAllHint')}>
					{tr('tc.export')}
				</button>
				<button class="btn btn-primary" onclick={startNew}>
					{currentContent ? tr('tc.saveCurrent') : tr('tc.newCase')}
				</button>
			{/if}
			<button class="close-btn" onclick={onClose}>&times;</button>
		</div>
	</div>

	{#if mode === 'list'}
		<div class="tc-search">
			<input bind:this={searchInputEl} bind:value={search} placeholder={tr('tc.search')} class="search-input" />
			<button class="btn" onclick={handleRunAll} disabled={runningAll || filtered.length === 0}>
				{runningAll ? tr('tc.running') : tr('tc.runAll')}
			</button>
			{#if runSummary}
				<span class="run-summary" class:all-pass={runSummary.passed === runSummary.total}>
					{tr('tc.runSummary', { passed: runSummary.passed, total: runSummary.total })}
				</span>
			{/if}
		</div>

		<div class="tc-body">
			<div class="tc-list">
				{#if loading}
					<div class="empty">{tr('xsd.loading')}</div>
				{:else if loadError}
					<div class="empty load-error">
						<p>{loadError}</p>
						<button class="btn" onclick={load}>{tr('tc.retry')}</button>
					</div>
				{:else if cases.length === 0}
					<div class="empty">
						<p>{tr('tc.empty')}</p>
						{#if currentContent}
							<p>{tr('tc.emptyHint')}</p>
						{/if}
					</div>
				{:else if filtered.length === 0}
					<div class="empty">{tr('tc.noMatch')}</div>
				{:else}
					{#each byCategory as [category, items]}
						<div class="tc-category">{category}</div>
						{#each items as tc (tc.id)}
							<button
								class="tc-item"
								class:selected={selectedId === tc.id}
								onclick={() => { selectedId = tc.id; }}
							>
								<div class="tc-name">
									{tc.name}
									{#if checkResults[tc.id]}
										<span class="check-badge" class:pass={checkResults[tc.id].pass} class:fail={!checkResults[tc.id].pass}
											title={checkResults[tc.id].detail}>
											{checkResults[tc.id].pass ? '✓' : '✖'}
										</span>
									{/if}
								</div>
								{#if tc.description}
									<div class="tc-desc">{tc.description}</div>
								{/if}
								{#if tc.tags}
									<div class="tc-tags">
										{#each tc.tags.split(',').filter(t => t.trim()) as tag}
											<span class="tag">{tag.trim()}</span>
										{/each}
									</div>
								{/if}
							</button>
						{/each}
					{/each}
				{/if}
			</div>

			<div class="tc-detail">
				{#if selected}
					<div class="detail-header">
						<h3>{selected.name}</h3>
						<div class="detail-actions">
							<button class="btn" onclick={() => handleRunCheck(selected)}>{tr('tc.runCheck')}</button>
							<button class="btn btn-primary" onclick={() => onLoad(selected)}>{tr('tc.loadInEditor')}</button>
							<button class="btn" onclick={() => startEdit(selected)}>{tr('tc.edit')}</button>
							<button class="btn btn-danger" onclick={() => handleDelete(selected)}>{tr('tc.delete')}</button>
						</div>
					</div>
					{#if selected.description}
						<div class="detail-desc">{selected.description}</div>
					{/if}
					<div class="detail-meta">
						<span class="meta-item">{tr('tc.category')}: <strong>{selected.category}</strong></span>
						<span class="meta-item">{tr('tc.updated')}: {new Date(selected.updated_at).toLocaleString()}</span>
						{#if selected.expected_message_type}
							<span class="meta-item">{tr('tc.expectedType')}: <strong>{selected.expected_message_type}</strong></span>
						{/if}
						<span class="meta-item">{tr('tc.expectedResult')}: <strong>{selected.expected_validation_result === 'invalid' ? tr('tc.resultInvalid') : tr('tc.resultValid')}</strong></span>
					</div>
					{#if checkResults[selected.id]}
						<div class="check-detail" class:pass={checkResults[selected.id].pass} class:fail={!checkResults[selected.id].pass}>
							{checkResults[selected.id].pass ? '✓' : '✖'} {checkResults[selected.id].detail}
						</div>
					{/if}
					<pre class="detail-content">{selected.content}</pre>
				{:else}
					<div class="empty">{tr('tc.selectPrompt')}</div>
				{/if}
			</div>
		</div>
	{:else if mode === 'export'}
		<div class="tc-form">
			<h3 class="pack-title">{tr('tc.exportTitle', { count: exportCount })}</h3>
			<p class="pack-sub">{exportIds.length ? tr('tc.exportFilteredHint', { count: exportCount }) : tr('tc.exportAllHint')}</p>
			{#if exportPhi.length === 0}
				<div class="pack-note ok">{tr('tc.phiNone')}</div>
			{:else}
				<div class="pack-note warn">
					<strong>{tr('tc.phiFound')}</strong>
					<ul class="pack-list">
						{#each exportPhi as p (p.id)}
							<li>
								<span class="pack-name">{p.name}</span> —
								{#if p.kind === 'hl7'}{p.fields.join(', ')}
								{:else if p.kind === 'fhir'}{tr('tc.phiFhir')}
								{:else}{tr('tc.phiUnparsed')}{/if}
							</li>
						{/each}
					</ul>
				</div>
				<label class="pack-check" class:disabled={!canMask}>
					<input type="checkbox" bind:checked={exportAnonymize} disabled={!canMask} />
					{tr('tc.exportAnonymize')}
					{#if !canMask}<span class="pro-badge">PRO</span>{/if}
				</label>
			{/if}
			<div class="form-actions">
				<button class="btn" onclick={() => (mode = 'list')}>{tr('dialog.cancel')}</button>
				<button class="btn btn-primary" onclick={doExport} disabled={busy}>{tr('tc.exportDo')}</button>
			</div>
		</div>
	{:else if mode === 'import' && importPreview}
		<div class="tc-form">
			<h3 class="pack-title">{tr('tc.importTitle')}</h3>
			<p class="pack-sub">
				{tr('tc.importFrom', {
					file: fileName(importPath),
					date: importPreview.exported_at ? new Date(importPreview.exported_at).toLocaleString() : '—',
					version: importPreview.app_version || '—',
				})}
			</p>
			{#if importPreview.items.length === 0}
				<div class="pack-note">{tr('tc.importEmpty')}</div>
			{:else}
				<div class="pack-items">
					{#each importPreview.items as it (it.index)}
						<div class="pack-item">
							<span class="pack-name">{it.name}</span>
							<span class="pack-cat">{it.category}</span>
							{#if it.status === 'new'}
								<span class="status new">{tr('tc.statusNew')}</span>
							{:else if it.status === 'identical'}
								<span class="status same">{tr('tc.statusIdentical')}</span>
							{:else}
								<span class="status conflict">{tr('tc.statusConflict', { name: it.existing_name ?? '' })}</span>
								<select class="form-input pack-choice" value={importChoices[it.index] ?? 'skip'}
									onchange={(e) => (importChoices = { ...importChoices, [it.index]: (e.currentTarget as HTMLSelectElement).value as ConflictChoice })}>
									<option value="skip">{tr('tc.choiceSkip')}</option>
									<option value="overwrite">{tr('tc.choiceOverwrite')}</option>
									<option value="copy">{tr('tc.choiceCopy')}</option>
								</select>
							{/if}
						</div>
					{/each}
				</div>
			{/if}
			<div class="pack-sub">{tr('tc.importSummary', { add: importTotals.add, update: importTotals.update, skip: importTotals.skip })}</div>
			{#if importOverLimit}
				<div class="pack-note warn">{tr('tc.importOverLimit', { room: importPreview.room ?? 0, add: importTotals.add })}</div>
			{/if}
			<div class="form-actions">
				<button class="btn" onclick={() => (mode = 'list')}>{tr('dialog.cancel')}</button>
				<button class="btn btn-primary" onclick={doImport}
					disabled={busy || importOverLimit || importTotals.add + importTotals.update === 0}>
					{tr('tc.importDo')}
				</button>
			</div>
		</div>
	{:else}
		<!-- Edit / New form -->
		<div class="tc-form">
			<div class="form-row">
				<label for="tc-name">{tr('tc.nameRequired')}</label>
				<input id="tc-name" bind:this={nameInputEl} bind:value={formName} placeholder="e.g. ADT^A01 admission test" class="form-input" />
			</div>
			<div class="form-row">
				<label for="tc-desc">{tr('tc.description')}</label>
				<textarea id="tc-desc" bind:value={formDescription} rows={2} placeholder="When to use this test case..." class="form-input"></textarea>
			</div>
			<div class="form-grid">
				<div class="form-row">
					<label for="tc-cat">{tr('tc.category')}</label>
					<input id="tc-cat" bind:value={formCategory} list="tc-cats" placeholder="admission, orders, ..." class="form-input" />
					<datalist id="tc-cats">
						{#each existingCategories as cat}<option value={cat}></option>{/each}
					</datalist>
				</div>
				<div class="form-row">
					<label for="tc-tags">{tr('tc.tags')}</label>
					<input id="tc-tags" bind:value={formTags} placeholder={tr('tc.tagsPlaceholder')} class="form-input" />
				</div>
			</div>
			<div class="form-grid">
				<div class="form-row">
					<label for="tc-exp-type">{tr('tc.expectedType')}</label>
					<input id="tc-exp-type" bind:value={formExpectedType} placeholder="ADT^A01" class="form-input" />
				</div>
				<div class="form-row">
					<label for="tc-exp-result">{tr('tc.expectedResult')}</label>
					<select id="tc-exp-result" bind:value={formExpectedResult} class="form-input">
						<option value="valid">{tr('tc.resultValid')}</option>
						<option value="invalid">{tr('tc.resultInvalid')}</option>
					</select>
				</div>
			</div>
			<div class="form-row">
				<label for="tc-content">{tr('tc.contentRequired')}</label>
				<textarea id="tc-content" bind:value={formContent} rows={10} class="form-input mono"></textarea>
			</div>
			<div class="form-actions">
				<button class="btn" onclick={cancelForm}>{tr('dialog.cancel')}</button>
				<button class="btn btn-primary" onclick={handleSave} disabled={!formName.trim() || !formContent.trim()}>
					{mode === 'edit' ? tr('tc.saveChanges') : tr('tc.saveNew')}
				</button>
			</div>
		</div>
	{/if}
</div>

<style>
	.tc-library { display: flex; flex-direction: column; height: 100%; max-height: 85vh; background: var(--color-bg-secondary); }
	.tc-header { display: flex; justify-content: space-between; align-items: center; padding: 10px 14px; border-bottom: 1px solid var(--color-border); flex-shrink: 0; font-weight: 600; }
	.header-actions { display: flex; align-items: center; gap: 8px; }
	.close-btn { background: none; border: none; color: var(--color-text-secondary); font-size: 20px; cursor: pointer; }

	.tc-search { padding: 8px 12px; border-bottom: 1px solid var(--color-border); flex-shrink: 0; display: flex; gap: 8px; align-items: center; }
	.run-summary { font-size: 11px; font-weight: 700; color: var(--color-error); white-space: nowrap; }
	.run-summary.all-pass { color: var(--color-success); }
	.check-badge { margin-left: 6px; font-size: 11px; font-weight: 700; }
	.check-badge.pass { color: var(--color-success); }
	.check-badge.fail { color: var(--color-error); }
	.check-detail { padding: 6px 10px; border-radius: 4px; font-size: 12px; border: 1px solid; }
	.check-detail.pass { color: var(--color-success); border-color: var(--color-success); }
	.check-detail.fail { color: var(--color-error); border-color: var(--color-error); }
	.search-input { flex: 1; width: auto; padding: 5px 8px; border: 1px solid var(--color-border); border-radius: 3px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 12px; font-family: inherit; }

	.tc-body { display: flex; flex: 1; min-height: 0; overflow: hidden; }
	.tc-list { width: 40%; overflow-y: auto; border-right: 1px solid var(--color-border); padding: 4px 0; }
	.tc-category { padding: 8px 12px 4px; font-size: 10px; font-weight: 700; text-transform: uppercase; color: var(--color-text-secondary); letter-spacing: 0.5px; }
	.tc-item { display: block; width: 100%; padding: 6px 12px; background: none; border: none; color: var(--color-text-primary); text-align: left; cursor: pointer; border-left: 2px solid transparent; font-family: inherit; }
	.tc-item:hover { background: var(--color-bg-tertiary); }
	.tc-item.selected { background: var(--color-bg-tertiary); border-left-color: var(--color-accent); }
	.tc-name { font-size: 12px; font-weight: 600; }
	.tc-desc { font-size: 11px; color: var(--color-text-secondary); margin-top: 2px; }
	.tc-tags { display: flex; flex-wrap: wrap; gap: 3px; margin-top: 3px; }
	.tag { padding: 1px 6px; background: var(--color-bg-primary); border-radius: 8px; font-size: 10px; color: var(--color-accent); }

	.tc-detail { flex: 1; padding: 12px; overflow-y: auto; display: flex; flex-direction: column; gap: 8px; }
	.detail-header { display: flex; justify-content: space-between; align-items: center; gap: 8px; }
	.detail-header h3 { margin: 0; font-size: 14px; }
	.detail-actions { display: flex; gap: 4px; }
	.detail-desc { font-size: 12px; color: var(--color-text-secondary); padding: 6px 10px; background: var(--color-bg-tertiary); border-radius: 4px; }
	.detail-meta { display: flex; gap: 12px; font-size: 11px; color: var(--color-text-secondary); }
	.detail-content { flex: 1; margin: 0; padding: 8px; background: var(--color-bg-primary); border: 1px solid var(--color-border); border-radius: 4px; font-family: 'JetBrains Mono', monospace; font-size: 11px; white-space: pre-wrap; overflow: auto; color: var(--color-text-primary); }

	.tc-form { flex: 1; overflow-y: auto; padding: 16px; display: flex; flex-direction: column; gap: 10px; }
	.form-row { display: flex; flex-direction: column; gap: 3px; }
	.form-row label { font-size: 11px; color: var(--color-text-secondary); font-weight: 600; }
	.form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
	.form-input { padding: 6px 8px; border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 12px; font-family: inherit; }
	.form-input.mono { font-family: 'JetBrains Mono', monospace; font-size: 11px; }
	.form-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 8px; }

	.pack-title { margin: 0; font-size: 14px; }
	.pack-sub { margin: 0; font-size: 12px; color: var(--color-text-secondary); }
	.pack-note { padding: 8px 10px; border-radius: 4px; font-size: 12px; border: 1px solid var(--color-border); }
	.pack-note.ok { border-color: var(--color-success); color: var(--color-success); }
	.pack-note.warn { border-color: var(--color-warning, #f9e2af); }
	.pack-list { margin: 6px 0 0; padding-left: 18px; max-height: 180px; overflow-y: auto; }
	.pack-name { font-weight: 600; }
	.pack-check { display: flex; align-items: center; gap: 6px; font-size: 12px; }
	.pack-check.disabled { color: var(--color-text-secondary); }
	.pro-badge { font-size: 9px; font-weight: 700; padding: 1px 5px; border-radius: 3px; background: var(--color-accent); color: var(--color-bg-primary); }
	.pack-items { display: flex; flex-direction: column; gap: 2px; max-height: 45vh; overflow-y: auto; border: 1px solid var(--color-border); border-radius: 4px; }
	.pack-item { display: flex; align-items: center; gap: 8px; padding: 5px 8px; font-size: 12px; border-bottom: 1px solid var(--color-border); }
	.pack-item:last-child { border-bottom: none; }
	.pack-cat { color: var(--color-text-secondary); font-size: 11px; }
	.status { margin-left: auto; font-size: 11px; font-weight: 600; }
	.status.new { color: var(--color-success); }
	.status.same { color: var(--color-text-secondary); }
	.status.conflict { color: var(--color-warning, #f9e2af); }
	.pack-choice { width: auto; padding: 3px 6px; }
	.load-error { color: var(--color-error); font-style: normal; display: flex; flex-direction: column; gap: 8px; align-items: center; }
	.empty { padding: 24px; text-align: center; color: var(--color-text-secondary); font-style: italic; font-size: 12px; }

	.btn { padding: 5px 14px; border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 12px; font-family: inherit; cursor: pointer; }
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.btn-primary { background: var(--color-accent); color: var(--color-bg-primary); border-color: var(--color-accent); }
	.btn-danger { background: var(--color-error); color: white; border-color: var(--color-error); }
</style>
