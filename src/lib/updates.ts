/**
 * Update check against the GitHub releases of the public repository.
 *
 * Used by Help → Check for updates and by the once-a-day check at startup.
 * The request is an anonymous GET to api.github.com: it carries nothing
 * about the user, the machine or the files (GitHub sees the IP address, as
 * for any web request). Release artifacts are not signed, so there is no
 * in-app install: a newer version is reported and the release page opened.
 */

const LATEST_URL = 'https://api.github.com/repos/techemv-srl/BridgeLab/releases/latest';
export const RELEASES_PAGE = 'https://github.com/techemv-srl/BridgeLab/releases';

/** Preference keys (the preferences table stores strings). */
export const PREF_STARTUP_CHECK = 'update_check_on_startup';
export const PREF_LAST_CHECK = 'update_last_check';
export const PREF_SKIPPED = 'update_skipped_version';

/** Machine policy and installer choice, as the backend reads them. */
export interface UpdatePolicy {
	disabled_by_policy: boolean;
	policy_source: string | null;
	installer_choice: boolean | null;
}

export async function getUpdatePolicy(): Promise<UpdatePolicy> {
	const { invoke } = await import('@tauri-apps/api/core');
	return invoke('get_update_policy');
}

/**
 * The preference the startup check should use: a machine policy wins
 * ("false"); otherwise the user's own choice; otherwise, the first time,
 * the Windows installer's choice (returned so the caller can store it).
 */
export function effectivePreference(
	policy: UpdatePolicy | null,
	userPref: string | null,
): { value: string | null; seedFromInstaller: boolean } {
	if (policy?.disabled_by_policy) return { value: 'false', seedFromInstaller: false };
	if (userPref === 'true' || userPref === 'false') return { value: userPref, seedFromInstaller: false };
	if (policy?.installer_choice != null) return { value: String(policy.installer_choice), seedFromInstaller: true };
	return { value: userPref, seedFromInstaller: false };
}

/**
 * True when nobody has decided yet — no machine policy, no user choice, no
 * installer answer — so the app asks at first start instead of checking.
 * This covers every package without an interactive installer (MSI, dmg,
 * deb, rpm, AppImage) and Windows installs done before the setup asked.
 */
export function needsFirstRunQuestion(policy: UpdatePolicy | null, userPref: string | null): boolean {
	return effectivePreference(policy, userPref).value === null;
}

/** At most one automatic check per day. */
export const CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;

export interface LatestRelease {
	version: string;
	url: string;
}

/** True when semver `a` is newer than `b` (numeric per part; a "v" prefix is ignored). */
export function isNewerVersion(a: string, b: string): boolean {
	const parts = (v: string) => v.replace(/^v/, '').split('.').map((n) => parseInt(n) || 0);
	const pa = parts(a);
	const pb = parts(b);
	for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
		const d = (pa[i] ?? 0) - (pb[i] ?? 0);
		if (d !== 0) return d > 0;
	}
	return false;
}

/**
 * Whether the startup check should run now. The preference defaults to ON:
 * only an explicit "false" turns it off. An unreadable or future timestamp
 * counts as "never checked".
 */
export function startupCheckDue(enabledPref: string | null, lastCheckPref: string | null, now: number): boolean {
	if (enabledPref === 'false') return false;
	const last = Number(lastCheckPref);
	if (!Number.isFinite(last) || last <= 0 || last > now) return true;
	return now - last >= CHECK_INTERVAL_MS;
}

/** Whether a found release deserves the banner: newer, and not skipped. */
export function shouldNotify(latest: string, current: string, skipped: string | null): boolean {
	if (!latest || !current) return false;
	if (skipped && latest === skipped) return false;
	return isNewerVersion(latest, current);
}

/** Latest published release, or an error. `timeoutMs` bounds the request. */
export async function fetchLatestRelease(timeoutMs = 10_000): Promise<LatestRelease> {
	const ctrl = new AbortController();
	const timer = setTimeout(() => ctrl.abort(), timeoutMs);
	try {
		const res = await fetch(LATEST_URL, {
			signal: ctrl.signal,
			headers: { Accept: 'application/vnd.github+json' },
		});
		if (!res.ok) throw new Error(`GitHub API: HTTP ${res.status}`);
		const rel = await res.json();
		const version = String(rel.tag_name ?? '').replace(/^v/, '');
		if (!version) throw new Error('No release tag found');
		return { version, url: String(rel.html_url ?? RELEASES_PAGE) };
	} finally {
		clearTimeout(timer);
	}
}
