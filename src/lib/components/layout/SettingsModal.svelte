<script lang="ts">
	import { t, subscribeLocale, setLocale, getLocale, type Locale } from '$lib/i18n';
	import { getPreference, setPreference } from '$lib/ipc/database';
	import {
		listPlugins, reloadPlugins, setPluginEnabled,
		openPluginsFolder, getPluginsDir, type PluginInfo,
	} from '$lib/ipc/plugins';
	import ShortcutsEditor from '$lib/components/layout/ShortcutsEditor.svelte';

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		theme: string;
		onClose: () => void;
		onThemeChange: (theme: string) => void;
	}

	let { theme, onClose, onThemeChange }: Props = $props();

	let activeSection = $state('editor');

	// Editor settings
	let fontSize = $state(13);
	let fontFamily = $state("'JetBrains Mono', 'Fira Code', 'Consolas', monospace");
	let wordWrap = $state('on');
	let minimap = $state(true);
	let lineNumbers = $state(true);
	let tabSize = $state(4);
	let smoothScrolling = $state(true);
	let cursorBlinking = $state('smooth');
	let renderWhitespace = $state('none');
	let bracketPairColorization = $state(false);

	// Parser settings
	let truncationThreshold = $state(100);
	let autoParseDelay = $state(500);
	let autoParse = $state(true);

	// Display settings
	let currentTheme = $state(theme);
	let currentLocale = $state<Locale>(getLocale());

	// Memory settings
	let maxOpenMessages = $state(50);

	// Session settings
	let restoreSession = $state(true);

	// Load saved settings
	let loaded = false;
	$effect(() => {
		if (loaded || typeof window === 'undefined') return;
		loaded = true;
		loadSettings();
	});

	async function loadSettings() {
		try {
			const fs = await getPreference('editor_font_size');
			if (fs) fontSize = parseInt(fs) || 13;
			const ff = await getPreference('editor_font_family');
			if (ff) fontFamily = ff;
			const ww = await getPreference('editor_word_wrap');
			if (ww) wordWrap = ww;
			const mm = await getPreference('editor_minimap');
			if (mm) minimap = mm === 'true';
			const ln = await getPreference('editor_line_numbers');
			if (ln) lineNumbers = ln === 'true';
			const ts = await getPreference('editor_tab_size');
			if (ts) tabSize = parseInt(ts) || 4;
			const tt = await getPreference('truncation_threshold');
			if (tt) truncationThreshold = parseInt(tt) || 100;
			const ap = await getPreference('auto_parse_delay');
			if (ap) autoParseDelay = parseInt(ap) || 500;
			const mo = await getPreference('max_open_messages');
			if (mo) maxOpenMessages = parseInt(mo) || 50;
			const rw = await getPreference('editor_render_whitespace');
			if (rw) renderWhitespace = rw;
			const rs = await getPreference('restore_session');
			if (rs !== null) restoreSession = rs !== 'false';
		} catch { /* web mode */ }
	}

	async function saveAndClose() {
		try {
			await setPreference('editor_font_size', String(fontSize));
			await setPreference('editor_font_family', fontFamily);
			await setPreference('editor_word_wrap', wordWrap);
			await setPreference('editor_minimap', String(minimap));
			await setPreference('editor_line_numbers', String(lineNumbers));
			await setPreference('editor_tab_size', String(tabSize));
			await setPreference('truncation_threshold', String(truncationThreshold));
			await setPreference('auto_parse_delay', String(autoParseDelay));
			await setPreference('max_open_messages', String(maxOpenMessages));
			await setPreference('editor_render_whitespace', renderWhitespace);
			await setPreference('restore_session', String(restoreSession));
			await setPreference('theme', currentTheme);
			await setPreference('language', currentLocale);
		} catch { /* web mode */ }
		if (currentTheme !== theme) onThemeChange(currentTheme);
		if (currentLocale !== getLocale()) setLocale(currentLocale);
		onClose();
	}

	// Plugins state
	let plugins = $state<PluginInfo[]>([]);
	let pluginsDir = $state('');
	let pluginsLoading = $state(false);
	let pluginsLoaded = $state(false);
	let pluginsError = $state<string | null>(null);

	async function loadPluginsInfo() {
		pluginsLoading = true;
		pluginsError = null;
		try {
			plugins = await listPlugins();
			pluginsDir = await getPluginsDir();
		} catch (e) {
			pluginsError = String(e);
		} finally {
			pluginsLoading = false;
			pluginsLoaded = true;
		}
	}

	async function handleReloadPlugins() {
		pluginsLoading = true;
		pluginsError = null;
		try {
			plugins = await reloadPlugins();
		} catch (e) {
			pluginsError = String(e);
		} finally {
			pluginsLoading = false;
		}
	}

	async function handleTogglePlugin(p: PluginInfo) {
		const nextEnabled = !p.enabled;
		try {
			await setPluginEnabled(p.id, nextEnabled);
			await setPreference(`plugin_enabled:${p.id}`, String(nextEnabled));
			p.enabled = nextEnabled;
		} catch (e) {
			pluginsError = String(e);
		}
	}

	async function handleOpenPluginsFolder() {
		try { await openPluginsFolder(); } catch (e) { pluginsError = String(e); }
	}

	$effect(() => {
		if (activeSection === 'plugins' && !pluginsLoaded && !pluginsLoading) {
			void loadPluginsInfo();
		}
	});

	const sections = [
		{ id: 'editor', label: 'Editor', icon: '\u270E' },
		{ id: 'display', label: 'Display', icon: '\u2600' },
		{ id: 'shortcuts', label: 'Shortcuts', icon: '\u2328' },
		{ id: 'parser', label: 'Parser', icon: '\u2699' },
		{ id: 'memory', label: 'Performance', icon: '\u26A1' },
		{ id: 'plugins', label: tr('plugins.title'), icon: '\u2699' },
	];

	const fontFamilies = [
		"'JetBrains Mono', 'Fira Code', 'Consolas', monospace",
		"'Fira Code', 'Consolas', monospace",
		"'Consolas', 'Courier New', monospace",
		"'Cascadia Code', 'Consolas', monospace",
		"'Source Code Pro', monospace",
		"monospace",
	];

	function fontLabel(f: string): string {
		return f.split(',')[0].replace(/'/g, '').trim();
	}
</script>

<div class="settings-modal">
	<div class="settings-header">
		<span>Settings</span>
		<button class="close-btn" onclick={onClose}>&times;</button>
	</div>

	<div class="settings-body">
		<!-- Sidebar -->
		<div class="settings-nav">
			{#each sections as sec}
				<button
					class="nav-item"
					class:active={activeSection === sec.id}
					onclick={() => { activeSection = sec.id; }}
				>
					<span class="nav-icon">{sec.icon}</span>
					{sec.label}
				</button>
			{/each}
		</div>

		<!-- Content -->
		<div class="settings-content">
			{#if activeSection === 'editor'}
				<h3>Editor</h3>

				<div class="setting-row">
					<label for="s-fontsize">Font Size</label>
					<input id="s-fontsize" type="number" min={8} max={32} bind:value={fontSize} class="input-sm" />
					<span class="hint">px</span>
				</div>

				<div class="setting-row">
					<label for="s-fontfamily">Font Family</label>
					<select id="s-fontfamily" bind:value={fontFamily}>
						{#each fontFamilies as ff}
							<option value={ff}>{fontLabel(ff)}</option>
						{/each}
					</select>
				</div>

				<div class="setting-row">
					<label for="s-tabsize">Tab Size</label>
					<input id="s-tabsize" type="number" min={1} max={8} bind:value={tabSize} class="input-xs" />
				</div>

				<div class="setting-row">
					<label for="s-wordwrap">Word Wrap</label>
					<select id="s-wordwrap" bind:value={wordWrap}>
						<option value="on">On</option>
						<option value="off">Off</option>
						<option value="wordWrapColumn">At Column</option>
					</select>
				</div>

				<div class="setting-row">
					<label for="s-whitespace">Render Whitespace</label>
					<select id="s-whitespace" bind:value={renderWhitespace}>
						<option value="none">None</option>
						<option value="boundary">Boundary</option>
						<option value="all">All</option>
					</select>
				</div>

				<div class="setting-check">
					<label><input type="checkbox" bind:checked={minimap} /> Show Minimap</label>
				</div>
				<div class="setting-check">
					<label><input type="checkbox" bind:checked={lineNumbers} /> Show Line Numbers</label>
				</div>
				<div class="setting-check">
					<label><input type="checkbox" bind:checked={smoothScrolling} /> Smooth Scrolling</label>
				</div>
				<div class="setting-check">
					<label><input type="checkbox" bind:checked={bracketPairColorization} /> Bracket Pair Colorization</label>
				</div>

			{:else if activeSection === 'display'}
				<h3>Display</h3>

				<div class="setting-row">
					<label>Theme</label>
					<div class="theme-options">
						<button
							class="theme-btn"
							class:active={currentTheme === 'dark'}
							onclick={() => { currentTheme = 'dark'; }}
						>
							<div class="theme-preview dark-preview"></div>
							Dark
						</button>
						<button
							class="theme-btn"
							class:active={currentTheme === 'light'}
							onclick={() => { currentTheme = 'light'; }}
						>
							<div class="theme-preview light-preview"></div>
							Light
						</button>
					</div>
				</div>

				<div class="setting-row">
					<label for="s-locale">Language</label>
					<select id="s-locale" bind:value={currentLocale}>
						<option value="en">English</option>
						<option value="it">Italiano</option>
						<option value="fr">Français</option>
						<option value="es">Español</option>
						<option value="de">Deutsch</option>
					</select>
				</div>

			{:else if activeSection === 'shortcuts'}
				<h3>Keyboard Shortcuts</h3>
				<ShortcutsEditor />

			{:else if activeSection === 'parser'}
				<h3>Parser &amp; Truncation</h3>

				<div class="setting-row">
					<label for="s-trunc">Truncation Threshold</label>
					<input id="s-trunc" type="number" min={50} max={10000} step={50} bind:value={truncationThreshold} class="input-sm" />
					<span class="hint">bytes - fields larger than this are truncated in the editor</span>
				</div>

				<div class="setting-row">
					<label for="s-autoparsedelay">Auto-Parse Delay</label>
					<input id="s-autoparsedelay" type="number" min={100} max={5000} step={100} bind:value={autoParseDelay} class="input-sm" />
					<span class="hint">ms - delay before auto-parsing after typing</span>
				</div>

				<div class="setting-check">
					<label><input type="checkbox" bind:checked={autoParse} /> Auto-parse on content change</label>
				</div>

			{:else if activeSection === 'memory'}
				<h3>Performance</h3>

				<div class="setting-row">
					<label for="s-maxmsg">Max Open Messages</label>
					<input id="s-maxmsg" type="number" min={5} max={200} bind:value={maxOpenMessages} class="input-sm" />
					<span class="hint">messages kept in memory</span>
				</div>

				<h3>Session</h3>

				<div class="setting-check">
					<label>
						<input type="checkbox" bind:checked={restoreSession} />
						Restore open tabs on startup
					</label>
					<div class="hint">
						When enabled, BridgeLab saves your open tabs (including unsaved
						edits) and reopens them the next time you launch the app.
					</div>
				</div>

				<div class="info-block">
					<strong>Memory tips:</strong>
					<ul>
						<li>Large messages (5-10 MB) with base64 are truncated in the editor for performance</li>
						<li>Full content is available via "Expand Field" or "Copy Full Message"</li>
						<li>Reduce max open messages if experiencing slowness</li>
						<li>The parser uses SIMD-accelerated scanning for fast indexing</li>
					</ul>
				</div>

			{:else if activeSection === 'plugins'}
				<h3>{tr('plugins.title')}</h3>

				<p class="hint" style="margin-bottom: 8px;">
					{tr('plugins.description')}
				</p>

				<div class="plugins-toolbar">
					<button class="btn" onclick={handleReloadPlugins} disabled={pluginsLoading}>
						{pluginsLoading ? tr('plugins.loading') : tr('plugins.reload')}
					</button>
					<button class="btn" onclick={handleOpenPluginsFolder}>{tr('plugins.openFolder')}</button>
					<code class="plugins-path" title={pluginsDir}>{pluginsDir}</code>
				</div>

				{#if pluginsError}
					<div class="plugin-error">{pluginsError}</div>
				{/if}

				{#if !pluginsLoading && pluginsLoaded && plugins.length === 0}
					<div class="info-block">
						{tr('plugins.noPlugins')}
					</div>
				{/if}

				{#each plugins as p}
					<div class="plugin-row" class:errored={!!p.error}>
						<div class="plugin-main">
							<div class="plugin-title">
								<strong>{p.name}</strong>
								<span class="plugin-kind">{p.kind}</span>
								<span class="plugin-version">v{p.version}</span>
							</div>
							{#if p.description}
								<div class="plugin-desc">{p.description}</div>
							{/if}
							<div class="plugin-meta">
								<span>{tr(p.rule_count === 1 ? 'plugins.rule' : 'plugins.rules', { count: p.rule_count })}</span>
								{#if p.author}<span>{tr('plugins.by', { author: p.author })}</span>{/if}
								<span class="plugin-filepath" title={p.path}>{p.path}</span>
							</div>
							{#if p.error}
								<div class="plugin-error">{tr('plugins.parseError', { error: p.error })}</div>
							{/if}
						</div>
						<label class="plugin-toggle">
							<input
								type="checkbox"
								checked={p.enabled}
								disabled={!!p.error}
								onchange={() => handleTogglePlugin(p)}
							/>
							{p.enabled ? tr('plugins.enabled') : tr('plugins.disabled')}
						</label>
					</div>
				{/each}
			{/if}
		</div>
	</div>

	<div class="settings-footer">
		<button class="btn" onclick={onClose}>Cancel</button>
		<button class="btn btn-primary" onclick={saveAndClose}>Save &amp; Close</button>
	</div>
</div>

<style>
	.settings-modal { display: flex; flex-direction: column; max-height: 80vh; width: 100%; }
	.settings-header { display: flex; justify-content: space-between; align-items: center; padding: 12px 16px; border-bottom: 1px solid var(--color-border); font-weight: 700; font-size: 14px; }
	.close-btn { background: none; border: none; color: var(--color-text-secondary); cursor: pointer; font-size: 20px; }

	.settings-body { display: flex; flex: 1; overflow: hidden; min-height: 350px; }

	/* Nav sidebar */
	.settings-nav { width: 140px; border-right: 1px solid var(--color-border); padding: 8px 0; flex-shrink: 0; }
	.nav-item { display: flex; align-items: center; gap: 8px; width: 100%; padding: 8px 16px; background: none; border: none; color: var(--color-text-secondary); font-size: 12px; font-family: inherit; cursor: pointer; text-align: left; }
	.nav-item:hover { background: var(--color-bg-tertiary); }
	.nav-item.active { background: var(--color-bg-tertiary); color: var(--color-text-primary); border-right: 2px solid var(--color-accent); }
	.nav-icon { font-size: 14px; width: 18px; text-align: center; }

	/* Content */
	.settings-content { flex: 1; overflow-y: auto; padding: 16px 20px; }
	.settings-content h3 { margin: 0 0 12px; font-size: 14px; color: var(--color-text-primary); }

	.setting-row { display: flex; align-items: center; gap: 8px; margin-bottom: 10px; }
	.setting-row label { width: 150px; flex-shrink: 0; font-size: 12px; color: var(--color-text-secondary); }
	.setting-row input, .setting-row select { padding: 4px 8px; border: 1px solid var(--color-border); border-radius: 3px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 12px; font-family: inherit; }
	.input-sm { width: 80px; }
	.input-xs { width: 55px; }
	.hint { font-size: 10px; color: var(--color-text-secondary); opacity: 0.7; }

	.setting-check { margin-bottom: 8px; }
	.setting-check label { font-size: 12px; color: var(--color-text-primary); display: flex; align-items: center; gap: 6px; cursor: pointer; }
	.setting-check input[type="checkbox"] { accent-color: var(--color-accent); }

	/* Theme selector */
	.theme-options { display: flex; gap: 10px; }
	.theme-btn { display: flex; flex-direction: column; align-items: center; gap: 4px; padding: 8px 16px; border: 2px solid var(--color-border); border-radius: 6px; background: none; color: var(--color-text-primary); font-size: 11px; font-family: inherit; cursor: pointer; }
	.theme-btn.active { border-color: var(--color-accent); }
	.theme-preview { width: 48px; height: 30px; border-radius: 4px; border: 1px solid var(--color-border); }
	.dark-preview { background: linear-gradient(135deg, #1e1e2e 50%, #313244 50%); }
	.light-preview { background: linear-gradient(135deg, #eff1f5 50%, #dce0e8 50%); }

	.info-block { margin-top: 12px; padding: 10px 12px; background: var(--color-bg-tertiary); border-radius: 4px; font-size: 11px; color: var(--color-text-secondary); }

	.plugins-toolbar { display: flex; align-items: center; gap: 8px; margin-bottom: 12px; flex-wrap: wrap; }
	.plugins-path { font-size: 10px; color: var(--color-text-secondary); word-break: break-all; }
	.plugin-row { display: flex; justify-content: space-between; align-items: flex-start; gap: 12px; padding: 10px; border: 1px solid var(--color-border); border-radius: 4px; margin-bottom: 8px; }
	.plugin-row.errored { border-color: var(--color-error); }
	.plugin-main { flex: 1; min-width: 0; }
	.plugin-title { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; font-size: 13px; }
	.plugin-kind { font-size: 10px; padding: 1px 6px; border-radius: 3px; background: var(--color-bg-tertiary); color: var(--color-text-secondary); text-transform: uppercase; }
	.plugin-version { font-size: 10px; color: var(--color-text-secondary); }
	.plugin-desc { font-size: 12px; color: var(--color-text-secondary); margin: 4px 0; }
	.plugin-meta { display: flex; gap: 10px; flex-wrap: wrap; font-size: 10px; color: var(--color-text-secondary); }
	.plugin-filepath { font-family: 'JetBrains Mono', monospace; opacity: 0.7; overflow: hidden; text-overflow: ellipsis; }
	.plugin-error { color: var(--color-error); font-size: 11px; margin-top: 4px; }
	.plugin-toggle { display: flex; align-items: center; gap: 6px; font-size: 11px; color: var(--color-text-secondary); white-space: nowrap; }
	.info-block ul { margin: 6px 0 0; padding-left: 16px; }
	.info-block li { margin-bottom: 4px; }

	/* Footer */
	.settings-footer { display: flex; justify-content: flex-end; gap: 8px; padding: 12px 16px; border-top: 1px solid var(--color-border); }
	.btn { padding: 6px 16px; border: 1px solid var(--color-border); border-radius: 4px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 12px; font-family: inherit; cursor: pointer; }
	.btn:hover { background: var(--color-border); }
	.btn-primary { background: var(--color-accent); color: var(--color-bg-primary); border-color: var(--color-accent); }
	.btn-primary:hover { opacity: 0.9; }
</style>
