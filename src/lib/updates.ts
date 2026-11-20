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

/** A machine policy file that is there but was ignored, and why. */
export interface PolicyProblem {
	path: string;
	kind: 'invalid_json' | 'unreadable' | 'untrusted_owner' | 'untrusted_permissions';
	/** The parser's or the system's own words, or BridgeLab's in English. */
	detail: string;
	/** For a reason BridgeLab words itself: the key under
	 *  `settings.policyDetail.` and its parameters, for the user's language. */
	detail_key?: string;
	detail_args?: Record<string, string>;
}

/** The detail line of the notice: in the user's language when BridgeLab
 *  words the reason itself, else the parser's or the system's own words. */
export function policyProblemDetail(problem: PolicyProblem, tr: (key: string, params?: Record<string, string>) => string): string {
	return problem.detail_key ? tr(`settings.policyDetail.${problem.detail_key}`, problem.detail_args ?? {}) : problem.detail;
}

/** Machine policy and installer choice, as the backend reads them. */
export interface UpdatePolicy {
	disabled_by_policy: boolean;
	policy_source: string | null;
	installer_choice: boolean | null;
	policy_problem?: PolicyProblem | null;
}

/** The i18n key of the Settings notice for a policy file that was ignored. */
export function policyProblemKey(kind: PolicyProblem['kind']): string {
	if (kind === 'untrusted_owner') return 'settings.policyUntrusted';
	if (kind === 'untrusted_permissions') return 'settings.policyUntrustedPermissions';
	if (kind === 'unreadable') return 'settings.policyUnreadable';
	return 'settings.policyInvalidJson';
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

/**
 * The page to open for a release: the `html_url` of GitHub's answer when it
 * is a page of this project's releases, the releases page otherwise. The
 * answer comes over the network and the dialog does not show the address:
 * behind a TLS-inspecting proxy the system trusts, it could name any
 * download page, and the browser would open it on the app's word.
 */
export function releasePageUrl(htmlUrl: unknown): string {
	if (typeof htmlUrl !== 'string') return RELEASES_PAGE;
	let u: URL;
	try {
		u = new URL(htmlUrl);
	} catch {
		return RELEASES_PAGE;
	}
	// GitHub owner and repository names ignore case; the URL parser has
	// already resolved any `..` in the path.
	const own = new URL(RELEASES_PAGE).pathname.toLowerCase();
	const path = u.pathname.toLowerCase();
	const ours =
		u.protocol === 'https:' &&
		u.hostname === 'github.com' &&
		u.port === '' &&
		u.username === '' &&
		u.password === '' &&
		(path === own || path.startsWith(`${own}/`));
	return ours ? u.href : RELEASES_PAGE;
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
		return { version, url: releasePageUrl(rel.html_url) };
	} finally {
		clearTimeout(timer);
	}
}
