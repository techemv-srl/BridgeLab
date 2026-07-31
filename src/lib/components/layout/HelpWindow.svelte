<script lang="ts">
	import { getLocale, subscribeLocale, t } from '$lib/i18n';
	import { generateManualHtml, TITLES } from './helpContent';
	import { shortcutStore, SHORTCUTS } from '$lib/stores/shortcuts.svelte';

	interface Props {
		onClose: () => void;
	}

	let { onClose }: Props = $props();

	let localeVersion = $state(0);
	if (typeof window !== 'undefined') { subscribeLocale(() => { localeVersion++; }); }

	// Strategy:
	// - In Tauri, use WebviewWindow to create a real OS window (independent,
	//   movable, minimizable, resizable via the OS).
	// - In browser, fall back to window.open() popup.
	// - If both fail, render an in-app draggable modal so the user always gets
	//   something.
	let mode = $state<'tauri' | 'popup' | 'modal' | 'pending'>('pending');
	let helpHtml = $derived.by(() => {
		void localeVersion;
		// Feed the LIVE bindings so the manual's shortcut table reflects
		// user customization instead of a drifting hand-written copy.
		const live = SHORTCUTS.map((s) => ({
			label: t('shortcut.' + s.id),
			keys: shortcutStore.get(s.id),
		}));
		return generateManualHtml(getLocale(), live);
	});

	// Modal fallback state
	let x = $state(80);
	let y = $state(60);
	let w = $state(780);
	let h = $state(620);
	let dragging = false;
	let resizing = false;
	let dragOff = { x: 0, y: 0 };

	let popupPollTimer: ReturnType<typeof setInterval> | null = null;
	// Set when the 2s fallback gives up on the Tauri window: a late
	// tauri://created must then close that window instead of adopting it,
	// or the user ends up with the fallback AND a zombie manual window.
	let tauriAbandoned = false;

	$effect(() => {
		// If neither tauri://created nor tauri://error fires (slow or wedged
		// webview), the user pressed F1 and sees nothing. Fall back to the
		// popup path after 2s instead of staying 'pending' forever.
		const pendingFallback = setTimeout(() => {
			if (mode === 'pending') {
				tauriAbandoned = true;
				tryPopup();
			}
		}, 2000);

		(async () => {
			// Try Tauri WebviewWindow first
			try {
				const mod = await import('@tauri-apps/api/webviewWindow');
				const label = 'bridgelab-help';
				// Close any existing instance
				const existing = await mod.WebviewWindow.getByLabel(label).catch(() => null);
				if (existing) {
					try { await existing.close(); } catch { /* ignore */ }
				}
				const blob = new Blob([helpHtml], { type: 'text/html;charset=utf-8' });
				const url = URL.createObjectURL(blob);
				const win = new mod.WebviewWindow(label, {
					url,
					title: TITLES[getLocale()] ?? TITLES.en,
					width: 860,
					height: 680,
					minWidth: 500,
					minHeight: 400,
					resizable: true,
					center: true,
				});
				win.once('tauri://created', () => {
					if (tauriAbandoned) {
						// Fallback already shown — discard the late window.
						void win.close().catch(() => { /* ignore */ });
						URL.revokeObjectURL(url);
						return;
					}
					mode = 'tauri';
				});
				win.once('tauri://destroyed', () => {
					URL.revokeObjectURL(url);
					if (!tauriAbandoned) onClose();
				});
				win.once('tauri://error', () => {
					URL.revokeObjectURL(url);
					if (!tauriAbandoned) tryPopup();
				});
				return;
			} catch {
				/* not Tauri - fall through to popup */
			}
			tryPopup();
		})();

		return () => {
			clearTimeout(pendingFallback);
			if (popupPollTimer) clearInterval(popupPollTimer);
		};
	});

	function tryPopup() {
		if (mode === 'popup' || mode === 'modal') return; // already resolved
		try {
			const blob = new Blob([helpHtml], { type: 'text/html;charset=utf-8' });
			const url = URL.createObjectURL(blob);
			const w2 = window.open(url, 'bridgelab-help', 'width=860,height=680,menubar=no,toolbar=no');
			if (w2) {
				mode = 'popup';
				popupPollTimer = setInterval(() => {
					if (w2.closed) {
						if (popupPollTimer) clearInterval(popupPollTimer);
						URL.revokeObjectURL(url);
						onClose();
					}
				}, 500);
				return;
			}
			URL.revokeObjectURL(url);
		} catch { /* ignore */ }
		mode = 'modal';
	}

	// Modal fallback handlers
	function startDrag(e: MouseEvent) {
		if ((e.target as HTMLElement).closest('.hw-close')) return;
		dragging = true;
		dragOff = { x: e.clientX - x, y: e.clientY - y };
		e.preventDefault();
	}
	function startResize(e: MouseEvent) { resizing = true; e.preventDefault(); e.stopPropagation(); }
	function onMove(e: MouseEvent) {
		if (dragging) {
			x = Math.max(0, e.clientX - dragOff.x);
			y = Math.max(0, e.clientY - dragOff.y);
		} else if (resizing) {
			w = Math.max(400, e.clientX - x);
			h = Math.max(300, e.clientY - y);
		}
	}
	function onUp() { dragging = false; resizing = false; }
</script>

{#if mode === 'modal'}
	<div class="hw-window" style="left:{x}px;top:{y}px;width:{w}px;height:{h}px">
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<div class="hw-title" onmousedown={startDrag}>
			<span>{TITLES[getLocale()] ?? TITLES.en}</span>
			<button class="hw-close" onclick={onClose} aria-label="Close">&times;</button>
		</div>
		<iframe class="hw-frame" srcdoc={helpHtml} title="Manual"></iframe>
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<div class="hw-resize" onmousedown={startResize}></div>
	</div>
{/if}

<svelte:window onmousemove={onMove} onmouseup={onUp} />

<style>
	.hw-window {
		position: fixed;
		z-index: 2000;
		display: flex;
		flex-direction: column;
		background: #1e1e2e;
		border: 1px solid #45475a;
		border-radius: 8px;
		box-shadow: 0 12px 40px rgba(0,0,0,0.6);
		overflow: hidden;
	}
	.hw-title {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 8px 14px;
		background: #313244;
		cursor: grab;
		user-select: none;
		font-size: 13px;
		font-weight: 700;
		color: #cdd6f4;
	}
	.hw-title:active { cursor: grabbing; }
	.hw-close {
		background: none; border: none; color: #a6adc8;
		font-size: 20px; cursor: pointer; line-height: 1; padding: 0 4px;
	}
	.hw-close:hover { color: #f38ba8; }
	.hw-frame {
		flex: 1;
		border: none;
		background: #1e1e2e;
	}
	.hw-resize {
		position: absolute;
		bottom: 0; right: 0;
		width: 16px; height: 16px;
		cursor: nwse-resize;
	}
	.hw-resize::after {
		content: '';
		position: absolute;
		bottom: 4px; right: 4px;
		width: 8px; height: 8px;
		border-right: 2px solid #a6adc8;
		border-bottom: 2px solid #a6adc8;
		opacity: 0.5;
	}
</style>
