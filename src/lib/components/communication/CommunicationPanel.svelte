<script lang="ts">
	import {
		mllpSend, mllpReceive, httpRequest,
		getRequestHistory, clearRequestHistory,
		type MllpSendResult, type HttpResult, type HistoryEntry,
	} from '$lib/ipc/communication';
	import { t, subscribeLocale } from '$lib/i18n';
	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }
	function tr(key: string, params?: Record<string, string | number>): string { void localeVersion; return t(key, params); }

	interface Props {
		currentMessage?: string;
		activeTabLabel?: string;
		onMessageReceived?: (content: string) => void;
	}

	let { currentMessage = '', activeTabLabel = '', onMessageReceived }: Props = $props();

	let activeSubTab = $state<'mllp' | 'http' | 'history'>('mllp');

	// MLLP state
	let mllpHost = $state('localhost');
	let mllpPort = $state(2575);
	let mllpTimeout = $state(30);
	let mllpResult = $state<MllpSendResult | null>(null);
	let mllpSending = $state(false);
	let mllpListening = $state(false);
	let mllpListenPort = $state(2576);
	let mllpShowAdvanced = $state(false);
	// MLLP advanced options
	let mllpResponseTimeout = $state(30);
	let mllpAutoAck = $state(true);
	let mllpEncoding = $state('UTF-8');
	let mllpStartChar = $state('0x0B');
	let mllpEndChar1 = $state('0x1C');
	let mllpEndChar2 = $state('0x0D');
	let mllpRetries = $state(0);
	let mllpRetryDelay = $state(2);

	// HTTP state
	let httpUrl = $state('http://localhost:8080/fhir');
	let httpMethod = $state('POST');
	let httpHeadersText = $state('Content-Type: application/hl7-v2\nAccept: application/hl7-v2');
	let httpBody = $state('');
	let httpResult = $state<HttpResult | null>(null);
	let httpSending = $state(false);
	let httpShowAdvanced = $state(false);
	// HTTP advanced options
	let httpTimeout = $state(30);
	let httpFollowRedirects = $state(true);
	let httpAuth = $state('none');
	let httpAuthUser = $state('');
	let httpAuthPass = $state('');

	// History state
	let history = $state<HistoryEntry[]>([]);
	let selectedHistoryId = $state<string | null>(null);

	// Derived
	let hasMessage = $derived(currentMessage.trim().length > 0);
	let messagePreview = $derived(currentMessage.trim().substring(0, 60) + (currentMessage.length > 60 ? '...' : ''));

	// --- MLLP ---
	async function handleMllpSend() {
		if (!hasMessage) return;
		mllpSending = true;
		mllpResult = null;
		try {
			mllpResult = await mllpSend(mllpHost, mllpPort, currentMessage, mllpTimeout, activeTabLabel || undefined);
		} catch (e) {
			mllpResult = { success: false, response: '', response_time_ms: 0, error: String(e) };
		}
		mllpSending = false;
		loadHistory();
	}

	async function handleMllpListen() {
		mllpListening = true;
		mllpResult = null;
		try {
			const msg = await mllpReceive(mllpListenPort, 120, true);
			mllpResult = {
				success: true,
				response: `Received from ${msg.source_addr} at ${msg.received_at}`,
				response_time_ms: 0,
				error: null,
			};
			onMessageReceived?.(msg.content);
		} catch (e) {
			mllpResult = { success: false, response: '', response_time_ms: 0, error: String(e) };
		}
		mllpListening = false;
		loadHistory();
	}

	// --- HTTP ---
	async function handleHttpSend() {
		httpSending = true;
		httpResult = null;
		try {
			const headers: Record<string, string> = {};
			for (const line of httpHeadersText.split('\n')) {
				const idx = line.indexOf(':');
				if (idx > 0) headers[line.substring(0, idx).trim()] = line.substring(idx + 1).trim();
			}
			const body = httpBody.trim() || currentMessage || undefined;
			httpResult = await httpRequest(httpUrl, httpMethod, headers, body, 30, activeTabLabel || undefined);
		} catch (e) {
			httpResult = { success: false, status_code: 0, status_text: '', headers: {}, body: '', response_time_ms: 0, error: String(e) };
		}
		httpSending = false;
		loadHistory();
	}

	// --- History ---
	async function loadHistory() {
		try { history = await getRequestHistory(50); } catch { /* web mode */ }
	}
	async function handleClearHistory() {
		try { await clearRequestHistory(); history = []; selectedHistoryId = null; } catch { /* */ }
	}
	$effect(() => { loadHistory(); });

	let selectedHistory = $derived(history.find(h => h.id === selectedHistoryId));

	function formatTimestamp(ts: string): string {
		try {
			const d = new Date(ts);
			return d.toLocaleTimeString() + ' ' + d.toLocaleDateString();
		} catch { return ts; }
	}
