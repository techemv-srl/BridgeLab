<script lang="ts">
	import { t, subscribeLocale } from '$lib/i18n';
	import { messageStore } from '$lib/stores/messages.svelte';
	import { sessionStore } from '$lib/stores/session.svelte';
	import { editorOptionsStore } from '$lib/stores/editor-options.svelte';
	import HelpWindow from '$lib/components/layout/HelpWindow.svelte';
	import AnonymizeDialog from '$lib/components/anonymization/AnonymizeDialog.svelte';
	import SettingsModal from '$lib/components/layout/SettingsModal.svelte';
	import SchemaExportDialog from '$lib/components/layout/SchemaExportDialog.svelte';
	import CompareDialog from '$lib/components/diff/CompareDialog.svelte';
	import BatchValidateDialog from '$lib/components/batch/BatchValidateDialog.svelte';
	import GenerateDialog from '$lib/components/generator/GenerateDialog.svelte';
	import ActivationDialog from '$lib/components/licensing/ActivationDialog.svelte';
	import TemplateDialog from '$lib/components/templates/TemplateDialog.svelte';
	import BundleVisualizer from '$lib/components/bundle/BundleVisualizer.svelte';
	import TestCaseLibrary from '$lib/components/testcases/TestCaseLibrary.svelte';
	import type { TestCase } from '$lib/ipc/testcases';
	import type { LicenseStatus } from '$lib/ipc/licensing';
	import type { MessageTemplate } from '$lib/ipc/templates';

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	// Real app version for the About dialog (was hardcoded "0.1.0" and went
	// stale). Falls back to the packaged version when the Tauri API is absent.
	let appVersion = $state('');
	if (typeof window !== 'undefined') {
		import('@tauri-apps/api/app')
			.then(({ getVersion }) => getVersion())
			.then((v) => { appVersion = v; })
			.catch(() => { /* web mode: leave placeholder */ });
	}

	interface Props {
		expandedFieldContent: string | null;
		showAnonymize: boolean;
		showAbout: boolean;
		showBundleVisualizer: boolean;
		showTestCases: boolean;
		showTemplates: boolean;
		showActivation: boolean;
		showSettings: boolean;
		showSchemaExport: boolean;
		showBatch: boolean;
		showGenerate: boolean;
		showCompare: boolean;
		showHelp: boolean;
		licenseStatus: LicenseStatus | null;
		settingsSection: string;
		theme: string;
		onTestCaseLoaded: (tc: TestCase) => void;
		onTemplateSelected: (template: MessageTemplate) => void;
		onAnonymized: (text: string) => void;
		onSetTheme: (theme: string) => void;
		onOpenRecentFile: (path: string) => void;
		onOpenGenerated: (content: string, label: string) => void;
	}

	let {
		expandedFieldContent = $bindable(),
		showAnonymize = $bindable(),
		showAbout = $bindable(),
		showBundleVisualizer = $bindable(),
		showTestCases = $bindable(),
		showTemplates = $bindable(),
		showActivation = $bindable(),
		showSettings = $bindable(),
		showSchemaExport = $bindable(),
		showBatch = $bindable(),
		showGenerate = $bindable(),
		showCompare = $bindable(),
		showHelp = $bindable(),
		licenseStatus = $bindable(),
		settingsSection,
		theme,
		onTestCaseLoaded,
		onTemplateSelected,
		onAnonymized,
		onSetTheme,
		onOpenRecentFile,
		onOpenGenerated,
	}: Props = $props();

	let activeTab = $derived(messageStore.activeTab);

	function closeExpandedField() {
		expandedFieldContent = null;
	}
</script>

