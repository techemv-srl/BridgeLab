/**
 * Every parse leaves a message in the backend's store, a full copy of it
 * (megabytes for a large message). This keeps track of them and releases
 * the ones no open tab shows any more: the previous parse of a tab parsed
 * again, a closed tab's, an auto-parse overtaken by a newer one.
 *
 * A message is released only once it has been unused for a while, so a
 * panel still finishing a request with the previous id does not see it
 * vanish under it.
 */
import { invoke } from '@tauri-apps/api/core';

const GRACE_MS = 15_000;
const SWEEP_MS = 10_000;

const seen = new Map<string, number>();
let liveIds: () => Iterable<string | undefined> = () => [];
let timer: ReturnType<typeof setInterval> | null = null;

/** Where the ids still in use come from (the message store's tabs). */
export function setLiveIds(source: () => Iterable<string | undefined>): void {
	liveIds = source;
}

/** Note a message id the backend now holds. */
export function trackMessage(id: string | undefined | null): void {
	if (!id) return;
	seen.set(id, Date.now());
	if (!timer && typeof window !== 'undefined') timer = setInterval(sweep, SWEEP_MS);
}

/** Release every tracked message no tab uses and not touched recently. */
export function sweep(now = Date.now()): string[] {
	const live = new Set<string>();
	for (const id of liveIds()) if (id) live.add(id);
	const dead: string[] = [];
	for (const [id, at] of seen) {
		if (live.has(id)) {
			seen.set(id, now);
		} else if (now - at >= GRACE_MS) {
			dead.push(id);
			seen.delete(id);
		}
	}
	if (dead.length) invoke('release_messages', { ids: dead }).catch(() => { /* web mode */ });
	return dead;
}