</script>

<div class="comm-panel">
	<!-- Sub-tabs -->
	<div class="comm-tabs">
		<button class="comm-tab" class:active={activeSubTab === 'mllp'} onclick={() => { activeSubTab = 'mllp'; }}>MLLP</button>
		<button class="comm-tab" class:active={activeSubTab === 'http'} onclick={() => { activeSubTab = 'http'; }}>HTTP</button>
		<button class="comm-tab" class:active={activeSubTab === 'history'} onclick={() => { activeSubTab = 'history'; loadHistory(); }}>
			History {history.length > 0 ? `(${history.length})` : ''}
		</button>
		<!-- Active message indicator -->
		<div class="tab-message-info">
			{#if hasMessage}
				<span class="msg-indicator" title={messagePreview}>
					{activeTabLabel || 'Untitled'}
				</span>
			{:else}
				<span class="msg-indicator empty">No message</span>
			{/if}
		</div>
	</div>

	<div class="comm-content">
		<!-- ==================== MLLP ==================== -->
		{#if activeSubTab === 'mllp'}
			<div class="comm-form">
				<div class="section-label">Connection</div>
				<div class="form-row">
					<label for="mllp-host">Host</label>
					<input id="mllp-host" bind:value={mllpHost} placeholder="localhost" class="input-grow" />
					<label for="mllp-port">Port</label>
					<input id="mllp-port" type="number" bind:value={mllpPort} class="input-sm" />
					<label for="mllp-timeout">Connect Timeout</label>
					<input id="mllp-timeout" type="number" bind:value={mllpTimeout} class="input-xs" />
					<span class="hint">s</span>
				</div>

				<button class="toggle-advanced" onclick={() => { mllpShowAdvanced = !mllpShowAdvanced; }}>
					{mllpShowAdvanced ? '\u25BC' : '\u25B6'} Advanced MLLP Settings
				</button>
				{#if mllpShowAdvanced}
					<div class="advanced-section">
						<div class="form-row">
							<label for="mllp-resptimeout">Response Timeout</label>
							<input id="mllp-resptimeout" type="number" bind:value={mllpResponseTimeout} class="input-xs" />
							<span class="hint">s</span>
							<label for="mllp-encoding">Encoding</label>
							<select id="mllp-encoding" bind:value={mllpEncoding} class="input-method">
								<option>UTF-8</option><option>ISO-8859-1</option><option>ASCII</option><option>Windows-1252</option>
							</select>
						</div>
						<div class="form-row">
							<label for="mllp-startchar">Start Block</label>
							<input id="mllp-startchar" bind:value={mllpStartChar} class="input-sm" title="MLLP start byte (VT = 0x0B)" />
							<label for="mllp-endchar1">End Block</label>
							<input id="mllp-endchar1" bind:value={mllpEndChar1} class="input-sm" title="MLLP end byte 1 (FS = 0x1C)" />
							<label for="mllp-endchar2">End CR</label>
							<input id="mllp-endchar2" bind:value={mllpEndChar2} class="input-sm" title="MLLP end byte 2 (CR = 0x0D)" />
						</div>
						<div class="form-row">
							<label for="mllp-retries">Retries</label>
							<input id="mllp-retries" type="number" min={0} max={10} bind:value={mllpRetries} class="input-xs" />
							<label for="mllp-retrydelay">Retry Delay</label>
							<input id="mllp-retrydelay" type="number" min={1} max={60} bind:value={mllpRetryDelay} class="input-xs" />
							<span class="hint">s</span>
						</div>
						<div class="setting-check">
							<label><input type="checkbox" bind:checked={mllpAutoAck} /> Auto-generate ACK on receive</label>
						</div>
					</div>
				{/if}

				<div class="section-label">Send</div>
				{#if !hasMessage}
					<div class="info-box">No message in active tab to send. Open or paste an HL7 message first.</div>
				{:else}
					<div class="info-box ok">Will send message from tab: <strong>{activeTabLabel || 'Untitled'}</strong> ({currentMessage.length} bytes)</div>
				{/if}
				<div class="form-actions">
					<button class="btn btn-primary" onclick={handleMllpSend} disabled={mllpSending || !hasMessage}>
						{mllpSending ? 'Sending...' : 'Send via MLLP'}
					</button>
					<span class="separator">|</span>
					<button class="btn" onclick={handleMllpListen} disabled={mllpListening}>
						{mllpListening ? `Listening on :${mllpListenPort}...` : 'Listen for incoming'}
					</button>
					<label for="mllp-listen-port">Port</label>
					<input id="mllp-listen-port" type="number" bind:value={mllpListenPort} class="input-sm" />
				</div>

				{#if mllpResult}
					<div class="result" class:success={mllpResult.success} class:error={!mllpResult.success}>
						<div class="result-header">
							<span>{mllpResult.success ? 'OK' : 'FAILED'}</span>
							<span>{mllpHost}:{mllpPort}</span>
							<span>{mllpResult.response_time_ms}ms</span>
						</div>
						{#if mllpResult.error}
							<div class="result-error">{mllpResult.error}</div>
						{/if}
						{#if mllpResult.response}
							<div class="result-label">
								{#if mllpResult.response.includes('MSA|AA')}
									ACK (Accept)
								{:else if mllpResult.response.includes('MSA|AE')}
									NACK (Application Error)
								{:else if mllpResult.response.includes('MSA|AR')}
									NACK (Application Reject)
								{:else}
									Response
								{/if}
							</div>
							<pre class="result-body">{mllpResult.response}</pre>
						{/if}
					</div>
				{/if}
			</div>

		<!-- ==================== HTTP ==================== -->
		{:else if activeSubTab === 'http'}
			<div class="comm-form">
				<div class="section-label">Request</div>
				<div class="form-row">
					<select id="http-method" bind:value={httpMethod} class="input-method">
						<option>GET</option><option>POST</option><option>PUT</option><option>DELETE</option><option>PATCH</option>
					</select>
					<input bind:value={httpUrl} placeholder="https://server/fhir/Patient" class="input-grow" />
				</div>

				<button class="toggle-advanced" onclick={() => { httpShowAdvanced = !httpShowAdvanced; }}>
					{httpShowAdvanced ? '\u25BC' : '\u25B6'} Advanced HTTP Settings
				</button>
				{#if httpShowAdvanced}
					<div class="advanced-section">
						<div class="form-row">
							<label for="http-timeout">Timeout</label>
							<input id="http-timeout" type="number" min={1} max={300} bind:value={httpTimeout} class="input-xs" />
							<span class="hint">s</span>
						</div>
						<div class="setting-check">
							<label><input type="checkbox" bind:checked={httpFollowRedirects} /> Follow Redirects</label>
						</div>
						<div class="form-row">
							<label for="http-auth">Authentication</label>
							<select id="http-auth" bind:value={httpAuth} class="input-method">
								<option value="none">None</option>
								<option value="basic">Basic Auth</option>
								<option value="bearer">Bearer Token</option>
							</select>
						</div>
						{#if httpAuth === 'basic'}
							<div class="form-row">
								<label for="http-user">Username</label>
								<input id="http-user" bind:value={httpAuthUser} class="input-grow" />
								<label for="http-pass">Password</label>
								<input id="http-pass" type="password" bind:value={httpAuthPass} class="input-grow" />
							</div>
						{:else if httpAuth === 'bearer'}
							<div class="form-row">
								<label for="http-token">Token</label>
								<input id="http-token" bind:value={httpAuthUser} placeholder="Bearer token" class="input-grow" />
							</div>
						{/if}
					</div>
				{/if}

				<div class="section-label">Headers</div>
				<textarea bind:value={httpHeadersText} rows={2} placeholder="Content-Type: application/json" class="input-area"></textarea>
				<div class="section-label">Body <span class="hint">(leave empty to send active message)</span></div>
				<textarea bind:value={httpBody} rows={2} placeholder="Custom body, or leave empty to use current tab message" class="input-area"></textarea>

				{#if !httpBody.trim() && hasMessage}
					<div class="info-box ok">Will send body from tab: <strong>{activeTabLabel || 'Untitled'}</strong> ({currentMessage.length} bytes)</div>
				{:else if !httpBody.trim() && !hasMessage}
					<div class="info-box">No body and no active message. Enter a body or open a message.</div>
				{/if}

				<div class="form-actions">
					<button class="btn btn-primary" onclick={handleHttpSend} disabled={httpSending}>
						{httpSending ? 'Sending...' : `${httpMethod} Request`}
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
						{#if Object.keys(httpResult.headers).length > 0}
							<details class="result-details">
								<summary>Response Headers ({Object.keys(httpResult.headers).length})</summary>
								<div class="result-headers">
									{#each Object.entries(httpResult.headers) as [k, v]}
										<div class="header-row"><span class="hk">{k}:</span> <span class="hv">{v}</span></div>
									{/each}
								</div>
							</details>
						{/if}
						{#if httpResult.body}
							<pre class="result-body">{httpResult.body.substring(0, 5000)}{httpResult.body.length > 5000 ? '\n...truncated...' : ''}</pre>
						{/if}
					</div>
				{/if}
			</div>

		<!-- ==================== HISTORY ==================== -->
		{:else if activeSubTab === 'history'}
			<div class="history-container">
				<div class="history-list">
					{#if history.length === 0}
						<div class="comm-empty">No request history yet</div>
					{:else}
						<div class="history-toolbar">
							<span class="history-count">{history.length} entries</span>
							<button class="btn btn-sm" onclick={handleClearHistory}>Clear All</button>
						</div>
						{#each history as entry (entry.id)}
							<button
								class="history-row"
								class:selected={selectedHistoryId === entry.id}
								onclick={() => { selectedHistoryId = selectedHistoryId === entry.id ? null : entry.id; }}
							>
								<span class="h-type">{entry.profile_type.toUpperCase()}</span>
								<span class="h-dir">{entry.direction === 'send' ? '\u2191' : '\u2193'}</span>
								<span class="h-status" class:ok={entry.status.startsWith('OK') || entry.status.startsWith('2')} class:fail={entry.status === 'FAILED'}>{entry.status}</span>
								<span class="h-target">{entry.profile_name}</span>
								<span class="h-time">{entry.response_time_ms}ms</span>
								<span class="h-ts">{formatTimestamp(entry.timestamp)}</span>
							</button>
						{/each}
					{/if}
				</div>
				{#if selectedHistory}
					<div class="history-detail">
						<div class="detail-header">Request Detail</div>
						<div class="detail-grid">
							<span class="dl">Protocol</span><span class="dv">{selectedHistory.profile_type.toUpperCase()}</span>
							<span class="dl">Direction</span><span class="dv">{selectedHistory.direction === 'send' ? 'Outgoing' : 'Incoming'}</span>
							<span class="dl">Target</span><span class="dv">{selectedHistory.profile_name}</span>
							<span class="dl">Status</span><span class="dv">{selectedHistory.status}</span>
							<span class="dl">Response Time</span><span class="dv">{selectedHistory.response_time_ms}ms</span>
							<span class="dl">Timestamp</span><span class="dv">{formatTimestamp(selectedHistory.timestamp)}</span>
						</div>
						<div class="detail-header">Content Preview</div>
						<pre class="detail-body">{selectedHistory.content_preview}</pre>
					</div>
				{/if}
			</div>
		{/if}
	</div>
</div>

<style>
	.comm-panel { display: flex; flex-direction: column; height: 100%; background: var(--color-bg-secondary); font-size: 12px; overflow: hidden; }

	/* Tabs */
	.comm-tabs { display: flex; align-items: center; border-bottom: 1px solid var(--color-border); flex-shrink: 0; padding-right: 8px; }
	.comm-tab { padding: 5px 14px; background: none; border: none; color: var(--color-text-secondary); font-size: 11px; font-family: inherit; cursor: pointer; border-bottom: 2px solid transparent; }
	.comm-tab.active { color: var(--color-text-primary); border-bottom-color: var(--color-accent); }
	.tab-message-info { margin-left: auto; font-size: 10px; }
	.msg-indicator { background: var(--color-bg-tertiary); padding: 2px 8px; border-radius: 10px; color: var(--color-success); }
	.msg-indicator.empty { color: var(--color-text-secondary); opacity: 0.5; }

	/* Content */
	.comm-content { flex: 1; overflow-y: auto; padding: 8px 10px; }
	.comm-form { display: flex; flex-direction: column; gap: 5px; }
	.section-label { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; color: var(--color-text-secondary); margin-top: 4px; }
	.section-label .hint { font-weight: 400; text-transform: none; letter-spacing: 0; opacity: 0.6; }

	/* Form elements */
	.form-row { display: flex; gap: 6px; align-items: center; }
	.form-row label { font-size: 10px; color: var(--color-text-secondary); white-space: nowrap; }
	.form-row input, .form-row select, .input-area { padding: 4px 6px; border: 1px solid var(--color-border); border-radius: 3px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-family: 'JetBrains Mono', monospace; font-size: 11px; }
	.input-grow { flex: 1; }
	.input-sm { width: 80px; }
	.input-xs { width: 55px; }
	.input-method { width: 80px; }
	.input-area { width: 100%; resize: vertical; font-size: 11px; }
	.info-box { padding: 4px 8px; border-radius: 3px; font-size: 11px; background: var(--color-bg-tertiary); color: var(--color-text-secondary); border-left: 3px solid var(--color-border); }
	.info-box.ok { border-left-color: var(--color-success); color: var(--color-text-primary); }
	.form-actions { display: flex; gap: 6px; align-items: center; margin-top: 2px; }
	.separator { color: var(--color-border); margin: 0 2px; }

	/* Buttons */
	.btn { padding: 4px 10px; border: 1px solid var(--color-border); border-radius: 3px; background: var(--color-bg-tertiary); color: var(--color-text-primary); font-size: 11px; font-family: inherit; cursor: pointer; white-space: nowrap; }
	.btn:hover { background: var(--color-border); }
	.btn:disabled { opacity: 0.4; cursor: not-allowed; }
	.btn-primary { background: var(--color-accent); color: var(--color-bg-primary); border-color: var(--color-accent); }
	.btn-primary:hover:not(:disabled) { opacity: 0.9; }
	.btn-sm { padding: 2px 8px; font-size: 10px; }

	/* Results */
	.result { margin-top: 6px; border: 1px solid var(--color-border); border-radius: 4px; overflow: hidden; }
	.result.success { border-color: var(--color-success); }
	.result.error { border-color: var(--color-error); }
	.result-header { display: flex; justify-content: space-between; padding: 4px 8px; background: var(--color-bg-tertiary); font-weight: 600; font-size: 11px; gap: 12px; }
	.result.success .result-header { color: var(--color-success); }
	.result.error .result-header { color: var(--color-error); }
	.result-error { padding: 4px 8px; color: var(--color-error); font-size: 11px; }
	.result-label { padding: 3px 8px; font-size: 10px; font-weight: 600; text-transform: uppercase; color: var(--color-text-secondary); background: var(--color-bg-primary); }
	.result-body { padding: 4px 8px; margin: 0; font-size: 11px; font-family: 'JetBrains Mono', monospace; white-space: pre-wrap; word-break: break-all; max-height: 150px; overflow-y: auto; color: var(--color-text-primary); }
	.result-details { border-top: 1px solid var(--color-border); }
	.result-details summary { padding: 3px 8px; font-size: 10px; cursor: pointer; color: var(--color-text-secondary); }
	.result-headers { padding: 2px 8px 4px; }
	.header-row { font-size: 10px; font-family: 'JetBrains Mono', monospace; }
	.hk { color: var(--color-accent); }
	.hv { color: var(--color-text-secondary); }

	/* History */
	.history-container { display: flex; flex-direction: column; gap: 6px; height: 100%; }
	.history-list { flex: 1; overflow-y: auto; }
	.history-toolbar { display: flex; justify-content: space-between; align-items: center; padding: 2px 0; }
	.history-count { font-size: 10px; color: var(--color-text-secondary); }
	.history-row { display: flex; gap: 6px; align-items: center; width: 100%; padding: 4px 6px; background: none; border: none; border-bottom: 1px solid var(--color-border); font-size: 11px; font-family: inherit; text-align: left; cursor: pointer; color: var(--color-text-primary); }
	.history-row:hover { background: var(--color-bg-tertiary); }
	.history-row.selected { background: var(--color-bg-tertiary); border-left: 2px solid var(--color-accent); }
	.h-type { font-weight: 700; width: 36px; flex-shrink: 0; color: var(--color-accent); font-family: 'JetBrains Mono', monospace; font-size: 10px; }
	.h-dir { width: 14px; flex-shrink: 0; }
	.h-status { width: 70px; flex-shrink: 0; font-weight: 600; font-family: 'JetBrains Mono', monospace; font-size: 10px; }
	.h-status.ok { color: var(--color-success); }
	.h-status.fail { color: var(--color-error); }
	.h-target { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--color-text-secondary); }
	.h-time { width: 50px; flex-shrink: 0; text-align: right; color: var(--color-text-secondary); font-family: 'JetBrains Mono', monospace; font-size: 10px; }
	.h-ts { width: 130px; flex-shrink: 0; text-align: right; font-size: 10px; color: var(--color-text-secondary); }

	/* History detail */
	.history-detail { border-top: 1px solid var(--color-border); padding-top: 6px; flex-shrink: 0; max-height: 50%; overflow-y: auto; }
	.detail-header { font-size: 10px; font-weight: 700; text-transform: uppercase; color: var(--color-text-secondary); margin-bottom: 4px; }
	.detail-grid { display: grid; grid-template-columns: 100px 1fr; gap: 2px 8px; font-size: 11px; margin-bottom: 8px; }
	.dl { color: var(--color-text-secondary); }
	.dv { color: var(--color-text-primary); font-family: 'JetBrains Mono', monospace; }
	.detail-body { font-size: 11px; font-family: 'JetBrains Mono', monospace; white-space: pre-wrap; word-break: break-all; margin: 0; padding: 4px; background: var(--color-bg-primary); border-radius: 3px; max-height: 80px; overflow-y: auto; color: var(--color-text-primary); }
	.comm-empty { padding: 16px; text-align: center; color: var(--color-text-secondary); font-style: italic; }

	/* Advanced toggle */
	.toggle-advanced { background: none; border: none; color: var(--color-accent); font-size: 11px; font-family: inherit; cursor: pointer; padding: 3px 0; text-align: left; }
	.toggle-advanced:hover { text-decoration: underline; }
	.advanced-section { padding: 6px 0 6px 12px; border-left: 2px solid var(--color-accent); margin: 2px 0; display: flex; flex-direction: column; gap: 5px; }
	.setting-check { margin: 2px 0; }
	.setting-check label { font-size: 11px; color: var(--color-text-primary); display: flex; align-items: center; gap: 5px; cursor: pointer; }
	.setting-check input[type="checkbox"] { accent-color: var(--color-accent); }
</style>