<!-- Expanded field modal -->
{#if expandedFieldContent !== null}
	<div class="modal-overlay" onclick={closeExpandedField} role="presentation">
		<!-- svelte-ignore a11y_interactive_supports_focus -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="modal" onclick={(e) => e.stopPropagation()} role="dialog">
			<div class="modal-header">
				<span>{tr('modal.fullContent')}</span>
				<button class="modal-close" onclick={closeExpandedField}>&times;</button>
			</div>
			<div class="modal-body">
				<pre>{expandedFieldContent}</pre>
			</div>
			<div class="modal-footer">
				<button class="btn" onclick={() => { navigator.clipboard.writeText(expandedFieldContent!); }}>
					{tr('modal.copy')}
				</button>
				<span class="modal-info">{tr('modal.characters', { count: expandedFieldContent.length.toLocaleString() })}</span>
			</div>
		</div>
	</div>
{/if}

<!-- Anonymize dialog -->
{#if showAnonymize && activeTab?.parseResult}
	<div class="modal-overlay" onclick={() => { showAnonymize = false; }} role="presentation">
		<!-- svelte-ignore a11y_interactive_supports_focus -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="modal" onclick={(e) => e.stopPropagation()} role="dialog">
			<AnonymizeDialog
				messageId={activeTab.parseResult.message_id}
				{onAnonymized}
				onClose={() => { showAnonymize = false; }}
			/>
		</div>
	</div>
{/if}

<!-- About dialog -->
{#if showAbout}
	<div class="modal-overlay" onclick={() => { showAbout = false; }} role="presentation">
		<!-- svelte-ignore a11y_interactive_supports_focus -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="modal modal-small" onclick={(e) => e.stopPropagation()} role="dialog">
			<div class="modal-header">
				<span>{tr('about.title')}</span>
				<button class="modal-close" onclick={() => { showAbout = false; }}>&times;</button>
			</div>
			<div class="modal-body about-body">
				<!-- Bridge logo -->
				<div class="about-logo" aria-hidden="true">
					<svg viewBox="0 0 120 50" width="120" height="50" fill="none" xmlns="http://www.w3.org/2000/svg">
						<defs>
							<linearGradient id="bridge-grad" x1="0%" y1="0%" x2="100%" y2="0%">
								<stop offset="0%" stop-color="var(--color-accent, #89b4fa)"/>
								<stop offset="100%" stop-color="var(--color-segment, #cba6f7)"/>
							</linearGradient>
						</defs>
						<path d="M10 32c15-24 75-24 100 0" stroke="url(#bridge-grad)" stroke-width="3.5" stroke-linecap="round"/>
						<line x1="28" y1="28" x2="28" y2="42" stroke="var(--color-accent, #89b4fa)" stroke-width="3" stroke-linecap="round"/>
						<line x1="48" y1="22" x2="48" y2="42" stroke="var(--color-accent, #89b4fa)" stroke-width="3" stroke-linecap="round"/>
						<line x1="72" y1="22" x2="72" y2="42" stroke="var(--color-accent, #89b4fa)" stroke-width="3" stroke-linecap="round"/>
						<line x1="92" y1="28" x2="92" y2="42" stroke="var(--color-accent, #89b4fa)" stroke-width="3" stroke-linecap="round"/>
						<rect x="8" y="42" width="104" height="4" rx="2" fill="url(#bridge-grad)"/>
					</svg>
				</div>

				<div class="about-title">{tr('app.title')}</div>
				<div class="about-subtitle">{tr('app.subtitle')}</div>
				{#if appVersion}
					<div class="about-version">{tr('about.version', { version: appVersion })}</div>
				{/if}
				<p class="about-desc">{tr('about.description')}</p>
				<p class="about-license">{tr('about.license')}</p>

				<div class="about-company">
					<div class="about-company-name">TECHEMV SRL</div>
					<div class="about-contact">
						<a href="mailto:info@techemv.it">info@techemv.it</a>
						<span class="about-sep">&middot;</span>
						<a href="https://www.techemv.it" target="_blank" rel="noopener">www.techemv.it</a>
					</div>
				</div>

				<p class="about-copyright">{tr('about.copyright', { year: new Date().getFullYear().toString() })}</p>
			</div>
		</div>
	</div>
{/if}

<!-- FHIR Bundle Visualizer modal -->
{#if showBundleVisualizer && activeTab?.parseResult}
	<div class="modal-overlay" onclick={() => { showBundleVisualizer = false; }} role="presentation">
		<!-- svelte-ignore a11y_interactive_supports_focus -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="modal modal-xl" onclick={(e) => e.stopPropagation()} role="dialog">
			<BundleVisualizer
				messageId={activeTab.parseResult.message_id}
				onClose={() => { showBundleVisualizer = false; }}
			/>
		</div>
	</div>
{/if}

<!-- Test Case Library modal -->
{#if showTestCases}
	<div class="modal-overlay" onclick={() => { showTestCases = false; }} role="presentation">
		<!-- svelte-ignore a11y_interactive_supports_focus -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="modal modal-xl" onclick={(e) => e.stopPropagation()} role="dialog">
			<TestCaseLibrary
				currentContent={activeTab?.content ?? ''}
				currentLabel={activeTab?.label ?? ''}
				onLoad={onTestCaseLoaded}
				onClose={() => { showTestCases = false; }}
			/>
		</div>
	</div>
{/if}

<!-- Template selection modal -->
{#if showTemplates}
	<div class="modal-overlay" onclick={() => { showTemplates = false; }} role="presentation">
		<!-- svelte-ignore a11y_interactive_supports_focus -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="modal modal-lg" onclick={(e) => e.stopPropagation()} role="dialog">
			<TemplateDialog
				onSelect={onTemplateSelected}
				onClose={() => { showTemplates = false; }}
			/>
		</div>
	</div>
{/if}

<!-- License Activation modal -->
{#if showActivation && licenseStatus}
	<div class="modal-overlay" onclick={() => { showActivation = false; }} role="presentation">
		<!-- svelte-ignore a11y_interactive_supports_focus -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="modal modal-lg" onclick={(e) => e.stopPropagation()} role="dialog">
			<ActivationDialog
				currentStatus={licenseStatus}
				onClose={() => { showActivation = false; }}
				onStatusChange={(s) => { licenseStatus = s; }}
			/>
		</div>
	</div>
{/if}

<!-- Settings modal -->
{#if showSettings}
	<div class="modal-overlay" onclick={() => { showSettings = false; }} role="presentation">
		<!-- svelte-ignore a11y_interactive_supports_focus -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<div class="modal modal-lg" onclick={(e) => e.stopPropagation()} role="dialog">
			<SettingsModal
				initialSection={settingsSection}
				onRestoreSessionChange={(enabled) => { sessionStore.restoreEnabled = enabled; }}
				onEditorOptionsChange={() => { void editorOptionsStore.loadFromPrefs(); }}
				{theme}
				onClose={() => { showSettings = false; }}
				onThemeChange={onSetTheme}
				onShowActivation={() => { showSettings = false; showActivation = true; }}
			/>
		</div>
	</div>
{/if}

<!-- Schema XSD export dialog -->
{#if showSchemaExport}
	<SchemaExportDialog onClose={() => { showSchemaExport = false; }} />
{/if}

{#if showBatch}
	<BatchValidateDialog
		onClose={() => { showBatch = false; }}
		onOpenFile={(path) => { showBatch = false; onOpenRecentFile(path); }}
	/>
{/if}

{#if showGenerate}
	<GenerateDialog
		onClose={() => { showGenerate = false; }}
		onOpenMessage={onOpenGenerated}
	/>
{/if}

<!-- Compare messages (side-by-side diff of two open tabs) -->
{#if showCompare}
	<CompareDialog
		tabs={messageStore.tabs}
		activeTabId={messageStore.activeTabId}
		theme={theme === 'light' ? 'bridgelab-light' : 'bridgelab-dark'}
		onClose={() => { showCompare = false; }}
	/>
{/if}

{#if showHelp}
	<HelpWindow onClose={() => { showHelp = false; }} />
{/if}

<style>
	/* Buttons */
	.btn {
		padding: 4px 12px;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		background-color: var(--color-bg-tertiary);
		color: var(--color-text-primary);
		cursor: pointer;
		font-size: 12px;
		font-family: inherit;
		transition: background-color 0.15s;
	}

	.btn:hover {
		background-color: var(--color-border);
	}

	/* Modal */
	.modal-overlay {
		position: fixed;
		inset: 0;
		background-color: rgba(0, 0, 0, 0.6);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 100;
	}

	.modal {
		background-color: var(--color-bg-secondary);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		width: 80%;
		max-width: 900px;
		max-height: 80vh;
		display: flex;
		flex-direction: column;
	}

	.modal-small {
		width: 400px;
		max-width: 90%;
	}

	.modal-lg {
		width: 700px;
		max-width: 90%;
	}

	.modal-xl {
		width: 1100px;
		max-width: 95%;
		height: 80vh;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 12px 16px;
		border-bottom: 1px solid var(--color-border);
		font-weight: 600;
	}

	.modal-close {
		background: none;
		border: none;
		color: var(--color-text-secondary);
		cursor: pointer;
		font-size: 20px;
		line-height: 1;
	}

	.modal-body {
		flex: 1;
		overflow: auto;
		padding: 16px;
	}

	.modal-body pre {
		margin: 0;
		white-space: pre-wrap;
		word-break: break-all;
		font-family: 'JetBrains Mono', monospace;
		font-size: 12px;
		color: var(--color-text-primary);
	}

	.modal-footer {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 12px 16px;
		border-top: 1px solid var(--color-border);
	}

	.modal-info {
		font-size: 11px;
		color: var(--color-text-secondary);
	}

	/* About dialog */
	.about-body {
		text-align: center;
		padding: 24px 20px;
	}

	.about-logo {
		margin-bottom: 16px;
		display: flex;
		justify-content: center;
	}

	.about-title {
		font-size: 26px;
		font-weight: 800;
		color: var(--color-accent);
		letter-spacing: -0.02em;
	}

	.about-subtitle {
		font-size: 14px;
		color: var(--color-text-secondary);
		font-style: italic;
		margin-bottom: 8px;
	}

	.about-version {
		font-size: 12px;
		color: var(--color-text-secondary);
		margin-bottom: 16px;
	}

	.about-desc {
		font-size: 13px;
		color: var(--color-text-primary);
		margin-bottom: 8px;
	}

	.about-license {
		font-size: 11px;
		color: var(--color-text-secondary);
		margin-bottom: 16px;
	}

	.about-company {
		padding: 12px 0;
		border-top: 1px solid var(--color-border);
		margin-top: 8px;
	}

	.about-company-name {
		font-size: 13px;
		font-weight: 700;
		color: var(--color-text-primary);
		margin-bottom: 4px;
	}

	.about-contact {
		font-size: 12px;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 8px;
	}

	.about-contact a {
		color: var(--color-accent);
		text-decoration: none;
	}

	.about-contact a:hover {
		text-decoration: underline;
	}

	.about-sep {
		color: var(--color-text-secondary);
		opacity: 0.5;
	}

	.about-copyright {
		font-size: 10px;
		color: var(--color-text-secondary);
		margin-top: 12px;
		opacity: 0.7;
	}
</style>
