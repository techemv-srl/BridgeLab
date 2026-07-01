<script lang="ts">
	import {
		mllpSend, httpRequest,
		mllpListenStart, mllpListenStop, mllpListenStatus,
		getRequestHistory, clearRequestHistory,
		saveConnectionProfile, getConnectionProfiles, deleteConnectionProfile,
		type MllpSendResult, type HttpResult, type HistoryEntry,
		type ListenerStatus, type MllpReceivedEvent, type ConnectionProfile,
	} from '$lib/ipc/communication';
	import { parseUpgradeError } from '$lib/ipc/licensing';
	import { listen, type UnlistenFn } from '@tauri-apps/api/event';
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
	let mllpListenPort = $state(2576);
	let mllpShowAdvanced = $state(false);

	// Persistent listener state — driven by the backend via mllp:received events
	let listenBindAddress = $state('0.0.0.0');
	let listenAckCode = $state('AA');
	let listenReadTimeout = $state(30);
	let listenEncoding = $state('UTF-8');
	let listenStatus = $state<ListenerStatus>({ running: false, port: null, bind_address: null });
	let listenError = $state<string | null>(null);
	let listenInboxCount = $state(0);
	let listenShowSettings = $state(false);

	// --- Listener console: rolling log of received messages and errors ---
	interface ConsoleEntry {
		id: number;
		time: string;            // local HH:MM:SS
		kind: 'msg' | 'error';
		peer?: string;
		bytes?: number;
		ack?: string | null;     // ACK code sent, null = auto-ACK off
		encoding?: string;
		snippet?: string;        // first line, always retained
		content?: string;        // full message for click-to-open; evicted by byte budget
		text?: string;           // error text
	}
	const CONSOLE_CAP = 200;
	// Total bytes of full message contents retained for click-to-open. The
	// backend accepts payloads up to 10 MiB each, so an entry cap alone could
	// pin gigabytes in renderer state during an unattended high-volume
	// session. Oldest entries lose `content` first (snippet row stays).
	const CONSOLE_CONTENT_BUDGET = 32 * 1024 * 1024;
	let consoleEntries = $state<ConsoleEntry[]>([]);
	let autoOpenReceived = $state(true);
	let consoleNextId = 0;

	function pushConsole(entry: Omit<ConsoleEntry, 'id'>) {
		// Newest first; cap so an unattended listener can't grow unbounded.
		const entries = [{ ...entry, id: consoleNextId++ }, ...consoleEntries.slice(0, CONSOLE_CAP - 1)];
		// Enforce the byte budget newest→oldest: once cumulative content size
		// exceeds it, strip full contents from the older entries.
		let retained = 0;
		for (let i = 0; i < entries.length; i++) {
			const e = entries[i];
			if (!e.content) continue;
			retained += e.bytes ?? e.content.length;
			if (retained > CONSOLE_CONTENT_BUDGET && i > 0) {
				entries[i] = { ...e, content: undefined };
			}
		}
		consoleEntries = entries;
	}

	function clearConsole() {
		consoleEntries = [];
		listenInboxCount = 0;
	}

	function localTime(iso: string): string {
		try { return new Date(iso).toLocaleTimeString(); } catch { return iso; }
	}

	function msgSnippet(content: string): string {
		const firstLine = content.split(/\r|\n/, 1)[0] ?? '';
		return firstLine.substring(0, 80);
	}
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

	// --- Connection profiles ---
	// The backend (DB table + save/get/delete commands) predates this UI;
	// until now the feature was documented in the manual but unreachable.
	let profiles = $state<ConnectionProfile[]>([]);
	let mllpProfileId = $state('');
	let httpProfileId = $state('');
	let mllpProfileName = $state('');
	let httpProfileName = $state('');

	let mllpProfiles = $derived(profiles.filter((p) => p.profile_type === 'mllp'));
	let httpProfiles = $derived(profiles.filter((p) => p.profile_type === 'http'));

	async function loadProfiles() {
		try { profiles = await getConnectionProfiles(); } catch { /* web mode */ }
	}
	$effect(() => { loadProfiles(); });

	function applyMllpProfile(id: string) {
		const p = profiles.find((pp) => pp.id === id);
		if (!p) return;
		mllpHost = p.host;
		mllpPort = p.port;
		mllpTimeout = p.timeout_secs;
		mllpAutoAck = p.auto_ack;
	}

	function applyHttpProfile(id: string) {
		const p = profiles.find((pp) => pp.id === id);
		if (!p) return;
		if (p.url) httpUrl = p.url;
		if (p.headers) httpHeadersText = p.headers;
		httpTimeout = p.timeout_secs;
	}

	async function saveProfile(kind: 'mllp' | 'http') {
		const name = (kind === 'mllp' ? mllpProfileName : httpProfileName).trim();
		if (!name) return;
		// Same name + type overwrites (upsert) instead of piling up duplicates.
		const existing = profiles.find((p) => p.name === name && p.profile_type === kind);
		const profile: ConnectionProfile = kind === 'mllp'
			? {
				id: existing?.id ?? crypto.randomUUID(),
				name, profile_type: 'mllp',
				host: mllpHost, port: mllpPort, timeout_secs: mllpTimeout,
				url: null, headers: null, auto_ack: mllpAutoAck,
			}
			: {
				id: existing?.id ?? crypto.randomUUID(),
				name, profile_type: 'http',
				host: '', port: 0, timeout_secs: httpTimeout,
				url: httpUrl, headers: httpHeadersText, auto_ack: false,
			};
		try {
			await saveConnectionProfile(profile);
			await loadProfiles();
			if (kind === 'mllp') { mllpProfileId = profile.id; mllpProfileName = ''; }
			else { httpProfileId = profile.id; httpProfileName = ''; }
		} catch { /* web mode */ }
	}

	async function deleteProfile(kind: 'mllp' | 'http') {
		const id = kind === 'mllp' ? mllpProfileId : httpProfileId;
		if (!id) return;
		try {
			await deleteConnectionProfile(id);
			if (kind === 'mllp') mllpProfileId = '';
			else httpProfileId = '';
			await loadProfiles();
		} catch { /* web mode */ }
	}

	// Derived
	let hasMessage = $derived(currentMessage.trim().length > 0);
	let messagePreview = $derived(currentMessage.trim().substring(0, 60) + (currentMessage.length > 60 ? '...' : ''));

	/** Turn a raw IPC error into a user-facing message; feature-gate errors
	 *  become the localized upgrade prompt instead of the raw
	 *  "UPGRADE_REQUIRED:feature:tier:..." string. */
	function friendlyError(e: unknown): string {
		const upgrade = parseUpgradeError(e);
		return upgrade ? tr('upgrade.required', { tier: upgrade.tier }) : String(e);
	}

	// --- MLLP ---
	async function handleMllpSend() {
		if (!hasMessage) return;
		mllpSending = true;
		mllpResult = null;

		// Retries: 0 = single attempt. Each retry waits mllpRetryDelay seconds.
		const attempts = 1 + Math.max(0, Math.min(10, mllpRetries));
		for (let attempt = 0; attempt < attempts; attempt++) {
			if (attempt > 0) {
				await new Promise((r) => setTimeout(r, Math.max(1, mllpRetryDelay) * 1000));
			}
			try {
				mllpResult = await mllpSend(mllpHost, mllpPort, currentMessage, {
					timeoutSecs: mllpTimeout,
					responseTimeoutSecs: mllpResponseTimeout,
					encoding: mllpEncoding,
					startChar: mllpStartChar,
					endChar1: mllpEndChar1,
					endChar2: mllpEndChar2,
					profileName: activeTabLabel || undefined,
				});
			} catch (e) {
				mllpResult = { success: false, response: '', response_time_ms: 0, error: friendlyError(e) };
				if (parseUpgradeError(e)) break; // retrying won't change the license
			}
			if (mllpResult?.success) break;
		}
		mllpSending = false;
		loadHistory();
	}

	async function handleListenStart() {
		listenError = null;
		try {
			listenStatus = await mllpListenStart({
				port: mllpListenPort,
				bind_address: listenBindAddress,
				auto_ack: mllpAutoAck,
				ack_code: listenAckCode,
				read_timeout_secs: listenReadTimeout,
				encoding: listenEncoding,
			});
		} catch (e) {
			listenError = friendlyError(e);
		}
	}

	async function handleListenStop() {
		try {
			listenStatus = await mllpListenStop();
		} catch (e) {
			listenError = String(e);
		}
	}

	// Subscribe to the backend event stream once. mllp:received fires for
	// every successfully-decoded incoming message; mllp:listen_error for any
	// per-connection or accept-loop failure (the listener itself stays up
	// unless accept() fatally fails).
	let unlistenReceived: UnlistenFn | null = null;
	let unlistenError: UnlistenFn | null = null;
	$effect(() => {
		if (typeof window === 'undefined') return;
		(async () => {
			// Sync initial status (in case the listener was running before this
			// component mounted, e.g. after a frontend reload with backend alive).
			try { listenStatus = await mllpListenStatus(); } catch {}

			unlistenReceived = await listen<MllpReceivedEvent>(
				'mllp:received',
				(ev) => {
					listenInboxCount += 1;
					pushConsole({
						time: localTime(ev.payload.received_at),
						kind: 'msg',
						peer: ev.payload.source_addr,
						bytes: ev.payload.bytes,
						ack: ev.payload.ack_code,
						encoding: ev.payload.encoding,
						snippet: msgSnippet(ev.payload.content),
						content: ev.payload.content,
					});
					if (autoOpenReceived) onMessageReceived?.(ev.payload.content);
					loadHistory();
				},
			);
			unlistenError = await listen<string>('mllp:listen_error', (ev) => {
				listenError = ev.payload;
				pushConsole({
					time: new Date().toLocaleTimeString(),
					kind: 'error',
					text: ev.payload,
				});
			});
		})();

		return () => {
			unlistenReceived?.();
			unlistenError?.();
		};
	});

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
			// Authentication from Advanced settings. An explicit Authorization
			// header in the Headers box wins over the dropdown.
			const hasExplicitAuth = Object.keys(headers).some((k) => k.toLowerCase() === 'authorization');
			if (!hasExplicitAuth) {
				if (httpAuth === 'basic' && httpAuthUser) {
					headers['Authorization'] = 'Basic ' + btoa(`${httpAuthUser}:${httpAuthPass}`);
				} else if (httpAuth === 'bearer' && httpAuthUser) {
					headers['Authorization'] = 'Bearer ' + httpAuthUser;
				}
			}
			const body = httpBody.trim() || currentMessage || undefined;
			httpResult = await httpRequest(
				httpUrl, httpMethod, headers, body,
				httpTimeout, httpFollowRedirects, activeTabLabel || undefined,
			);
		} catch (e) {
			httpResult = { success: false, status_code: 0, status_text: '', headers: {}, body: '', response_time_ms: 0, error: friendlyError(e) };
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
			{tr('comm.history')} {history.length > 0 ? `(${history.length})` : ''}
		</button>
		<!-- Active message indicator -->
		<div class="tab-message-info">
			{#if hasMessage}
				<span class="msg-indicator" title={messagePreview}>
					{activeTabLabel || 'Untitled'}
				</span>
			{:else}
				<span class="msg-indicator empty">{tr('comm.noMessageShort')}</span>
			{/if}
		</div>
	</div>

	<div class="comm-content">
		<!-- ==================== MLLP ==================== -->
		{#if activeSubTab === 'mllp'}
			<div class="comm-form">
				<div class="section-label">{tr('comm.connection')}</div>
				<div class="form-row">
					<label for="mllp-host">{tr('comm.host')}</label>
					<input id="mllp-host" bind:value={mllpHost} placeholder="localhost" class="input-grow" />
					<label for="mllp-port">{tr('comm.port')}</label>
					<input id="mllp-port" type="number" bind:value={mllpPort} class="input-sm" />
					<label for="mllp-timeout">{tr('comm.connectTimeout')}</label>
					<input id="mllp-timeout" type="number" bind:value={mllpTimeout} class="input-xs" />
					<span class="hint">s</span>
				</div>
				<div class="form-row">
					<label for="mllp-profile">{tr('comm.profile')}</label>
					<select id="mllp-profile" bind:value={mllpProfileId}
						onchange={() => applyMllpProfile(mllpProfileId)}
						style="min-width: 130px; padding: 4px 6px;">
						<option value="">\u2014</option>
						{#each mllpProfiles as p (p.id)}
							<option value={p.id}>{p.name}</option>
						{/each}
					</select>
					{#if mllpProfileId}
						<button class="btn btn-sm" onclick={() => deleteProfile('mllp')}>{tr('comm.profileDelete')}</button>
					{/if}
					<input class="input-grow" bind:value={mllpProfileName}
						placeholder={tr('comm.profileNamePlaceholder')} />
					<button class="btn btn-sm" onclick={() => saveProfile('mllp')} disabled={!mllpProfileName.trim()}>
						{tr('comm.profileSave')}
					</button>
				</div>

				<button class="toggle-advanced" onclick={() => { mllpShowAdvanced = !mllpShowAdvanced; }}>
					{mllpShowAdvanced ? '\u25BC' : '\u25B6'} {tr('comm.advancedMllp')}
				</button>
				{#if mllpShowAdvanced}
					<div class="advanced-section">
						<div class="form-row">
							<label for="mllp-resptimeout">{tr('comm.responseTimeout')}</label>
							<input id="mllp-resptimeout" type="number" min={1} max={600} bind:value={mllpResponseTimeout} class="input-xs" />
							<span class="hint">s</span>
							<label for="mllp-encoding">{tr('comm.encoding')}</label>
							<select id="mllp-encoding" bind:value={mllpEncoding}
								style="min-width: 150px; padding: 4px 6px;">
								<option value="UTF-8">UTF-8 ({tr('comm.encUnicode')})</option>
								<option value="ISO-8859-1">ISO-8859-1 ({tr('comm.encLatin1')})</option>
								<option value="ISO-8859-2">ISO-8859-2 ({tr('comm.encLatin2')})</option>
								<option value="ISO-8859-15">ISO-8859-15 ({tr('comm.encLatin9')})</option>
								<option value="windows-1252">windows-1252 ({tr('comm.encWesternEU')})</option>
								<option value="windows-1250">windows-1250 ({tr('comm.encCentralEU')})</option>
								<option value="windows-1251">windows-1251 ({tr('comm.encCyrillic')})</option>
								<option value="ASCII">ASCII (US)</option>
							</select>
						</div>
						<div class="form-row">
							<label for="mllp-startchar">{tr('comm.startBlock')}</label>
							<input id="mllp-startchar" bind:value={mllpStartChar} class="input-sm" title="MLLP start byte (VT = 0x0B)" />
							<label for="mllp-endchar1">{tr('comm.endBlock')}</label>
							<input id="mllp-endchar1" bind:value={mllpEndChar1} class="input-sm" title="MLLP end byte 1 (FS = 0x1C)" />
							<label for="mllp-endchar2">{tr('comm.endCr')}</label>
							<input id="mllp-endchar2" bind:value={mllpEndChar2} class="input-sm" title="MLLP end byte 2 (CR = 0x0D)" />
						</div>
						<div class="form-row">
							<label for="mllp-retries">{tr('comm.retries')}</label>
							<input id="mllp-retries" type="number" min={0} max={10} bind:value={mllpRetries} class="input-xs" />
							<label for="mllp-retrydelay">{tr('comm.retryDelay')}</label>
							<input id="mllp-retrydelay" type="number" min={1} max={60} bind:value={mllpRetryDelay} class="input-xs" />
							<span class="hint">s</span>
						</div>
						<div class="setting-check">
							<label><input type="checkbox" bind:checked={mllpAutoAck} /> {tr('comm.listenAutoAck')}</label>
						</div>
					</div>
				{/if}

				<div class="section-label">{tr('comm.send')}</div>
				{#if !hasMessage}
					<div class="info-box">{tr('comm.noMessage')}</div>
				{:else}
					<div class="info-box ok">{tr('comm.willSend', { tab: activeTabLabel || tr('editor.untitled'), size: currentMessage.length })}</div>
				{/if}
				<div class="form-actions">
					<button class="btn btn-primary" onclick={handleMllpSend} disabled={mllpSending || !hasMessage}>
						{mllpSending ? tr('comm.sending') : tr('comm.sendViaMllp')}
					</button>
				</div>

				<div class="section-label">{tr('comm.listen')}</div>
				<div class="form-row">
					{#if !listenStatus.running}
						<button class="btn" onclick={handleListenStart}>
							{tr('comm.startListening')}
						</button>
						<label for="mllp-listen-port">{tr('comm.port')}</label>
						<input id="mllp-listen-port" type="number" bind:value={mllpListenPort} class="input-sm" />
						<label for="mllp-listen-bind">{tr('comm.bind')}</label>
						<input id="mllp-listen-bind" type="text" bind:value={listenBindAddress} class="input-sm" placeholder="0.0.0.0" />
						<button class="btn-link" onclick={() => listenShowSettings = !listenShowSettings}>
							{listenShowSettings ? tr('comm.listenHideSettings') + ' ▲' : tr('comm.listenSettings') + ' ▼'}
						</button>
					{:else}
						<button class="btn btn-danger" onclick={handleListenStop}>
							{tr('comm.stopListening')}
						</button>
						<span class="status-pill running">
							{tr('comm.listenStatus', {
								addr: listenStatus.bind_address ?? '',
								port: listenStatus.port ?? '',
								count: listenInboxCount,
							})}
						</span>
					{/if}
				</div>

				{#if listenShowSettings && !listenStatus.running}
					<div class="advanced-options">
						<div class="form-row">
							<label for="mllp-listen-ack-code">{tr('comm.listenAckCode')}</label>
							<select id="mllp-listen-ack-code" bind:value={listenAckCode}
								style="min-width: 130px; padding: 4px 6px;">
								<option value="AA">{tr('comm.listenAckAA')}</option>
								<option value="AE">{tr('comm.listenAckAE')}</option>
								<option value="AR">{tr('comm.listenAckAR')}</option>
							</select>
						</div>
						<div class="form-row">
							<label for="mllp-listen-encoding">{tr('comm.encoding')}</label>
							<select id="mllp-listen-encoding" bind:value={listenEncoding}
								style="min-width: 150px; padding: 4px 6px;">
								<option value="UTF-8">UTF-8 ({tr('comm.encUnicode')})</option>
								<option value="ISO-8859-1">ISO-8859-1 ({tr('comm.encLatin1')})</option>
								<option value="ISO-8859-2">ISO-8859-2 ({tr('comm.encLatin2')})</option>
								<option value="ISO-8859-15">ISO-8859-15 ({tr('comm.encLatin9')})</option>
								<option value="windows-1252">windows-1252 ({tr('comm.encWesternEU')})</option>
								<option value="windows-1250">windows-1250 ({tr('comm.encCentralEU')})</option>
								<option value="windows-1251">windows-1251 ({tr('comm.encCyrillic')})</option>
								<option value="ASCII">ASCII (US)</option>
							</select>
						</div>
						<div class="form-row">
							<label for="mllp-listen-read-timeout">{tr('comm.listenReadTimeout')}</label>
							<input id="mllp-listen-read-timeout" type="number" min={1} max={600}
								bind:value={listenReadTimeout} class="input-xs" />
							<span class="hint">s</span>
						</div>
						<div class="setting-check">
							<label><input type="checkbox" bind:checked={mllpAutoAck} /> {tr('comm.listenAutoAck')}</label>
						</div>
						<div class="setting-check">
							<label><input type="checkbox" bind:checked={autoOpenReceived} /> {tr('comm.consoleAutoOpen')}</label>
						</div>
					</div>
				{/if}

				{#if listenError}
					<div class="result error">
						<div class="result-header"><span>{tr('comm.listenError')}</span></div>
						<div class="result-body">{listenError}</div>
					</div>
				{/if}

				<!-- Listener console: live log of received messages / errors -->
				{#if listenStatus.running || consoleEntries.length > 0}
					<div class="console">
						<div class="console-header">
							<span class="console-title">{tr('comm.console')}</span>
							<span class="console-count">{consoleEntries.length}</span>
							<button class="btn btn-sm" onclick={clearConsole} disabled={consoleEntries.length === 0}>
								{tr('comm.consoleClear')}
							</button>
						</div>
						{#if consoleEntries.length === 0}
							<div class="console-empty">{tr('comm.consoleEmpty')}</div>
						{:else}
							<div class="console-list">
								{#each consoleEntries as entry (entry.id)}
									{#if entry.kind === 'msg'}
										<button
											class="console-row"
											class:no-content={!entry.content}
											disabled={!entry.content}
											title={entry.content ? tr('comm.consoleOpenHint') : tr('comm.consoleEvicted')}
											onclick={() => entry.content && onMessageReceived?.(entry.content)}
										>
											<span class="c-time">{entry.time}</span>
											<span class="c-peer">{entry.peer}</span>
											<span class="c-bytes">{entry.bytes} B</span>
											<span class="c-ack" class:ack-ok={entry.ack === 'AA'} class:ack-err={entry.ack === 'AE' || entry.ack === 'AR'}>
												{entry.ack ?? '—'}
											</span>
											<span class="c-enc">{entry.encoding}</span>
											<span class="c-snippet">{entry.snippet}</span>
										</button>
									{:else}
										<div class="console-row console-error">
											<span class="c-time">{entry.time}</span>
											<span class="c-err-text">{entry.text}</span>
										</div>
									{/if}
								{/each}
							</div>
						{/if}
					</div>
				{/if}

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
				<div class="section-label">{tr('comm.request')}</div>
				<div class="form-row">
					<select id="http-method" bind:value={httpMethod} class="input-method">
						<option>GET</option><option>POST</option><option>PUT</option><option>DELETE</option><option>PATCH</option>
					</select>
					<input bind:value={httpUrl} placeholder="https://server/fhir/Patient" class="input-grow" />
				</div>
				<div class="form-row">
					<label for="http-profile">{tr('comm.profile')}</label>
					<select id="http-profile" bind:value={httpProfileId}
						onchange={() => applyHttpProfile(httpProfileId)}
						style="min-width: 130px; padding: 4px 6px;">
						<option value="">\u2014</option>
						{#each httpProfiles as p (p.id)}
							<option value={p.id}>{p.name}</option>
						{/each}
					</select>
					{#if httpProfileId}
						<button class="btn btn-sm" onclick={() => deleteProfile('http')}>{tr('comm.profileDelete')}</button>
					{/if}
					<input class="input-grow" bind:value={httpProfileName}
						placeholder={tr('comm.profileNamePlaceholder')} />
					<button class="btn btn-sm" onclick={() => saveProfile('http')} disabled={!httpProfileName.trim()}>
						{tr('comm.profileSave')}
					</button>
				</div>

				<button class="toggle-advanced" onclick={() => { httpShowAdvanced = !httpShowAdvanced; }}>
					{httpShowAdvanced ? '\u25BC' : '\u25B6'} {tr('comm.advancedHttp')}
				</button>
				{#if httpShowAdvanced}
					<div class="advanced-section">
						<div class="form-row">
							<label for="http-timeout">{tr('comm.timeout')}</label>
							<input id="http-timeout" type="number" min={1} max={300} bind:value={httpTimeout} class="input-xs" />
							<span class="hint">s</span>
						</div>
						<div class="setting-check">
							<label><input type="checkbox" bind:checked={httpFollowRedirects} /> {tr('comm.followRedirects')}</label>
						</div>
						<div class="form-row">
							<label for="http-auth">{tr('comm.auth')}</label>
							<select id="http-auth" bind:value={httpAuth} class="input-method">
								<option value="none">{tr('comm.authNone')}</option>
								<option value="basic">{tr('comm.authBasic')}</option>
								<option value="bearer">{tr('comm.authBearer')}</option>
							</select>
						</div>
						{#if httpAuth === 'basic'}
							<div class="form-row">
								<label for="http-user">{tr('comm.username')}</label>
								<input id="http-user" bind:value={httpAuthUser} class="input-grow" />
								<label for="http-pass">{tr('comm.password')}</label>
								<input id="http-pass" type="password" bind:value={httpAuthPass} class="input-grow" />
							</div>
						{:else if httpAuth === 'bearer'}
							<div class="form-row">
								<label for="http-token">{tr('comm.token')}</label>
								<input id="http-token" bind:value={httpAuthUser} placeholder="Bearer token" class="input-grow" />
							</div>
						{/if}
					</div>
				{/if}

				<div class="section-label">{tr('comm.headers')}</div>
				<textarea bind:value={httpHeadersText} rows={2} placeholder="Content-Type: application/json" class="input-area"></textarea>
				<div class="section-label">{tr('comm.body')} <span class="hint">{tr('comm.bodyHint')}</span></div>
				<textarea bind:value={httpBody} rows={2} placeholder={tr('comm.bodyPlaceholder')} class="input-area"></textarea>

				{#if !httpBody.trim() && hasMessage}
					<div class="info-box ok">{tr('comm.willSend', { tab: activeTabLabel || tr('editor.untitled'), size: currentMessage.length })}</div>
				{:else if !httpBody.trim() && !hasMessage}
					<div class="info-box">{tr('comm.noBodyNoMessage')}</div>
				{/if}

				<div class="form-actions">
					<button class="btn btn-primary" onclick={handleHttpSend} disabled={httpSending}>
						{httpSending ? tr('comm.sending') : `${httpMethod} ${tr('comm.request')}`}
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
						<div class="comm-empty">{tr('comm.noHistory')}</div>
					{:else}
						<div class="history-toolbar">
							<span class="history-count">{tr('comm.entries', { count: history.length })}</span>
							<button class="btn btn-sm" onclick={handleClearHistory}>{tr('comm.clearHistory')}</button>
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
						<div class="detail-header">{tr('comm.requestDetail')}</div>
						<div class="detail-grid">
							<span class="dl">{tr('comm.protocol')}</span><span class="dv">{selectedHistory.profile_type.toUpperCase()}</span>
							<span class="dl">{tr('comm.direction')}</span><span class="dv">{selectedHistory.direction === 'send' ? tr('comm.outgoing') : tr('comm.incoming')}</span>
							<span class="dl">{tr('comm.target')}</span><span class="dv">{selectedHistory.profile_name}</span>
							<span class="dl">{tr('comm.status')}</span><span class="dv">{selectedHistory.status}</span>
							<span class="dl">{tr('comm.responseTime')}</span><span class="dv">{selectedHistory.response_time_ms}ms</span>
							<span class="dl">{tr('comm.timestamp')}</span><span class="dv">{formatTimestamp(selectedHistory.timestamp)}</span>
						</div>
						<div class="detail-header">{tr('comm.contentPreview')}</div>
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

	/* Listener console */
	.console { margin-top: 6px; border: 1px solid var(--color-border); border-radius: 4px; overflow: hidden; }
	.console-header { display: flex; align-items: center; gap: 8px; padding: 4px 8px; background: var(--color-bg-tertiary); }
	.console-title { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.5px; color: var(--color-text-secondary); }
	.console-count { font-size: 10px; color: var(--color-text-secondary); background: var(--color-bg-primary); padding: 0 6px; border-radius: 8px; }
	.console-header .btn-sm { margin-left: auto; }
	.console-empty { padding: 10px; text-align: center; font-style: italic; font-size: 11px; color: var(--color-text-secondary); }
	.console-list { max-height: 180px; overflow-y: auto; }
	.console-row { display: flex; align-items: center; gap: 8px; width: 100%; padding: 3px 8px; background: none; border: none; border-bottom: 1px solid var(--color-border); font-family: 'JetBrains Mono', monospace; font-size: 10px; text-align: left; cursor: pointer; color: var(--color-text-primary); white-space: nowrap; overflow: hidden; }
	.console-row:hover { background: var(--color-bg-tertiary); }
	.console-row:last-child { border-bottom: none; }
	.c-time { flex-shrink: 0; color: var(--color-text-secondary); }
	.c-peer { flex-shrink: 0; color: var(--color-accent); min-width: 110px; }
	.c-bytes { flex-shrink: 0; color: var(--color-text-secondary); min-width: 50px; text-align: right; }
	.c-ack { flex-shrink: 0; min-width: 26px; text-align: center; font-weight: 700; color: var(--color-text-secondary); border: 1px solid var(--color-border); border-radius: 3px; padding: 0 3px; }
	.c-ack.ack-ok { color: var(--color-success); border-color: var(--color-success); }
	.c-ack.ack-err { color: var(--color-error); border-color: var(--color-error); }
	.c-enc { flex-shrink: 0; color: var(--color-text-secondary); opacity: 0.7; }
	.c-snippet { overflow: hidden; text-overflow: ellipsis; color: var(--color-text-secondary); }
	.console-row.no-content { cursor: default; opacity: 0.55; }
	.console-error { cursor: default; }
	.c-err-text { color: var(--color-error); overflow: hidden; text-overflow: ellipsis; }

	/* Advanced toggle */
	.toggle-advanced { background: none; border: none; color: var(--color-accent); font-size: 11px; font-family: inherit; cursor: pointer; padding: 3px 0; text-align: left; }
	.toggle-advanced:hover { text-decoration: underline; }
	.advanced-section { padding: 6px 0 6px 12px; border-left: 2px solid var(--color-accent); margin: 2px 0; display: flex; flex-direction: column; gap: 5px; }
	.setting-check { margin: 2px 0; }
	.setting-check label { font-size: 11px; color: var(--color-text-primary); display: flex; align-items: center; gap: 5px; cursor: pointer; }
	.setting-check input[type="checkbox"] { accent-color: var(--color-accent); }
</style>
