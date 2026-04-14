<script lang="ts">
	import {
		mllpSend, mllpReceive, httpRequest,
		getConnectionProfiles, saveConnectionProfile, deleteConnectionProfile,
		getRequestHistory, clearRequestHistory,
		type ConnectionProfile, type HistoryEntry, type MllpSendResult, type HttpResult,
	} from '$lib/ipc/communication';
	import { t, subscribeLocale } from '$lib/i18n';
	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		currentMessage?: string;
		onMessageReceived?: (content: string) => void;
	}

	let { currentMessage = '', onMessageReceived }: Props = $props();

	let activeSubTab = $state<'mllp' | 'http' | 'history'>('mllp');

	// MLLP state
	let mllpHost = $state('localhost');
	let mllpPort = $state(2575);
	let mllpTimeout = $state(30);
	let mllpResult = $state<MllpSendResult | null>(null);
	let mllpSending = $state(false);
	let mllpListening = $state(false);
	let mllpListenPort = $state(2576);

	// HTTP state
	let httpUrl = $state('http://localhost:8080/fhir');
	let httpMethod = $state('POST');
	let httpHeadersText = $state('Content-Type: application/json');
	let httpBody = $state('');
	let httpResult = $state<HttpResult | null>(null);
	let httpSending = $state(false);

	// History state
	let history = $state<HistoryEntry[]>([]);

	// --- MLLP actions ---
	async function handleMllpSend() {
		if (!currentMessage.trim()) return;
		mllpSending = true;
		mllpResult = null;
		try {
			mllpResult = await mllpSend(mllpHost, mllpPort, currentMessage, mllpTimeout);
		} catch (e) {
			mllpResult = { success: false, response: '', response_time_ms: 0, error: String(e) };
		}
		mllpSending = false;
		loadHistory();
	}

	async function handleMllpListen() {
		mllpListening = true;
		try {
			const msg = await mllpReceive(mllpListenPort, 60, true);
			onMessageReceived?.(msg.content);
		} catch (e) {
			console.error('MLLP listen error:', e);
		}
		mllpListening = false;
	}

	// --- HTTP actions ---
	async function handleHttpSend() {
		httpSending = true;
		httpResult = null;
		try {
			const headers: Record<string, string> = {};
			for (const line of httpHeadersText.split('\n')) {
				const idx = line.indexOf(':');
				if (idx > 0) {
					headers[line.substring(0, idx).trim()] = line.substring(idx + 1).trim();
				}
			}
			const body = httpBody.trim() || currentMessage || undefined;
			httpResult = await httpRequest(httpUrl, httpMethod, headers, body, 30);
		} catch (e) {
			httpResult = { success: false, status_code: 0, status_text: '', headers: {}, body: '', response_time_ms: 0, error: String(e) };
		}
		httpSending = false;
		loadHistory();
	}

	// --- History ---
	async function loadHistory() {
		try { history = await getRequestHistory(30); } catch { /* web mode */ }
	}

	async function handleClearHistory() {
		try { await clearRequestHistory(); history = []; } catch { /* web mode */ }
	}

	// Load history on mount
	$effect(() => { loadHistory(); });
</script>

<div class="comm-panel">
	<div class="comm-tabs">
		<button class="comm-tab" class:active={activeSubTab === 'mllp'} onclick={() => { activeSubTab = 'mllp'; }}>MLLP</button>
		<button class="comm-tab" class:active={activeSubTab === 'http'} onclick={() => { activeSubTab = 'http'; }}>HTTP</button>
		<button class="comm-tab" class:active={activeSubTab === 'history'} onclick={() => { activeSubTab = 'history'; loadHistory(); }}>History</button>
	</div>

	<div class="comm-content">
		{#if activeSubTab === 'mllp'}
			<div class="comm-form">
				<div class="form-row">
					<label>Host</label>
					<input bind:value={mllpHost} placeholder="localhost" />
					<label>Port</label>
					<input type="number" bind:value={mllpPort} style="width:80px" />
					<label>Timeout</label>
					<input type="number" bind:value={mllpTimeout} style="width:60px" />
				</div>
				<div class="form-actions">
					<button class="btn btn-primary" onclick={handleMllpSend} disabled={mllpSending || !currentMessage}>
						{mllpSending ? 'Sending...' : 'Send via MLLP'}
					</button>
					<button class="btn" onclick={handleMllpListen} disabled={mllpListening}>
						{mllpListening ? 'Listening...' : 'Listen'}
					</button>
					<input type="number" bind:value={mllpListenPort} style="width:80px" placeholder="Port" />
				</div>
				{#if mllpResult}
					<div class="result" class:success={mllpResult.success} class:error={!mllpResult.success}>
						<div class="result-header">
							<span>{mllpResult.success ? 'Success' : 'Failed'}</span>
							<span>{mllpResult.response_time_ms}ms</span>
						</div>
						{#if mllpResult.error}
							<div class="result-error">{mllpResult.error}</div>
						{/if}
						{#if mllpResult.response}
							<pre class="result-body">{mllpResult.response}</pre>
						{/if}
					</div>
				{/if}
			</div>

		{:else if activeSubTab === 'http'}
			<div class="comm-form">
				<div class="form-row">
					<select bind:value={httpMethod} style="width:90px">
						<option>GET</option><option>POST</option><option>PUT</option><option>DELETE</option><option>PATCH</option>
					</select>
					<input bind:value={httpUrl} placeholder="https://..." style="flex:1" />
				</div>
				<div class="form-row">
					<textarea bind:value={httpHeadersText} rows={2} placeholder="Content-Type: application/json" style="flex:1;font-size:11px"></textarea>
				</div>
				<div class="form-row">
					<textarea bind:value={httpBody} rows={3} placeholder="Request body (or uses current message)" style="flex:1;font-size:11px"></textarea>
				</div>
				<div class="form-actions">
					<button class="btn btn-primary" onclick={handleHttpSend} disabled={httpSending}>
						{httpSending ? 'Sending...' : 'Send Request'}
					</button>
				</div>
				{#if httpResult}
					<div class="result" class:success={httpResult.success} class:error={!httpResult.success}>
						<div class="result-header">
							<span>{httpResult.status_code} {httpResult.status_text}</span>
							<span>{httpResult.response_time_ms}ms</span>
						</div>
						{#if httpResult.error}
							<div class="result-error">{httpResult.error}</div>
						{/if}
						{#if httpResult.body}
							<pre class="result-body">{httpResult.body.substring(0, 2000)}{httpResult.body.length > 2000 ? '...' : ''}</pre>
						{/if}
					</div>
				{/if}
			</div>

		{:else if activeSubTab === 'history'}
			<div class="history-list">
				{#if history.length === 0}
					<div class="comm-empty">No request history</div>
				{:else}
					<div class="history-actions">
						<button class="btn btn-sm" onclick={handleClearHistory}>Clear History</button>
					</div>
					{#each history as entry (entry.id)}
						<div class="history-row">
							<span class="history-type">{entry.profile_type.toUpperCase()}</span>
							<span class="history-status" class:ok={entry.status.startsWith('OK') || entry.status.startsWith('2')} class:fail={entry.status === 'FAILED'}>{entry.status}</span>
							<span class="history-preview">{entry.content_preview}</span>
							<span class="history-time">{entry.response_time_ms}ms</span>
						</div>
					{/each}
				{/if}
			</div>
		{/if}
	</div>
</div>

<style>
	.comm-panel { display: flex; flex-direction: column; height: 100%; background: var(--color-bg-secondary); font-size: 12px; }
	.comm-tabs { display: flex; border-bottom: 1px solid var(--color-border); flex-shrink: 0; }
	.comm-tab { padding: 4px 12px; background: none; border: none; color: var(--color-text-secondary); font-size: 11px; font-family: inherit; cursor: pointer; border-bottom: 2px solid transparent; }
	.comm-tab.active { color: var(--color-text-primary); border-bottom-color: var(--color-accent); }
	.comm-content { flex: 1; overflow-y: auto; padding: 8px; }
	.comm-form { display: flex; flex-direction: column; gap: 6px; }
	.form-row { display: flex; gap: 6px; align-items: center; }
	.form-row label { font-size: 11px; color: var(--color-text-secondary); white-space: nowrap; }
	.form-row input, .form-row select, .form-row textarea { padding: 3px 6px; border: 1px solid var(--color-border); border-radius: 3px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-family: 'JetBrains Mono', monospace; font-size: 12px; }
	.form-row textarea { resize: vertical; }
	.form-actions { display: flex; gap: 6px; align-items: center; }
	.btn { padding: 4px 10px; border: 1px solid var(--color-border); border-radius: 3px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 11px; font-family: inherit; cursor: pointer; }
	.btn:hover { background: var(--color-border); }
	.btn:disabled { opacity: 0.5; cursor: not-allowed; }
	.btn-primary { background: var(--color-accent); color: var(--color-bg-primary); border-color: var(--color-accent); }
	.btn-primary:hover { opacity: 0.9; }
	.btn-sm { padding: 2px 8px; font-size: 10px; }
	.result { margin-top: 6px; border: 1px solid var(--color-border); border-radius: 4px; overflow: hidden; }
	.result.success { border-color: var(--color-success); }
	.result.error { border-color: var(--color-error); }
	.result-header { display: flex; justify-content: space-between; padding: 4px 8px; background: var(--color-bg-tertiary); font-weight: 600; font-size: 11px; }
	.result.success .result-header { color: var(--color-success); }
	.result.error .result-header { color: var(--color-error); }
	.result-error { padding: 4px 8px; color: var(--color-error); font-size: 11px; }
	.result-body { padding: 4px 8px; margin: 0; font-size: 11px; font-family: 'JetBrains Mono', monospace; white-space: pre-wrap; word-break: break-all; max-height: 120px; overflow-y: auto; color: var(--color-text-primary); }
	.comm-empty { padding: 16px; text-align: center; color: var(--color-text-secondary); font-style: italic; }
	.history-list { display: flex; flex-direction: column; gap: 2px; }
	.history-actions { padding: 4px 0; }
	.history-row { display: flex; gap: 8px; align-items: center; padding: 3px 4px; border-bottom: 1px solid var(--color-border); font-size: 11px; }
	.history-type { font-weight: 600; width: 40px; flex-shrink: 0; color: var(--color-accent); }
	.history-status { width: 60px; flex-shrink: 0; font-weight: 600; }
	.history-status.ok { color: var(--color-success); }
	.history-status.fail { color: var(--color-error); }
	.history-preview { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--color-text-secondary); }
	.history-time { flex-shrink: 0; color: var(--color-text-secondary); }
</style>
